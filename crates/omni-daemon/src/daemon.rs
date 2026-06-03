use std::fs::File;
use std::io::Read;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::{UnixListener, UnixStream};

use cosmic_text::{FontSystem, SwashCache};
use libc::off_t;
use memmap2::{Mmap, MmapMut};
use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_backend::client::WaylandError;
use wayland_client::{delegate_noop, event_created_child, Connection, Dispatch, EventQueue, QueueHandle, WEnum};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{
    self, ZwlrLayerShellV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, ZwlrLayerSurfaceV1,
};

use wisp::input::{InputAction, WispInput};
use wisp::surface::WispSurface;
use wisp::{CursorBlink, CursorManager};

use omni_ipc::IpcCommand;

use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::ExtDataControlOfferV1,
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
};

use crate::shortcuts::{HyprlandGlobalShortcutsManagerV1, ShortcutAction, ShortcutManager};

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

fn create_surface_buffers(
    surf: &mut WispSurface,
    shm: &wl_shm::WlShm,
    w: i32,
    h: i32,
    qh: &QueueHandle<Inner>,
) {
    let stride = w * 4;
    let buf_size = (h * stride) as usize;
    surf.pixmap = tiny_skia::Pixmap::new(w as u32, h as u32).expect("pixmap");

    // Keep old buffer/pool alive (don't destroy) — the compositor may not have sent
    // the release event yet after hide, and destroying an unreleased buffer is a
    // protocol violation. Old resources are reclaimed when the surface is destroyed.
    drop(surf.mmap.take());

    let memfd = create_memfd(buf_size);
    let dup = dup_fd(memfd);
    let shm_file = unsafe { File::from_raw_fd(dup) };
    let mmap = unsafe { MmapMut::map_mut(&shm_file) }.expect("mmap");
    let pool_fd = unsafe { OwnedFd::from_raw_fd(memfd) };
    let pool = shm.create_pool(pool_fd.as_fd(), buf_size as i32, qh, ());
    let buffer = pool.create_buffer(0, w, h, stride, wl_shm::Format::Argb8888, qh, ());
    surf.set_buffers(pool, buffer, mmap);
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
    pub ipc_listener: Option<UnixListener>,
    pub ipc_actions: Vec<IpcCommand>,
    pub ipc_clients: Vec<UnixStream>,
    pub ipc_buf: Vec<u8>,
    pub surface_visible: bool,
    pub pending_shortcut: Option<ShortcutAction>,
    pub shortcut_manager: ShortcutManager,
    // Clipboard monitoring
    pub data_manager: Option<ExtDataControlManagerV1>,
    pub data_device: Option<ExtDataControlDeviceV1>,
    pub pending_offer: Option<ExtDataControlOfferV1>,
    pub pending_read_offer: Option<ExtDataControlOfferV1>,
    pub skipping_next_read: bool,
    pub clipboard: Option<omni_clipboard::ClipboardState>,
    pub pending_paste_text: Option<String>,
    pub pending_source: Option<ExtDataControlSourceV1>,
}

delegate_noop!(Inner: ignore wl_compositor::WlCompositor);
delegate_noop!(Inner: ignore wl_surface::WlSurface);
delegate_noop!(Inner: ignore wl_shm::WlShm);
delegate_noop!(Inner: ignore wl_shm_pool::WlShmPool);
delegate_noop!(Inner: ignore wl_buffer::WlBuffer);
delegate_noop!(Inner: ignore ZwlrLayerShellV1);
delegate_noop!(Inner: ignore HyprlandGlobalShortcutsManagerV1);
delegate_noop!(Inner: ignore ExtDataControlManagerV1);

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
                "hyprland_global_shortcuts_manager_v1" => {
                    let mgr = registry.bind::<HyprlandGlobalShortcutsManagerV1, _, _>(name, 1, qh, ());
                    state.shortcut_manager.init(&mgr, qh);
                }
                "ext_data_control_manager_v1" => {
                    let mgr = registry.bind::<ExtDataControlManagerV1, _, _>(name, 1, qh, ());
                    state.data_manager = Some(mgr.clone());
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
            // Create data device once we have a seat
            if let Some(ref mgr) = state.data_manager {
                let device = mgr.get_data_device(seat, qh, ());
                state.data_device = Some(device);
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
            wl_pointer::Event::Enter { serial, surface_x, surface_y, .. } => {
                state.input.mouse_enter(surface_x as f32, surface_y as f32);
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
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                state.input.mouse_move(surface_x as f32, surface_y as f32);
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
                let action = state.input.mouse_button(mb, pressed);
                if !matches!(action, InputAction::None) {
                    state.actions.push(action);
                }
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
                    if width > 0 && height > 0 {
                        surf.width = width as i32;
                        surf.height = height as i32;
                    }
                    surf.ack_configure(serial);

                    let w = surf.width.max(1);
                    let h = surf.height.max(1);

                    if w <= 0 || h <= 0 { return; }

                    let same_size = surf.pixmap.width() == w as u32 && surf.pixmap.height() == h as u32;
                    if same_size && surf.buffer.is_some() {
                        state.dirty = true;
                        return;
                    }

                    if let Some(shm) = state.shm.clone() {
                        surf.set_shm(shm.clone());
                        create_surface_buffers(surf, &shm, w, h, qh);
                    }
                }
                state.dirty = true;
            }
            zwlr_layer_surface_v1::Event::Closed => {
                tracing::warn!("layer surface closed by compositor");
                state.running = false;
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtDataControlDeviceV1, ()> for Inner {
    fn event(
        state: &mut Self,
        _proxy: &ExtDataControlDeviceV1,
        event: <ExtDataControlDeviceV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_device_v1::Event::DataOffer { id } => {
                state.pending_offer = Some(id);
            }
            ext_data_control_device_v1::Event::Selection { id } => {
                if state.skipping_next_read {
                    state.skipping_next_read = false;
                    tracing::info!("data_control: skipping Selection (own selection)");
                } else {
                    state.pending_read_offer = id;
                }
                state.pending_offer = None;
            }
            ext_data_control_device_v1::Event::Finished => {
                state.data_device = None;
            }
            _ => {}
        }
    }

    event_created_child!(Inner, ExtDataControlDeviceV1, [
        0 => (ExtDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ExtDataControlOfferV1, ()> for Inner {
    fn event(
        _state: &mut Self,
        _proxy: &ExtDataControlOfferV1,
        _event: <ExtDataControlOfferV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlSourceV1, ()> for Inner {
    fn event(
        state: &mut Self,
        proxy: &ExtDataControlSourceV1,
        event: <ExtDataControlSourceV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let is_current_source = state.pending_source.as_ref().is_some_and(|s| s == proxy);
        match event {
            ext_data_control_source_v1::Event::Send { mime_type, fd } => {
                if is_current_source {
                    if let Some(ref pending) = state.pending_paste_text {
                        if mime_type == "text/plain" || mime_type == "text/plain;charset=utf-8" {
                            use std::io::Write;
                            let mut file = std::fs::File::from(fd);
                            let _ = file.write_all(pending.as_bytes());
                            let _ = file.write_all(b"\n");
                        }
                    }
                }
            }
            ext_data_control_source_v1::Event::Cancelled => {
                if is_current_source {
                    state.pending_paste_text = None;
                    state.pending_source = None;
                }
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
            ipc_listener: None,
            ipc_actions: Vec::new(),
            ipc_clients: Vec::new(),
            ipc_buf: Vec::new(),
            surface_visible: false,
            pending_shortcut: None,
            shortcut_manager: ShortcutManager::new(),
            data_manager: None,
            data_device: None,
            pending_offer: None,
            pending_read_offer: None,
            skipping_next_read: false,
            clipboard: None,
            pending_paste_text: None,
            pending_source: None,
        };

        Daemon { conn, event_queue, inner: Box::new(inner), font_system, swash_cache }
    }

    pub fn init_globals(&mut self) {
        let qh = self.event_queue.handle();
        self.conn.display().get_registry(&qh, ());
        self.event_queue.roundtrip(&mut *self.inner).expect("registry roundtrip");
    }

    pub fn init_ipc(&mut self) {
        match omni_ipc::bind_abstract("omni-ipc") {
            Ok(listener) => {
                self.inner.ipc_listener = Some(listener);
            }
            Err(e) => tracing::error!("Failed to bind IPC socket: {}", e),
        }
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

    pub fn resize_surface(&mut self, width: i32, height: i32) {
        if let Some(surf) = self.inner.surface.as_mut() {
            surf.set_size(width as u32, height as u32);
            surf.wl_surface.commit();
        }
        let _ = self.conn.flush();
    }

    pub fn surface_size(&self) -> (i32, i32) {
        self.inner.surface.as_ref().map_or((0, 0), |s| (s.width, s.height))
    }

    pub fn hide_surface(&mut self) {
        if let Some(surf) = self.inner.surface.as_mut() {
            surf.wl_surface.attach(None::<&wl_buffer::WlBuffer>, 0, 0);
            // Reset anchor and margin so the next show_surface defaults to centered
            surf.set_anchor(zwlr_layer_surface_v1::Anchor::empty());
            surf.set_margin(0, 0, 0, 0);
            surf.wl_surface.commit();
        }
        self.inner.surface_visible = false;
        let _ = self.conn.flush();
    }

    pub fn show_surface(&mut self, width: i32, height: i32) {
        {
            let surf = self.inner.surface.as_mut();
            if let Some(surf) = surf {
                surf.set_size(width as u32, height as u32);
                surf.set_keyboard_interactivity(
                    zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
                );
                surf.wl_surface.commit();
            }
        }
        let _ = self.conn.flush();
        let _ = self.event_queue.roundtrip(&mut *self.inner).ok();
        if let Some(surf) = self.inner.surface.as_mut() {
            if surf.buffer.is_some() && surf.mmap.is_some() {
                surf.rebuild_pixmap(width, height);
            }
        }
        self.inner.surface_visible = true;
    }

    pub fn mouse_y(&self) -> f32 {
        self.inner.input.mouse_y()
    }

    pub fn mouse_x(&self) -> f32 {
        self.inner.input.mouse_x()
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

    pub fn drain_shortcut_command(&mut self) -> Option<ShortcutAction> {
        self.inner.pending_shortcut.take()
    }

    /// Cursor blink — delegates to wisp::CursorBlink (GPUI-compatible).
    pub fn cursor_visible(&self) -> bool {
        self.inner.cursor_blink.visible()
    }

    /// Call on any user input — keeps cursor solid for 300ms (GPUI behavior).
    pub fn mark_input(&mut self) {
        self.inner.cursor_blink.mark_activity();
    }

    /// Read clipboard text from a data offer (deferred path: non-blocking, 200ms timeout).
    /// Called from `poll()` after dispatch so we never block inside an event handler.
    pub fn read_clipboard_text(&mut self, offer: ExtDataControlOfferV1) {
        let text = self.try_read_clipboard_offer(offer);
        if let Some(text) = text {
            if !text.is_empty() {
                let state = self.inner.clipboard.get_or_insert_with(|| omni_clipboard::ClipboardState::new());
                state.push(text);
            }
        }
    }

    /// Inner dispatcher-compatible variant: takes &Connection so it can be called from
    /// a `Dispatch` impl without borrowing the `EventQueue` from `&mut self`.
    pub fn inner_read_clipboard(conn: &Connection, inner: &mut Inner, offer: ExtDataControlOfferV1) {
        let mut fds: [RawFd; 2] = [-1, -1];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return;
        }
        let r = fds[0];
        let w = fds[1];
        let owned_w = unsafe { OwnedFd::from_raw_fd(w) };
        offer.receive("text/plain".to_string(), owned_w.as_fd());
        let _ = conn.flush();
        // Poll with 200ms timeout instead of blocking read_to_string.
        let mut pfd = libc::pollfd { fd: r, events: libc::POLLIN, revents: 0 };
        let ret = unsafe { libc::poll(&mut pfd, 1, 200) };
        drop(offer);
        if ret > 0 && pfd.revents & libc::POLLIN != 0 {
            use std::io::Read;
            let mut text = String::new();
            if std::io::BufReader::new(unsafe { std::fs::File::from_raw_fd(r) }).read_to_string(&mut text).is_ok()
                && !text.is_empty()
            {
                let state = inner.clipboard.get_or_insert_with(|| omni_clipboard::ClipboardState::new());
                state.push(text);
            }
        } else {
            unsafe { libc::close(r); }
        }
    }

    /// Non-blocking read of a clipboard offer. Returns the text if available within 200ms.
    fn try_read_clipboard_offer(&mut self, offer: ExtDataControlOfferV1) -> Option<String> {
        let mut fds: [RawFd; 2] = [-1, -1];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return None;
        }
        let r = fds[0];
        let w = fds[1];
        let owned_w = unsafe { OwnedFd::from_raw_fd(w) };
        offer.receive("text/plain".to_string(), owned_w.as_fd());
        let _ = self.conn.flush();
        drop(owned_w);
        let mut pfd = libc::pollfd { fd: r, events: libc::POLLIN, revents: 0 };
        let ret = unsafe { libc::poll(&mut pfd, 1, 200) };
        if ret > 0 && pfd.revents & libc::POLLIN != 0 {
            use std::io::Read;
            let mut text = String::new();
            let _ = std::io::BufReader::new(unsafe { std::fs::File::from_raw_fd(r) }).read_to_string(&mut text);
            Some(text)
        } else {
            unsafe { libc::close(r); }
            None
        }
    }

    /// Try to read a pending offer stored by the Selection dispatch handler.
    /// Returns the text if available; None if no pending offer or no data within 200ms.
    pub fn try_pending_clipboard_read(&mut self) -> Option<String> {
        let offer = self.inner.pending_read_offer.take()?;
        self.try_read_clipboard_offer(offer)
    }

    /// Set clipboard selection and prepare for paste.
    pub fn set_clipboard_selection(&mut self, text: &str) {
        let mgr = match self.inner.data_manager.as_ref() {
            Some(m) => m,
            None => return,
        };
        let qh = self.event_queue.handle();
        let source = mgr.create_data_source(&qh, ());
        source.offer("text/plain".to_string());
        source.offer("text/plain;charset=utf-8".to_string());
        self.inner.pending_paste_text = Some(text.to_string());
        self.inner.pending_source = Some(source.clone());
        if let Some(ref device) = self.inner.data_device {
            device.set_selection(Some(&source));
        }
        self.inner.skipping_next_read = true;
        let _ = self.conn.flush();
    }

    /// Get a handle to the clipboard state for use in main loop.
    pub fn clipboard_entries(&self) -> Vec<omni_clipboard::ClipboardEntry> {
        self.inner.clipboard.as_ref().map_or_else(Vec::new, |c| c.entries.clone())
    }

    /// Set clipboard selection and hide the surface.
    /// The user pastes manually with Ctrl+V.
    pub fn clipboard_paste(&mut self, text: &str) {
        tracing::info!("clipboard_paste: setting {} bytes", text.len());
        self.set_clipboard_selection(text);
        self.hide_surface();
    }

    pub fn poll(&mut self) -> bool {
        let _ = self.conn.flush();

        // Build poll fd set: wayland + ipc listener + ipc clients
        let mut poll_fds = Vec::new();
        poll_fds.push(libc::pollfd {
            fd: self.conn.as_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        });
        if let Some(ref listener) = self.inner.ipc_listener {
            poll_fds.push(libc::pollfd {
                fd: listener.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
        }
        let client_offset = poll_fds.len();
        for client in &self.inner.ipc_clients {
            poll_fds.push(libc::pollfd {
                fd: client.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
        }

        // Flush explicitly before prepare_read. If the compositor already closed the
        // connection (e.g. due to a protocol error), the flush will fail with EPIPE here,
        // and we can still dispatch any protocol-error event already in the socket buffer.
        if let Err(WaylandError::Io(_)) = self.conn.flush() {
            // The compositor likely disconnected. Try to dispatch any events that may
            // already be in the buffer (e.g. a protocol error from a previous read).
            tracing::error!("poll: flush before prepare failed with EPIPE");
            match self.event_queue.dispatch_pending(&mut *self.inner) {
                Err(wayland_client::DispatchError::Backend(WaylandError::Protocol(err))) => {
                    tracing::error!(
                        "wayland: protocol error (flush failed): object={}@{} code={} message={:?}, shutting down",
                        err.object_interface, err.object_id, err.code, err.message,
                    );
                }
                Err(e) => {
                    tracing::error!("wayland: disconnection, flush+dispatch failed: {e}");
                }
                Ok(_) => {
                    tracing::error!("wayland: compositor disconnected (flush EPIPE, no protocol error)");
                }
            }
            self.inner.running = false;
            return false;
        }

        let read_guard = self.event_queue.prepare_read();
        let poll_ret = unsafe {
            libc::poll(poll_fds.as_mut_ptr(), poll_fds.len() as _, 16)
        };
        if let Some(guard) = read_guard {
            let wl_revents = poll_fds[0].revents;
            if wl_revents & libc::POLLIN != 0 {
                match guard.read() {
                    Err(WaylandError::Protocol(err)) => {
                        tracing::error!(
                            "wayland: protocol error (poll_ret={poll_ret}, revents={wl_revents:#06x}): \
                             object={}@{} code={} message={:?}, shutting down",
                            err.object_interface, err.object_id, err.code, err.message,
                        );
                        self.inner.running = false;
                        return false;
                    }
                    Err(WaylandError::Io(e)) => {
                        // guard.read() flushes internally; if the compositor disconnected
                        // between our flush above and this one, try dispatch_pending.
                        match self.event_queue.dispatch_pending(&mut *self.inner) {
                            Err(wayland_client::DispatchError::Backend(WaylandError::Protocol(err))) => {
                                tracing::error!(
                                    "wayland: protocol error after io error: object={}@{} code={} message={:?}, shutting down",
                                    err.object_interface, err.object_id, err.code, err.message,
                                );
                            }
                            Err(dispatch_err) => {
                                tracing::error!(
                                    "wayland: io error (poll_ret={poll_ret}, revents={wl_revents:#06x}): {}; \
                                     dispatch error: {}",
                                    e, dispatch_err,
                                );
                            }
                            Ok(_) => {
                                tracing::error!(
                                    "wayland: io error (poll_ret={poll_ret}, revents={wl_revents:#06x}): {}, shutting down",
                                    e,
                                );
                            }
                        }
                        self.inner.running = false;
                        return false;
                    }
                    Ok(_) => {}
                }
            } else if wl_revents & (libc::POLLHUP | libc::POLLERR) != 0 {
                tracing::error!("wayland: fd hung up (revents={wl_revents:#06x}), shutting down");
                self.inner.running = false;
                return false;
            }
        } else if poll_fds[0].revents & (libc::POLLHUP | libc::POLLERR) != 0 {
            tracing::error!("wayland: fd error (revents={:#x}), shutting down", poll_fds[0].revents);
            self.inner.running = false;
            return false;
        }
        if let Err(e) = self.event_queue.dispatch_pending(&mut *self.inner) {
            if let wayland_client::DispatchError::Backend(WaylandError::Protocol(err)) = &e {
                tracing::error!(
                    "wayland: protocol error: object={}@{} code={} message={:?}, shutting down",
                    err.object_interface, err.object_id, err.code, err.message,
                );
            } else {
                tracing::error!("wayland: dispatch error: {e}");
            }
            self.inner.running = false;
            return false;
        }

        // Drain any pending clipboard read OUTSIDE the dispatch context so it can
        // never block inside an event handler. The non-blocking poll inside caps
        // each read at 200ms.
        if self.inner.pending_read_offer.is_some() {
            if let Some(text) = self.try_pending_clipboard_read() {
                if !text.is_empty() {
                    let state = self.inner.clipboard.get_or_insert_with(|| omni_clipboard::ClipboardState::new());
                    state.push(text);
                }
            }
        }

        // Accept new IPC connections
        if let Some(ref listener) = self.inner.ipc_listener {
            if poll_fds.get(1).is_some_and(|p| p.revents & libc::POLLIN != 0) {
                while let Ok((client, _)) = listener.accept() {
                    client.set_nonblocking(true).ok();
                    self.inner.ipc_clients.push(client);
                }
            }
        }

        // Read from IPC clients
        let mut to_remove: Vec<usize> = Vec::new();
        for (i, mut client) in self.inner.ipc_clients.iter().enumerate() {
            let pf_index = client_offset + i;
            if pf_index >= poll_fds.len() { break; }
            if poll_fds[pf_index].revents & libc::POLLIN == 0 { continue; }

            let mut buf = [0u8; 4096];
            match client.read(&mut buf) {
                Ok(0) => to_remove.push(i),
                Ok(n) => {
                    let cmds = omni_ipc::read_commands_from_buf(&mut self.inner.ipc_buf, &buf[..n]);
                    self.inner.ipc_actions.extend(cmds);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => to_remove.push(i),
            }
        }
        for i in to_remove.into_iter().rev() {
            self.inner.ipc_clients.swap_remove(i);
        }

        self.inner.running
    }

    pub fn drain_actions(&mut self) -> Vec<InputAction> {
        std::mem::take(&mut self.inner.actions)
    }

    pub fn drain_ipc_actions(&mut self) -> Vec<IpcCommand> {
        std::mem::take(&mut self.inner.ipc_actions)
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
        tracing::info!("quit requested via IPC");
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
