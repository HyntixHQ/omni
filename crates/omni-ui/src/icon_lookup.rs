use std::path::PathBuf;
use std::collections::HashMap;
use tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};

pub struct IconCache {
    cache: HashMap<String, Option<Pixmap>>,
    keys: Vec<String>, // For simple LRU eviction
    theme: String,
    size: u32,
    max_entries: usize,
}

impl IconCache {
    pub fn new(theme: String, size: u32) -> Self {
        Self {
            cache: HashMap::new(),
            keys: Vec::new(),
            theme,
            size,
            max_entries: 50, // Keep only 50 icons in memory
        }
    }

    pub fn get_icon(&mut self, icon_name: &str) -> Option<&Pixmap> {
        if !self.cache.contains_key(icon_name) {
            let pixmap = self.load_icon(icon_name);
            
            if self.cache.len() >= self.max_entries {
                if let Some(oldest) = self.keys.get(0).cloned() {
                    self.cache.remove(&oldest);
                    self.keys.remove(0);
                }
            }
            
            self.cache.insert(icon_name.to_string(), pixmap);
            self.keys.push(icon_name.to_string());
        } else {
            // Move to end of keys to keep it fresh
            if let Some(pos) = self.keys.iter().position(|k| k == icon_name) {
                let k = self.keys.remove(pos);
                self.keys.push(k);
            }
        }
        self.cache.get(icon_name).unwrap().as_ref()
    }

    pub fn get_icon_ref(&self, icon_name: &str) -> Option<&Pixmap> {
        self.cache.get(icon_name).and_then(|p| p.as_ref())
    }

    fn load_icon(&self, icon_name: &str) -> Option<Pixmap> {
        let path = find_icon(icon_name, &self.theme, self.size)?;
        let ext = path.extension()?.to_string_lossy().to_lowercase();

        if ext == "svg" {
            let svg_data = std::fs::read(&path).ok()?;
            let opt = Options::default();
            let tree = Tree::from_data(&svg_data, &opt).ok()?;

            let mut pixmap = Pixmap::new(self.size, self.size)?;
            
            // Calculate scale to fit icon into requested size
            let svg_size = tree.size().to_int_size();
            let scale_x = self.size as f32 / svg_size.width() as f32;
            let scale_y = self.size as f32 / svg_size.height() as f32;
            let scale = scale_x.min(scale_y);
            
            let transform = Transform::from_scale(scale, scale);
            resvg::render(&tree, transform, &mut pixmap.as_mut());
            Some(pixmap)
        } else if ext == "png" {
            let png_data = std::fs::read(&path).ok()?;
            let mut img = image::load_from_memory(&png_data).ok()?;
            
            if img.width() != self.size || img.height() != self.size {
                img = img.resize_exact(self.size, self.size, image::imageops::FilterType::Lanczos3);
            }
            
            let img_rgba = img.to_rgba8();
            let width = img_rgba.width();
            let height = img_rgba.height();
            
            let mut pixmap = Pixmap::new(width, height)?;
            let data = pixmap.data_mut();
            data.copy_from_slice(img_rgba.as_raw());
            
            // Pre-multiply alpha for tiny-skia
            for pixel in data.chunks_mut(4) {
                let a = pixel[3] as u32;
                pixel[0] = ((pixel[0] as u32 * a) / 255) as u8;
                pixel[1] = ((pixel[1] as u32 * a) / 255) as u8;
                pixel[2] = ((pixel[2] as u32 * a) / 255) as u8;
            }

            Some(pixmap)
        } else {
            None
        }
    }
}

pub fn find_icon(icon_name: &str, theme: &str, size: u32) -> Option<PathBuf> {
    if icon_name.starts_with('/') {
        let p = PathBuf::from(icon_name);
        if p.exists() {
            return Some(p);
        }
        for ext in ["svg", "png", "xpm"] {
            let p_ext = p.with_extension(ext);
            if p_ext.exists() {
                return Some(p_ext);
            }
        }
    }

    freedesktop_icons::lookup(icon_name)
        .with_theme(theme)
        .with_size(size as u16)
        .find()
}
