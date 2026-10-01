use crate::math::{smoothstep, Vec3};
use crate::noise::{rand3, vnoise};
use crate::texture::Texture;

pub const FACE_NAMES: [&str; 6] = ["px", "nx", "py", "ny", "pz", "nz"];

/// Direccion hacia la luna 
pub const MOON_DIR: Vec3 = Vec3::new(0.62, 0.30, -0.72);

pub struct Skybox {
    pub faces: Vec<Texture>,
}

fn face_dir(face: usize, u: f32, v: f32) -> Vec3 {
    let sc = 2.0 * u - 1.0;
    let tc = 2.0 * v - 1.0;
    match face {
        0 => Vec3::new(1.0, -tc, -sc),
        1 => Vec3::new(-1.0, -tc, sc),
        2 => Vec3::new(sc, 1.0, tc),
        3 => Vec3::new(sc, -1.0, -tc),
        4 => Vec3::new(sc, -tc, 1.0),
        _ => Vec3::new(-sc, -tc, -1.0),
    }
    .normalize()
}

impl Skybox {
    pub fn generate(size: usize) -> Self {
        let faces = (0..6)
            .map(|f| {
                let mut data = Vec::with_capacity(size * size);
                for y in 0..size {
                    for x in 0..size {
                        let d = face_dir(f, (x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
                        let c = sky_color(d);
                        data.push([c.x, c.y, c.z, 1.0]);
                    }
                }
                Texture::from_linear(&format!("sky_{}", FACE_NAMES[f]), size, size, data)
            })
            .collect();
        Skybox { faces }
    }

    pub fn sample(&self, d: Vec3) -> Vec3 {
        let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
        let (face, ma, sc, tc) = if ax >= ay && ax >= az {
            if d.x > 0.0 {
                (0, ax, -d.z, -d.y)
            } else {
                (1, ax, d.z, -d.y)
            }
        } else if ay >= az {
            if d.y > 0.0 {
                (2, ay, d.x, d.z)
            } else {
                (3, ay, d.x, -d.z)
            }
        } else if d.z > 0.0 {
            (4, az, d.x, -d.y)
        } else {
            (5, az, -d.x, -d.y)
        };
        let u = (sc / ma + 1.0) * 0.5;
        let v = (tc / ma + 1.0) * 0.5;
        let c = self.faces[face].sample_bilinear(u, v);
        Vec3::new(c[0], c[1], c[2])
    }
}

fn sky_color(d: Vec3) -> Vec3 {
    let y = d.y;
    let moon = MOON_DIR.normalize();
    let horizon = Vec3::new(0.050, 0.060, 0.120);
    let mid = Vec3::new(0.018, 0.024, 0.065);
    let zenith = Vec3::new(0.004, 0.006, 0.022);
    let t = y.max(0.0);
    let mut col = if t < 0.25 { horizon.lerp(mid, t / 0.25) } else { mid.lerp(zenith, ((t - 0.25) / 0.75).min(1.0)) };

    // Estrellas
    if y > -0.05 {
        let q = 320.0;
        let (cx, cy, cz) = ((d.x * q).floor() as i32, (d.y * q).floor() as i32, (d.z * q).floor() as i32);
        let r = rand3(cx, cy, cz);
        if r > 0.9984 {
            let b = 0.6 + 3.0 * rand3(cz, cx, cy).powi(3);
            let tint = Vec3::new(0.85, 0.9, 1.0).lerp(Vec3::new(1.0, 0.92, 0.8), rand3(cy, cz, cx));
            col += tint * (b * smoothstep(-0.05, 0.2, y));
        }
    }

    // Halo de la luna y luna cuadrada con crateres
    let md = d.dot(moon);
    col += Vec3::new(0.35, 0.45, 0.8) * (md.max(0.0).powf(30.0) * 0.25);
    col += Vec3::new(0.5, 0.6, 0.9) * (md.max(0.0).powf(400.0) * 0.5);
    if md > 0.0 {
        let su = moon.cross(Vec3::UP).normalize();
        let sv = su.cross(moon);
        let a = d.dot(su) / md;
        let b = d.dot(sv) / md;
        let m = a.abs().max(b.abs());
        if m < 0.06 {
            let px = ((a / 0.06 + 1.0) * 4.0).floor() as i32;
            let py = ((b / 0.06 + 1.0) * 4.0).floor() as i32;
            let crater = rand3(px, py, 77) > 0.72;
            col = if crater { Vec3::new(1.6, 1.7, 1.95) } else { Vec3::new(2.6, 2.7, 3.0) };
        }
    }

    // Nubes en bloques
    if y > 0.03 {
        let k = 1.0 / y;
        let px = (d.x * k * 9.0).floor();
        let pz = (d.z * k * 9.0).floor();
        let n = vnoise(px * 0.18, pz * 0.18, 41) * 0.7 + vnoise(px * 0.5, pz * 0.5, 42) * 0.3;
        if n > 0.62 {
            let fade = smoothstep(0.03, 0.18, y);
            let lit = Vec3::new(0.035, 0.042, 0.075).lerp(Vec3::new(0.12, 0.14, 0.22), md.max(0.0).powi(3));
            let shade = if rand3(px as i32, pz as i32, 43) > 0.5 { 1.0 } else { 0.85 };
            col = col.lerp(lit * shade, 0.9 * fade);
        }
    }

    // Bajo el horizonte
    if y < 0.0 {
        let g = smoothstep(0.0, -0.6, y);
        col = horizon.lerp(Vec3::new(0.006, 0.008, 0.02), g.sqrt());
        if y < -0.02 {
            let k = 1.0 / -y;
            let px = (d.x * k * 14.0).floor();
            let pz = (d.z * k * 14.0).floor();
            let n = vnoise(px * 0.15, pz * 0.15, 47) * 0.7 + vnoise(px * 0.45, pz * 0.45, 48) * 0.3;
            if n > 0.56 {
                let fade = smoothstep(-0.02, -0.12, y);
                let top = Vec3::new(0.022, 0.027, 0.055).lerp(Vec3::new(0.045, 0.055, 0.1), (n - 0.56) * 2.3);
                let top = top * (1.0 + 0.6 * md.max(0.0).powi(4));
                col = col.lerp(top, 0.8 * fade);
            }
        }
    }
    col
}
