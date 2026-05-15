use std::fs::File;
use std::os::unix::io::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};

use cosmic_text::{FontSystem, SwashCache};
use libc::{c_char, off_t};
use memmap2::{Mmap, MmapMut};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self as zwlr_layer_surface_v1, ZwlrLayerSurfaceV1,
};
use omni_ui::components::input::{InputAction, InputHandler};
use omni_ui::config::Config;
use omni_ui::render::{draw_frame, rgba_to_bgra};
use omni_ui::state::OmniApp;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

struct AppState {
    running: bool,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    seat: Option<wl_seat::WlSeat>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    layer_shell: Option<ZwlrLayerShellV1>,
    surface: Option<wl_surface::WlSurface>,
    layer_surface: Option<ZwlrLayerSurfaceV1>,
    pool: Option<wl_shm_pool::WlShmPool>,
    buffer: Option<wl_buffer::WlBuffer>,

    _shm_file: Option<File>,
    _mmap: Option<MmapMut>,

    pixmap: Option<tiny_skia::Pixmap>,
    font_system: FontSystem,
    swash_cache: SwashCache,

    app: OmniApp,
    config: Config,
    width: i32,
    height: i32,

    input: InputHandler,

    dirty: bool,

    // Cursor
    cursor_theme: Option<wayland_cursor::CursorTheme>,
    cursor_surface: Option<wl_surface::WlSurface>,
    last_serial: u32,
    scroll_axis: f32,
    scroll_discrete: i32,
    mouse_y: f32,

    // Key Repeat
    repeat_delay: i32,
    repeat_rate: i32,
    active_key: Option<u32>,
    last_key_time: Option<std::time::Instant>,
    is_repeating: bool,

    // Animation / Blinking
    start_time: std::time::Instant,
}

fn create_memfd(size: usize) -> RawFd {
    unsafe {
        let name = b"omni-shm\0".as_ptr() as *const c_char;
        let fd = libc::memfd_create(name, 0);
        if fd < 0 {
            panic!("memfd_create: {}", std::io::Error::last_os_error());
        }
        libc::ftruncate(fd, size as off_t);
        fd
    }
}

fn dup_fd(fd: RawFd) -> RawFd {
    unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) }
}

impl AppState {
    fn new(config: Config) -> Self {
        let mut app = OmniApp::new(&config);
        app.search("");
        
        // Only load the specific font family used by the system, not all 129 fonts
        let target = config.font.family.to_lowercase();
        let mut tmp_db = cosmic_text::fontdb::Database::new();
        tmp_db.load_system_fonts();
        let font_path = tmp_db.faces()
            .find(|face| face.families.iter().any(|(name, _)| name.to_lowercase() == target))
            .and_then(|face| match &face.source {
                cosmic_text::fontdb::Source::File(path) => Some(path.clone()),
                _ => None,
            });

        let mut db = cosmic_text::fontdb::Database::new();
        if let Some(path) = font_path {
            let _ = db.load_font_file(&path);
        }

        let font_system = FontSystem::new_with_locale_and_db("en-US".into(), db);
        let swash_cache = SwashCache::new();
        AppState {
            running: true,
            compositor: None,
            shm: None,
            seat: None,
            keyboard: None,
            pointer: None,
            layer_shell: None,
            surface: None,
            layer_surface: None,
            pool: None,
            buffer: None,
            _shm_file: None,
            _mmap: None,
            pixmap: None,
            font_system,
            swash_cache,
            app,
            config,
            width: 0,
            height: 0,
            input: InputHandler::new(),
            dirty: false,
            cursor_theme: None,
            cursor_surface: None,
            last_serial: 0,
            scroll_axis: 0.0,
            scroll_discrete: 0,
            mouse_y: 0.0,
            repeat_delay: 200,
            repeat_rate: 25,
            active_key: None,
            last_key_time: None,
            is_repeating: false,
            start_time: std::time::Instant::now(),
        }

    }

    fn body_height(&self) -> f32 {
        use omni_ui::render::{FOOTER_HEIGHT, GAP, HEADER_HEIGHT, PADDING};
        let h = self.height as f32;
        let list_y = PADDING + HEADER_HEIGHT + GAP;
        let footer_y = h - PADDING - FOOTER_HEIGHT;
        (footer_y - GAP - list_y).max(0.0)
    }

    fn setup_surface(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        // 1. Calculate ideal height first to avoid partial rows
        self.width = self.config.window.width;
        self.height = self.config.window.height;
        
        let row_height = self.compute_row_height();
        let body_h = self.body_height();
        let visible_rows = (body_h / row_height).floor().max(1.0);
        let snapped_body_h = visible_rows * row_height;
        let non_body_h = self.height as f32 - body_h;
        
        self.height = (non_body_h + snapped_body_h).round() as i32;

        let w = self.width;
        let h = self.height;

        // 2. Now borrow compositor and shell to create surfaces
        let compositor = self.compositor.as_ref().expect("no compositor");
        let shell = self.layer_shell.as_ref().expect("no layer shell");

        let surface = compositor.create_surface(qh, ());
        let layer_surface = shell.get_layer_surface(
            &surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            "omni".to_string(),
            qh,
            (),
        );
        layer_surface.set_size(w as u32, h as u32);
        layer_surface.set_exclusive_zone(0);
        layer_surface.set_keyboard_interactivity(
            zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
        );

        let pixmap = tiny_skia::Pixmap::new(w as u32, h as u32).expect("pixmap");

        self.surface = Some(surface);
        self.layer_surface = Some(layer_surface);
        self.pixmap = Some(pixmap);

        // Initialize Cursor
        let shm = self.shm.as_ref().expect("no shm");
        
        // wayland-cursor 0.31 uses environment variables for theme selection
        unsafe { std::env::set_var("XCURSOR_THEME", &self.config.icons.cursor_theme); }
        
        let theme = wayland_cursor::CursorTheme::load(
            conn,
            shm.clone(),
            24,
        ).expect("failed to load cursor theme");
        
        let cursor_surface = compositor.create_surface(qh, ());
        self.cursor_theme = Some(theme);
        self.cursor_surface = Some(cursor_surface);

        self.surface.as_ref().unwrap().commit();
    }

    fn update_cursor(&mut self, cursor_name: &str, serial: u32) {
        let theme = match self.cursor_theme.as_mut() {
            Some(t) => t,
            None => return,
        };
        let cursor_surface = match self.cursor_surface.as_ref() {
            Some(s) => s,
            None => return,
        };
        let pointer = match self.pointer.as_ref() {
            Some(p) => p,
            None => return,
        };

        if let Some(cursor) = theme.get_cursor(cursor_name) {
            let image = &cursor[0];
            let (w, h) = image.dimensions();
            let (hx, hy) = image.hotspot();
            
            // CursorImageBuffer implements Deref<Target = WlBuffer>
            cursor_surface.attach(Some(&*image), 0, 0);
            cursor_surface.damage_buffer(0, 0, w as i32, h as i32);
            cursor_surface.commit();
            
            pointer.set_cursor(serial, Some(cursor_surface), hx as i32, hy as i32);
        }
    }

    fn create_shm_buffers(&mut self, qh: &QueueHandle<Self>) {
        let shm = self.shm.as_ref().expect("no shm");
        let w = self.width;
        let h = self.height;
        let stride = w * 4;
        let buf_size = (h * stride) as usize;

        let raw = create_memfd(buf_size);
        let dup = unsafe { libc::fcntl(raw, libc::F_DUPFD_CLOEXEC, 0) };

        let shm_file = unsafe { File::from_raw_fd(dup) };
        let mmap = unsafe { MmapMut::map_mut(&shm_file) }.expect("mmap");

        let pool_fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let pool = shm.create_pool(pool_fd.as_fd(), buf_size as i32, qh, ());
        let buffer = pool.create_buffer(
            0, w, h, stride,
            wl_shm::Format::Argb8888,
            qh, (),
        );

        self._shm_file = Some(shm_file);
        self._mmap = Some(mmap);
        self.pool = Some(pool);
        self.buffer = Some(buffer);
    }

    fn draw(&mut self) {
        let mmap = match self._mmap.as_mut() {
            Some(m) => m,
            None => return,
        };

        if let Some(ref mut pixmap) = self.pixmap {
            let cursor_visible = self.start_time.elapsed().as_millis() % 1000 < 500;
            draw_frame(
                pixmap,
                &mut self.font_system,
                &mut self.swash_cache,
                &mut self.app,
                &self.config,
                cursor_visible,
            );
            // RGBA → BGRA for Wayland SHM
            rgba_to_bgra(pixmap.data(), unsafe {
                std::slice::from_raw_parts_mut(mmap.as_mut_ptr(), pixmap.data().len())
            });
        }
    }

    fn commit(&self) {
        if let (Some(surface), Some(buffer)) = (&self.surface, &self.buffer) {
            surface.attach(Some(buffer), 0, 0);
            surface.damage_buffer(0, 0, self.width, self.height);
            surface.commit();
        }
    }

    fn redraw_and_commit(&mut self) {
        self.draw();
        self.commit();
    }

    fn apply_action(&mut self, action: InputAction) {
        match action {
            InputAction::Confirm => {
                if !self.app.results.is_empty() {
                    let exec = self.app.results[self.app.selected_index]
                        .entry
                        .exec
                        .clone();
                    OmniApp::launch_selected(&exec);
                }
                self.running = false;
            }
            InputAction::Cancel => {
                self.running = false;
            }
            InputAction::Backspace => {
                self.app.backspace();
                self.dirty = true;
            }
            InputAction::Delete => {
                self.app.delete();
                self.dirty = true;
            }
            InputAction::MoveCursorLeft => {
                self.app.move_cursor_left();
                self.dirty = true;
            }
            InputAction::MoveCursorRight => {
                self.app.move_cursor_right();
                self.dirty = true;
            }
            InputAction::MoveCursorStart => {
                self.app.move_cursor_start();
                self.dirty = true;
            }
            InputAction::MoveCursorEnd => {
                self.app.move_cursor_end();
                self.dirty = true;
            }
            InputAction::DeleteWordBackward => {
                self.app.delete_word_backward();
                self.dirty = true;
            }
            InputAction::DeleteWordForward => {
                self.app.delete_word_forward();
                self.dirty = true;
            }
            InputAction::MoveWordLeft => {
                self.app.move_word_left();
                self.dirty = true;
            }
            InputAction::MoveWordRight => {
                self.app.move_word_right();
                self.dirty = true;
            }
            InputAction::SelectPrev => {
                self.app.select_prev();
                let row_height = self.compute_row_height();
                let max_items = self.app.results.len();
                let body_h = self.app.mouse.body_bounds().map_or(0.0, |(_, h)| h);
                self.app.list_state.ensure_visible(self.app.selected_index, row_height, body_h, max_items);
                self.dirty = true;
            }
            InputAction::SelectNext => {
                self.app.select_next();
                let row_height = self.compute_row_height();
                let max_items = self.app.results.len();
                let body_h = self.app.mouse.body_bounds().map_or(0.0, |(_, h)| h);
                self.app.list_state.ensure_visible(self.app.selected_index, row_height, body_h, max_items);
                self.dirty = true;
            }
            InputAction::AppendChar(ch) => {
                self.app.insert_char(ch);
                self.dirty = true;
            }
            InputAction::None => {}
        }
    }
}

// ── Dispatch ────────────────────────────────────────────────

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, .. } = event {
            match interface.as_str() {
                "wl_compositor" => {
                    let obj = registry.bind::<wl_compositor::WlCompositor, _, _>(name, 4, qh, ());
                    state.compositor = Some(obj);
                }
                "wl_shm" => {
                    let obj = registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ());
                    state.shm = Some(obj);
                }
                "zwlr_layer_shell_v1" => {
                    let obj =
                        registry.bind::<ZwlrLayerShellV1, _, _>(name, 1, qh, ());
                    state.layer_shell = Some(obj);
                }
                "wl_seat" => {
                    let obj = registry.bind::<wl_seat::WlSeat, _, _>(name, 7, qh, ());
                    state.seat = Some(obj);
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(AppState: ignore wl_compositor::WlCompositor);
delegate_noop!(AppState: ignore wl_surface::WlSurface);
delegate_noop!(AppState: ignore wl_shm::WlShm);
delegate_noop!(AppState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(AppState: ignore wl_buffer::WlBuffer);
delegate_noop!(AppState: ignore ZwlrLayerShellV1);

impl Dispatch<wl_pointer::WlPointer, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let row_height = state.compute_row_height();
        match event {
            wl_pointer::Event::Enter { serial, surface_y, .. } => {
                state.last_serial = serial;
                state.mouse_y = surface_y as f32;
                let my = surface_y as f32;
                let region = state.app.mouse.region_at(my, my);
                state.app.list_state.hover_y = (region == Some(omni_ui::events::RegionId::Body))
                    .then_some(my);
                let cursor_name = state.get_cursor_name(region);
                state.update_cursor(&cursor_name, serial);
                state.dirty = true;
            }
            wl_pointer::Event::Leave { .. } => {
                state.app.list_state.hover_y = None;
                state.dirty = true;
            }
            wl_pointer::Event::Motion { surface_y, .. } => {
                state.mouse_y = surface_y as f32;
                let my = surface_y as f32;
                let region = state.app.mouse.region_at(my, my);
                state.app.list_state.hover_y = (region == Some(omni_ui::events::RegionId::Body))
                    .then_some(my);
                let cursor_name = state.get_cursor_name(region);
                let serial = state.last_serial;
                state.update_cursor(&cursor_name, serial);
                state.dirty = true;
            }
            wl_pointer::Event::Button { state: btn_state, serial, .. } => {
                state.last_serial = serial;
                if btn_state == WEnum::Value(wl_pointer::ButtonState::Pressed) {
                    if state.app.mouse.region_at(state.mouse_y, state.mouse_y)
                        == Some(omni_ui::events::RegionId::Body)
                    {
                        if let Some((body_y, _)) = state.app.mouse.body_bounds() {
                            let rel_y = state.mouse_y - body_y + state.app.list_state.scroll_offset;
                            let idx = (rel_y / row_height).floor() as usize;
                            if idx < state.app.results.len() {
                                state.app.selected_index = idx;
                                let exec = state.app.results[idx].entry.exec.clone();
                                OmniApp::launch_selected(&exec);
                                state.running = false;
                            }
                        }
                    }
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                if axis == WEnum::Value(wl_pointer::Axis::VerticalScroll) {
                    state.scroll_axis += value as f32;
                }
            }
            wl_pointer::Event::AxisDiscrete { axis, discrete } => {
                if axis == WEnum::Value(wl_pointer::Axis::VerticalScroll) {
                    state.scroll_discrete += discrete;
                }
            }
            wl_pointer::Event::Frame => {
                if state.app.mouse.region_at(state.mouse_y, state.mouse_y)
                    != Some(omni_ui::events::RegionId::Body)
                {
                    state.scroll_axis = 0.0;
                    state.scroll_discrete = 0;
                    return;
                }
                let delta = if state.scroll_discrete != 0 {
                    state.scroll_discrete as f32 * row_height
                } else {
                    state.scroll_axis
                };
                if delta != 0.0 {
                    let max_items = state.app.results.len();
                    let body_h = state.app.mouse.body_bounds().map_or(0.0, |(_, h)| h);
                    state.app.list_state.scroll_by(delta, max_items, row_height, body_h);
                    state.dirty = true;
                }
                state.scroll_axis = 0.0;
                state.scroll_discrete = 0;
            }
            _ => {}
        }
    }
}

impl AppState {
    fn compute_row_height(&mut self) -> f32 {
        let font_size = self.config.font.size as f32;
        let family = &self.config.font.family;
        let (n_ascent, n_descent) = omni_ui::render::get_text_metrics(&mut self.font_system, "Ag", font_size, family);
        let (d_ascent, d_descent) = omni_ui::render::get_text_metrics(&mut self.font_system, "Ag", font_size - 2.0, family);
        (n_ascent + n_descent) + (d_ascent + d_descent) + 2.0 + 12.0
    }

    fn get_cursor_name(&self, region: Option<omni_ui::events::RegionId>) -> String {
        match region {
            Some(omni_ui::events::RegionId::Header) => "xterm".into(),
            Some(omni_ui::events::RegionId::Body) => "pointer".into(),
            _ => "left_ptr".into(),
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities } = event {
            if let WEnum::Value(caps) = capabilities {
                if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                    let kb = seat.get_keyboard(qh, ());
                    state.keyboard = Some(kb);
                }
                if caps.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
                    let ptr = seat.get_pointer(qh, ());
                    state.pointer = Some(ptr);
                }
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format, fd, size } => {
                tracing::info!("Keymap event: format={:?}, size={}", format, size);
                if format == WEnum::Value(wl_keyboard::KeymapFormat::XkbV1) {
                    let file = unsafe { File::from_raw_fd(dup_fd(fd.as_raw_fd())) };
                    match unsafe { Mmap::map(&file) } {
                        Ok(mmap) => {
                            let mut len = size as usize;
                            while len > 0 && mmap[len - 1] == 0 {
                                len -= 1;
                            }
                            let bytes = &mmap[..len];
                            match std::str::from_utf8(bytes) {
                                Ok(s) => {
                                    tracing::info!("Keymap string read successfully, length={}", s.len());
                                    state.input.set_keymap(s);
                                }
                                Err(e) => tracing::error!("Keymap is not valid UTF-8: {}", e),
                            }
                        }
                        Err(e) => tracing::error!("Failed to mmap keymap: {}", e),
                    }
                } else {
                    tracing::warn!("Unsupported keymap format: {:?}", format);
                }
            }
            wl_keyboard::Event::Key {
                key,
                state: key_state,
                ..
            } => {
                let pressed = key_state == WEnum::Value(wl_keyboard::KeyState::Pressed);
                if pressed {
                    state.active_key = Some(key);
                    state.last_key_time = Some(std::time::Instant::now());
                    state.is_repeating = false;
                    let action = state.input.handle_key(key, true);
                    state.apply_action(action);
                } else {
                    if state.active_key == Some(key) {
                        state.active_key = None;
                        state.last_key_time = None;
                        state.is_repeating = false;
                    }
                    state.input.handle_key(key, false);
                }
                
                if state.dirty {
                    state.redraw_and_commit();
                    state.dirty = false;
                }
            }
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => {
                tracing::debug!("Modifiers: dep={} latch={} lock={} group={}", mods_depressed, mods_latched, mods_locked, group);
                state.input.update_modifiers(mods_depressed, mods_latched, mods_locked, group);
            }
            wl_keyboard::Event::RepeatInfo { rate, delay } => {
                state.repeat_rate = rate;
                state.repeat_delay = delay;
            }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                if state.buffer.is_none() {
                    if width > 0 && height > 0 {
                        state.width = width as i32;
                        state.height = height as i32;
                    }
                    state.create_shm_buffers(qh);
                    let w = state.width;
                    let h = state.height;
                    state.layer_surface.as_ref().unwrap().ack_configure(serial);
                    state.draw();
                    if let (Some(surf), Some(buf)) = (&state.surface, &state.buffer) {
                        surf.attach(Some(buf), 0, 0);
                        surf.damage_buffer(0, 0, w, h);
                        surf.commit();
                    }
                }
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.running = false;
            }
            _ => {}
        }
    }
}

// ── Main ────────────────────────────────────────────────────

fn main() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
                .parse("info,usvg=error,omni=debug,omni_ui=debug")
                .unwrap(),
        )
        .init();
    
    let config = Config::load();

    let conn = Connection::connect_to_env().expect("wayland connection");
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut state = AppState::new(config);

    conn.display().get_registry(&qh, ());
    event_queue.roundtrip(&mut state).expect("roundtrip");

    state.setup_surface(&conn, &qh);
    conn.flush().unwrap();

    event_queue.blocking_dispatch(&mut state).unwrap();
    conn.flush().unwrap();

    while state.running {
        // 1. Flush outgoing requests
        let _ = conn.flush();

        // 2. Prepare for reading events
        if let Some(guard) = event_queue.prepare_read() {
            let fd = conn.as_fd().as_raw_fd();
            let mut poll_fd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };

            // Poll with a 16ms timeout (matches ~60fps)
            let ret = unsafe { libc::poll(&mut poll_fd, 1, 16) };

            if ret > 0 && (poll_fd.revents & libc::POLLIN) != 0 {
                // We have data, read it
                let _ = guard.read();
            } else {
                // Timeout or error, drop the guard
            }
        }

        // 3. Dispatch all pending events
        event_queue.dispatch_pending(&mut state).unwrap();
        
        // 4. Handle Key Repeat
        if let Some(key) = state.active_key {
            if let Some(last_time) = state.last_key_time {
                let elapsed = last_time.elapsed().as_millis() as i32;
                let delay = if state.is_repeating {
                    if state.repeat_rate > 0 { 1000 / state.repeat_rate } else { 100 }
                } else {
                    state.repeat_delay
                };

                if elapsed >= delay {
                    let action = state.input.handle_key(key, true);
                    state.apply_action(action);
                    state.last_key_time = Some(std::time::Instant::now());
                    state.is_repeating = true;
                    state.dirty = true;
                }
            }
        }

        // 5. Redraw and Commit
        if state.dirty {
            state.redraw_and_commit();
            state.dirty = false;
        } else {
            // Always redraw for blinking cursor
            state.redraw_and_commit();
        }

        conn.flush().unwrap();
    }
}
