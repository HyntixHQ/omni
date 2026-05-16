use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::io::{AsFd, OwnedFd, RawFd};

use cosmic_text::{FontSystem, SwashCache};
use libc::off_t;
use memmap2::{Mmap, MmapMut};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle, WEnum};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{
    self, ZwlrLayerShellV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, ZwlrLayerSurfaceV1,
};

use wisp::input::{InputAction, WispInput};
use wisp::surface::WispSurface;
use wisp::{CursorBlink, CursorManager};

fn create_memfd(size: usize) -> RawFd {
    unsafe {
        let name = c"wisp-shm".as_ptr();
        let fd = libc::memfd_create(name, 0);
        if fd < 0 {
            panic!("memfd_create: {}", std::io::Error::last_os_error());
        }
        libc::ftruncate(fd, size as off_t);
        fd
    }
}

fn dup_fd(fd: std::os::unix::io::RawFd) -> std::os::unix::io::RawFd {
    unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) }
}

/// Dispatch state — owns Wayland objects and queued actions.
pub struct Inner {
    pub running: bool,
    pub dirty: bool,
    pub surface: Option<WispSurface>,
    pub input: WispInput,
    pub actions: Vec<InputAction>,
    pub shm: Option<wl_shm::WlShm>,
    pub repeat_delay: i32,
    pub repeat_rate: i32,
    pub active_key: Option<u32>,
    pub last_key_time: Option<std::time::Instant>,
    pub is_repeating: bool,
    pub cursor_blink: CursorBlink,
    pub cursor: Option<CursorManager>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub layer_shell: Option<ZwlrLayerShellV1>,
}

delegate_noop!(Inner: ignore wl_compositor::WlCompositor);
delegate_noop!(Inner: ignore wl_surface::WlSurface);
delegate_noop!(Inner: ignore wl_shm::WlShm);
delegate_noop!(Inner: ignore wl_shm_pool::WlShmPool);
delegate_noop!(Inner: ignore wl_buffer::WlBuffer);
delegate_noop!(Inner: ignore ZwlrLayerShellV1);

impl Dispatch<wl_registry::WlRegistry, ()> for Inner {
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
                    state.shm = Some(obj.clone());
                    if let Some(s) = state.surface.as_mut() {
                        s.shm = Some(obj);
                    }
                }
                "zwlr_layer_shell_v1" => {
                    let obj = registry.bind::<ZwlrLayerShellV1, _, _>(name, 1, qh, ());
                    state.layer_shell = Some(obj);
                }
                "wl_seat" => {
                    let _ = registry.bind::<wl_seat::WlSeat, _, _>(name, 7, qh, ());
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for Inner {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format: WEnum::Value(wl_keyboard::KeymapFormat::XkbV1), fd, size } => {
                let file = unsafe { File::from_raw_fd(dup_fd(fd.as_raw_fd())) };
                match unsafe { Mmap::map(&file) } {
                    Ok(mmap) => {
                        let mut len = size as usize;
                        while len > 0 && mmap[len - 1] == 0 { len -= 1; }
                        if let Ok(s) = std::str::from_utf8(&mmap[..len]) {
                            state.input.set_keymap(s);
                        }
                    }
                    Err(e) => tracing::error!("Failed to mmap keymap: {}", e),
                }
            }
            wl_keyboard::Event::Key { key, state: key_state, .. } => {
                let pressed = key_state == WEnum::Value(wl_keyboard::KeyState::Pressed);
                if pressed {
                    state.active_key = Some(key);
                    state.last_key_time = Some(std::time::Instant::now());
                    state.is_repeating = false;
                    let action = state.input.keydown(key);
                    state.actions.push(action);
                } else {
                    if state.active_key == Some(key) {
                        state.active_key = None;
                        state.last_key_time = None;
                        state.is_repeating = false;
                    }
                    state.input.keyup(key);
                }
                state.dirty = true;
            }
            wl_keyboard::Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } => {
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

impl Dispatch<wl_seat::WlSeat, ()> for Inner {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(caps) } = event {
            if caps.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
            if caps.contains(wl_seat::Capability::Pointer) {
                let ptr = seat.get_pointer(qh, ());
                state.pointer = Some(ptr);
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for Inner {
    fn event(
        state: &mut Self,
        _proxy: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Enter { serial, surface_y, .. } => {
                state.input.mouse_enter(surface_y as f32);
                if let Some(cursor) = state.cursor.as_mut() {
                    cursor.set_serial(serial);
                    if let Some(pointer) = state.pointer.as_ref() {
                        cursor.set_cursor(pointer, wisp::CursorStyle::Arrow);
                    }
                }
                state.dirty = true;
            }
            wl_pointer::Event::Leave { .. } => {
                state.input.mouse_exit();
                state.dirty = true;
            }
            wl_pointer::Event::Motion { surface_y, .. } => {
                state.input.mouse_move(surface_y as f32);
                state.dirty = true;
            }
            wl_pointer::Event::Button { button, state: btn_state, serial, .. } => {
                if let Some(cursor) = state.cursor.as_mut() {
                    cursor.set_serial(serial);
                }
                let pressed = btn_state == WEnum::Value(wl_pointer::ButtonState::Pressed);
                // Map wayland button codes to MouseButton
                let mb = match button {
                    0x110 => wisp::MouseButton::Left,
                    0x111 => wisp::MouseButton::Right,
                    0x112 => wisp::MouseButton::Middle,
                    _ => return,
                };
                state.input.mouse_button(mb, pressed);
                state.dirty = true;
            }
            wl_pointer::Event::Axis { axis: WEnum::Value(wl_pointer::Axis::VerticalScroll), value, .. } => {
                state.input.scroll_axis(value as f32);
            }
            wl_pointer::Event::AxisDiscrete { axis: WEnum::Value(wl_pointer::Axis::VerticalScroll), discrete } => {
                state.input.scroll_discrete(discrete);
            }
            wl_pointer::Event::Frame => { state.dirty = true; }
            _ => {}
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for Inner {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                if let Some(surf) = state.surface.as_mut() {
                    if surf.buffer.is_none() {
                        if width > 0 && height > 0 {
                            surf.width = width as i32;
                            surf.height = height as i32;
                        }
                        surf.ack_configure(serial);
                        surf.set_shm(state.shm.clone().expect("no shm"));

                        // Create SHM pool + buffer + mmap
                        let shm = state.shm.as_ref().expect("no shm").clone();
                        let w = surf.width;
                        let h = surf.height;
                        let stride = w * 4;
                        let buf_size = (h * stride) as usize;
                        surf.pixmap = tiny_skia::Pixmap::new(
                            width.max(1), height.max(1),
                        ).expect("pixmap");
                        let memfd = create_memfd(buf_size);
                        let dup = unsafe { libc::fcntl(memfd, libc::F_DUPFD_CLOEXEC, 0) };
                        let shm_file = unsafe { File::from_raw_fd(dup) };
                        let mmap = unsafe { MmapMut::map_mut(&shm_file) }.expect("mmap");
                        let pool_fd = unsafe { OwnedFd::from_raw_fd(memfd) };
                        let pool = shm.create_pool(
                            pool_fd.as_fd(), buf_size as i32, qh, (),
                        );
                        let buffer = pool.create_buffer(
                            0, w, h, stride, wl_shm::Format::Argb8888, qh, (),
                        );
                        surf.set_buffers(pool, buffer, mmap);
                    }
                }
                state.dirty = true;
            }
            zwlr_layer_surface_v1::Event::Closed => {
                state.running = false;
            }
            _ => {}
        }
    }
}

/// Pure business logic daemon — manages Wayland, surfaces, input.
/// No UI code. Caller handles rendering and actions.
pub struct Daemon {
    pub conn: Connection,
    pub event_queue: EventQueue<Inner>,
    pub inner: Box<Inner>,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
}

impl Daemon {
    pub fn connect() -> Self {
        let mut font_db = cosmic_text::fontdb::Database::new();
        font_db.load_system_fonts();
        let font_system = FontSystem::new_with_locale_and_db(
            "en-US".into(),
            font_db,
        );
        let swash_cache = SwashCache::new();
        let conn = Connection::connect_to_env().expect("wayland connection");
        let event_queue = conn.new_event_queue();
        let _qh = event_queue.handle();

        let inner = Inner {
            running: true,
            dirty: false,
            surface: None,
            input: WispInput::new(),
            actions: Vec::new(),
            shm: None,
            repeat_delay: 200,
            repeat_rate: 25,
            active_key: None,
            last_key_time: None,
            is_repeating: false,
            cursor_blink: CursorBlink::new(),
            cursor: None,
            pointer: None,
            compositor: None,
            layer_shell: None,
        };

        Daemon { conn, event_queue, inner: Box::new(inner), font_system, swash_cache }
    }

    pub fn init_globals(&mut self) {
        let qh = self.event_queue.handle();
        self.conn.display().get_registry(&qh, ());
        self.event_queue.roundtrip(&mut *self.inner).expect("registry roundtrip");
    }

    pub fn create_surface(&mut self, width: i32, height: i32) {
        let compositor = self.inner.compositor.clone().expect("no compositor");
        let shell = self.inner.layer_shell.clone().expect("no layer shell");
        let qh = self.event_queue.handle();

        let wl_surface = compositor.create_surface(&qh, ());
        let layer_surface = shell.get_layer_surface(
            &wl_surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            "omni".to_string(),
            &qh,
            (),
        );

        let mut surf = WispSurface::new(compositor.clone(), shell, wl_surface, layer_surface);
        surf.set_size(width as u32, height as u32);
        surf.set_exclusive_zone(0);
        surf.set_keyboard_interactivity(
            zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
        );
        surf.pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).expect("pixmap");
        surf.width = width;
        surf.height = height;
        surf.wl_surface.commit();

        self.inner.surface = Some(surf);

        // Initialize cursor manager
        let shm = self.inner.shm.clone().expect("no shm");
        let cursor_surface = compositor.create_surface(&qh, ());
        let cursor = CursorManager::new(
            &self.conn,
            &shm,
            cursor_surface,
            "Adwaita",
            24,
        );
        self.inner.cursor = Some(cursor);
    }

    pub fn mouse_y(&self) -> f32 {
        self.inner.input.mouse_y()
    }

    pub fn mouse_inside(&self) -> bool {
        self.inner.input.mouse_inside()
    }

    pub fn cursor(&mut self) -> &mut CursorManager {
        self.inner.cursor.as_mut().expect("cursor not initialized")
    }

    pub fn pointer(&self) -> Option<&wl_pointer::WlPointer> {
        self.inner.pointer.as_ref()
    }

    /// Returns accumulated scroll delta (converted to pixels) and resets counters.
    pub fn drain_scroll(&mut self, row_height: f32) -> f32 {
        self.inner.input.drain_scroll(row_height)
    }

    pub fn surface(&mut self) -> &mut WispSurface {
        self.inner.surface.as_mut().expect("surface not initialized")
    }

    /// Cursor blink — delegates to wisp::CursorBlink (GPUI-compatible).
    pub fn cursor_visible(&self) -> bool {
        self.inner.cursor_blink.visible()
    }

    /// Call on any user input — keeps cursor solid for 300ms (GPUI behavior).
    pub fn mark_input(&mut self) {
        self.inner.cursor_blink.mark_activity();
    }

    pub fn poll(&mut self) -> bool {
        let _ = self.conn.flush();
        if let Some(guard) = self.event_queue.prepare_read() {
            let fd = self.conn.as_fd().as_raw_fd();
            let mut poll_fd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let _ret = unsafe { libc::poll(&mut poll_fd, 1, 16) };
            if poll_fd.revents & libc::POLLIN != 0 {
                let _ = guard.read();
            }
        }
        self.event_queue.dispatch_pending(&mut *self.inner).unwrap();
        self.inner.running
    }

    pub fn drain_actions(&mut self) -> Vec<InputAction> {
        std::mem::take(&mut self.inner.actions)
    }

    pub fn process_repeat(&mut self) -> Option<InputAction> {
        let key = self.inner.active_key?;
        let last_time = self.inner.last_key_time?;
        let elapsed = last_time.elapsed().as_millis() as i32;
        let delay = if self.inner.is_repeating {
            if self.inner.repeat_rate > 0 { 1000 / self.inner.repeat_rate } else { 100 }
        } else {
            self.inner.repeat_delay
        };
        if elapsed >= delay {
            let action = self.inner.input.handle_repeat(key);
            self.inner.last_key_time = Some(std::time::Instant::now());
            self.inner.is_repeating = true;
            self.inner.dirty = true;
            Some(action)
        } else {
            None
        }
    }

    pub fn set_dirty(&mut self) {
        self.inner.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.inner.dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.inner.dirty
    }

    pub fn commit(&self) {
        if let Some(surf) = self.inner.surface.as_ref() {
            surf.commit();
        }
    }

    pub fn draw_frame<F>(&mut self, f: F)
    where
        F: FnOnce(&mut WispSurface, &mut FontSystem, &mut SwashCache),
    {
        let surf = self.inner.surface.as_mut().expect("surface not initialized");
        f(surf, &mut self.font_system, &mut self.swash_cache);
    }

    pub fn running(&self) -> bool {
        self.inner.running
    }

    pub fn quit(&mut self) {
        self.inner.running = false;
    }

    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn swash_cache_mut(&mut self) -> &mut SwashCache {
        &mut self.swash_cache
    }

    pub fn queue_handle(&self) -> QueueHandle<Inner> {
        self.event_queue.handle()
    }
}
