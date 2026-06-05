use std::os::fd::FromRawFd;

use libc::{c_char, off_t};
use memmap2::MmapMut;
use wayland_client::protocol::{wl_buffer, wl_compositor, wl_shm, wl_shm_pool, wl_surface};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    self, ZwlrLayerSurfaceV1,
};

const SHM_NAME: &[u8] = b"wisp-shm\0";

/// Wraps a Wayland surface + layer-surface with SHM buffer lifecycle.
/// Objects are created externally and injected — dispatch routing is the caller's responsibility.
pub struct WispSurface {
    pub compositor: wl_compositor::WlCompositor,
    pub layer_shell: ZwlrLayerShellV1,
    pub wl_surface: wl_surface::WlSurface,
    pub layer_surface: ZwlrLayerSurfaceV1,
    pub shm: Option<wl_shm::WlShm>,
    pub pool: Option<wl_shm_pool::WlShmPool>,
    pub buffer: Option<wl_buffer::WlBuffer>,
    pub pixmap: tiny_skia::Pixmap,
    pub mmap: Option<MmapMut>,
    pub width: i32,
    pub height: i32,
}

impl WispSurface {
    pub fn new(
        compositor: wl_compositor::WlCompositor,
        layer_shell: ZwlrLayerShellV1,
        wl_surface: wl_surface::WlSurface,
        layer_surface: ZwlrLayerSurfaceV1,
    ) -> Self {
        Self {
            compositor,
            layer_shell,
            wl_surface,
            layer_surface,
            shm: None,
            pool: None,
            buffer: None,
            pixmap: tiny_skia::Pixmap::new(1, 1).expect("tiny pixmap placeholder"),
            mmap: None,
            width: 0,
            height: 0,
        }
    }

    pub fn set_shm(&mut self, shm: wl_shm::WlShm) {
        self.shm = Some(shm);
    }

    /// Set new pool/buffer/mmap without destroying old ones.
    /// Old pool/buffer are leaked to avoid sending destroy-while-unreleased, which
    /// is a Wayland protocol violation.  Resources are reclaimed on connection close.
    pub fn set_buffers(
        &mut self,
        pool: wl_shm_pool::WlShmPool,
        buffer: wl_buffer::WlBuffer,
        mmap: MmapMut,
    ) {
        if let Some(old) = self.buffer.take() {
            std::mem::forget(old);
        }
        if let Some(old) = self.pool.take() {
            std::mem::forget(old);
        }
        self.pool = Some(pool);
        self.buffer = Some(buffer);
        self.mmap = Some(mmap);
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn set_size(&self, width: u32, height: u32) {
        self.layer_surface.set_size(width, height);
    }

    pub fn set_anchor(&self, anchor: zwlr_layer_surface_v1::Anchor) {
        self.layer_surface.set_anchor(anchor);
    }

    pub fn set_margin(&self, top: i32, right: i32, bottom: i32, left: i32) {
        self.layer_surface.set_margin(top, right, bottom, left);
    }

    pub fn set_keyboard_interactivity(&self, mode: zwlr_layer_surface_v1::KeyboardInteractivity) {
        self.layer_surface.set_keyboard_interactivity(mode);
    }

    pub fn set_exclusive_zone(&self, zone: i32) {
        self.layer_surface.set_exclusive_zone(zone);
    }

    pub fn ack_configure(&self, serial: u32) {
        self.layer_surface.ack_configure(serial);
    }

    pub fn configure(&mut self, serial: u32, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.width = width as i32;
            self.height = height as i32;
        }
        self.layer_surface.ack_configure(serial);
    }

    pub fn allocate_shm_mmap(&mut self) -> MmapMut {
        let w = self.width;
        let h = self.height;
        let stride = w * 4;
        let buf_size = (h * stride) as usize;

        self.pixmap = tiny_skia::Pixmap::new(w as u32, h as u32).expect("pixmap");

        let raw = create_memfd(buf_size);
        let dup = unsafe { libc::fcntl(raw, libc::F_DUPFD_CLOEXEC, 0) };
        let shm_file = unsafe { std::fs::File::from_raw_fd(dup) };
        let mmap = unsafe { MmapMut::map_mut(&shm_file) }.expect("mmap");
        let _ = raw; // memfd fd consumed by OwnedFd in caller

        mmap
    }

    pub fn commit(&self) {
        if let Some(buffer) = &self.buffer {
            self.wl_surface.attach(Some(buffer), 0, 0);
            self.wl_surface.damage_buffer(0, 0, self.width, self.height);
            self.wl_surface.commit();
        }
    }

    /// Recreate the pixmap without touching buffers/mmap — used when re-showing
    /// at the same size as before.
    pub fn rebuild_pixmap(&mut self, width: i32, height: i32) {
        self.pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).expect("pixmap");
    }

    pub fn hide(&self) {
        self.wl_surface
            .attach(None as Option<&wl_buffer::WlBuffer>, 0, 0);
        self.wl_surface.commit();
    }

    pub fn destroy(self) {
        if let Some(buffer) = self.buffer {
            buffer.destroy();
        }
        if let Some(pool) = self.pool {
            pool.destroy();
        }
        self.layer_surface.destroy();
        self.wl_surface.destroy();
    }
}

fn create_memfd(size: usize) -> std::os::unix::io::RawFd {
    unsafe {
        let name = SHM_NAME.as_ptr() as *const c_char;
        let fd = libc::memfd_create(name, 0);
        if fd < 0 {
            panic!("memfd_create: {}", std::io::Error::last_os_error());
        }
        libc::ftruncate(fd, size as off_t);
        fd
    }
}
