use crate::image_io;
use std::path::Path;

pub struct Texture {
    pub name: String,
    pub w: usize,
    pub h: usize,
    pub data: Vec<[f32; 4]>,
}

fn srgb_to_linear(c: f32) -> f32 {
    c.clamp(0.0, 1.0).powf(2.2)
}

fn linear_to_srgb(c: f32) -> f32 {
    c.max(0.0).powf(1.0 / 2.2).min(1.0)
}

impl Texture {
    /// Crea una textura con `f(x, y)`, que devuelve color sRGB + alfa.
    pub fn from_fn(name: &str, w: usize, h: usize, f: impl Fn(usize, usize) -> [f32; 4]) -> Self {
        let mut data = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let c = f(x, y);
                data.push([srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2]), c[3]]);
            }
        }
        Texture { name: name.to_string(), w, h, data }
    }

    /// Crea una textura con valores lineales (para el skybox).
    pub fn from_linear(name: &str, w: usize, h: usize, data: Vec<[f32; 4]>) -> Self {
        Texture { name: name.to_string(), w, h, data }
    }

    /// Muestreo pixelado (nearest) con repeticion.
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let x = ((u * self.w as f32).floor() as i32).rem_euclid(self.w as i32) as usize;
        let y = ((v * self.h as f32).floor() as i32).rem_euclid(self.h as i32) as usize;
        self.data[y * self.w + x]
    }

    /// Muestreo suavizado (bilineal), para el skybox.
    pub fn sample_bilinear(&self, u: f32, v: f32) -> [f32; 4] {
        let fx = (u * self.w as f32 - 0.5).clamp(0.0, (self.w - 1) as f32);
        let fy = (v * self.h as f32 - 0.5).clamp(0.0, (self.h - 1) as f32);
        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1).min(self.w - 1);
        let y1 = (y0 + 1).min(self.h - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let p = |x: usize, y: usize| self.data[y * self.w + x];
        let (a, b, c, d) = (p(x0, y0), p(x1, y0), p(x0, y1), p(x1, y1));
        let mut out = [0.0; 4];
        for i in 0..4 {
            let top = a[i] + (b[i] - a[i]) * tx;
            let bot = c[i] + (d[i] - c[i]) * tx;
            out[i] = top + (bot - top) * ty;
        }
        out
    }

    fn to_rgba8(&self) -> Vec<[u8; 4]> {
        self.data
            .iter()
            .map(|c| {
                [
                    (linear_to_srgb(c[0]) * 255.0 + 0.5) as u8,
                    (linear_to_srgb(c[1]) * 255.0 + 0.5) as u8,
                    (linear_to_srgb(c[2]) * 255.0 + 0.5) as u8,
                    (c[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                ]
            })
            .collect()
    }

    pub fn save_bmp(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        image_io::write_bmp(&dir.join(format!("{}.bmp", self.name)), self.w, self.h, &self.to_rgba8())
    }

    /// Usa `dir/<nombre>.bmp` en lugar de la textura generada, si existe.
    pub fn try_override(&mut self, dir: &Path) -> bool {
        let path = dir.join(format!("{}.bmp", self.name));
        if !path.exists() {
            return false;
        }
        match image_io::read_bmp(&path) {
            Ok((w, h, px)) => {
                self.w = w;
                self.h = h;
                self.data = px
                    .iter()
                    .map(|p| {
                        [
                            srgb_to_linear(p[0] as f32 / 255.0),
                            srgb_to_linear(p[1] as f32 / 255.0),
                            srgb_to_linear(p[2] as f32 / 255.0),
                            p[3] as f32 / 255.0,
                        ]
                    })
                    .collect();
                true
            }
            Err(e) => {
                eprintln!("No se pudo cargar {}: {e}", path.display());
                false
            }
        }
    }
}
