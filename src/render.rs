// Raytracer: camara, luces con sombras, reflexion, refraccion y
// post-proceso (bloom y tonemapping). Renderiza en varios hilos.

use crate::math::{smoothstep, Vec3};
use crate::noise::{halton, hash, Rng};
use crate::dragon::Dragon;
use crate::skybox::Skybox;
use crate::world::{face_uv, Intersect, RayIntersect, World, AIR};
use std::sync::Mutex;
use std::thread;

const EPS: f32 = 1e-3;
const MAX_DEPTH: u32 = 6;

// ---------------------------------------------------------------- camara

/// Camara libre: posicion y hacia donde mira (yaw / pitch).
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub pos: Vec3,
    /// Giro horizontal (al aumentar gira a la izquierda).
    pub yaw: f32,
    /// Inclinacion (positivo = mirar hacia abajo).
    pub pitch: f32,
    pub fov: f32,
}

impl Camera {
    /// Centro de la isla: la vista inicial y la rotacion giran alrededor de el.
    pub const CENTER: Vec3 = Vec3::new(46.0, 54.0, 42.0);
    pub const DEFAULT_YAW: f32 = -0.55;
    pub const DEFAULT_PITCH: f32 = 0.20;
    pub const DEFAULT_DIST: f32 = 140.0;

    /// Direccion desde el punto observado hacia la camara.
    fn back(yaw: f32, pitch: f32) -> Vec3 {
        let (sp, cp) = pitch.sin_cos();
        let (sy, cy) = yaw.sin_cos();
        Vec3::new(cp * sy, sp, cp * cy)
    }

    /// Camara a distancia `dist` de `target`, mirando hacia el.
    pub fn orbit(target: Vec3, yaw: f32, pitch: f32, dist: f32) -> Self {
        Camera { pos: target + Self::back(yaw, pitch) * dist, yaw, pitch, fov: 45f32.to_radians() }
    }

    pub fn default_view() -> Self {
        Self::orbit(Self::CENTER, Self::DEFAULT_YAW, Self::DEFAULT_PITCH, Self::DEFAULT_DIST)
    }

    pub fn forward(&self) -> Vec3 {
        -Self::back(self.yaw, self.pitch)
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::UP).normalize()
    }

    /// Gira la camara alrededor de `center` (equivale a rotar el diorama).
    pub fn orbit_around(&mut self, center: Vec3, angle: f32) {
        let (s, c) = angle.sin_cos();
        let o = self.pos - center;
        self.pos = center + Vec3::new(o.x * c + o.z * s, o.y, o.z * c - o.x * s);
        self.yaw += angle;
    }

    /// Acerca (factor < 1) o aleja (factor > 1) la camara respecto a `center`.
    pub fn zoom_to(&mut self, center: Vec3, factor: f32) {
        let o = self.pos - center;
        let d = o.length().max(1e-3);
        self.pos = center + o * ((d * factor).clamp(8.0, 320.0) / d);
    }

    pub fn clamp(&mut self) {
        self.pitch = self.pitch.clamp(-1.5, 1.5);
        self.yaw %= std::f32::consts::TAU;
        let lim = 400.0;
        self.pos = Vec3::new(
            self.pos.x.clamp(Self::CENTER.x - lim, Self::CENTER.x + lim),
            self.pos.y.clamp(-lim, lim),
            self.pos.z.clamp(Self::CENTER.z - lim, Self::CENTER.z + lim),
        );
    }

    pub fn view(&self, w: usize, h: usize) -> View {
        let forward = self.forward();
        let right = self.right();
        let up = right.cross(forward);
        View {
            eye: self.pos,
            forward,
            right,
            up,
            tan_half: (self.fov * 0.5).tan(),
            aspect: w as f32 / h as f32,
        }
    }
}

pub struct View {
    eye: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    tan_half: f32,
    aspect: f32,
}

impl View {
    /// Pixel -> direccion del rayo (como en la clase).
    fn ray_dir(&self, px: f32, py: f32, w: usize, h: usize) -> Vec3 {
        let sx = (2.0 * px / w as f32 - 1.0) * self.aspect * self.tan_half;
        let sy = (1.0 - 2.0 * py / h as f32) * self.tan_half;
        (self.forward + self.right * sx + self.up * sy).normalize()
    }
}

// ---------------------------------------------------------------- luces

pub struct Spot {
    pub dir: Vec3,
    pub cos_outer: f32,
    pub cos_inner: f32,
}

pub struct Light {
    pub pos: Vec3,
    pub color: Vec3,
    /// Distancia a la que la intensidad baja a la mitad.
    pub range: f32,
    /// Distancia maxima de la luz (INFINITY = sin limite).
    pub max_dist: f32,
    pub spot: Option<Spot>,
    /// Que tan visible es el haz de luz (0 = sin haz).
    pub volumetric: f32,
    /// Si proyecta sombras.
    pub shadows: bool,
}

impl Light {
    pub fn point(pos: Vec3, color: Vec3, range: f32) -> Self {
        Light { pos, color, range, max_dist: f32::INFINITY, spot: None, volumetric: 0.0, shadows: true }
    }

    /// Luz direccional (la luna): una luz puntual muy lejana.
    pub fn directional(center: Vec3, dir_to_light: Vec3, color: Vec3) -> Self {
        Light::point(center + dir_to_light.normalize() * 5000.0, color, 1e9)
    }

    #[allow(dead_code)] // reflector: no se usa en esta escena
    pub fn spot(pos: Vec3, target: Vec3, color: Vec3, range: f32, outer_deg: f32, inner_deg: f32) -> Self {
        Light {
            pos,
            color,
            range,
            max_dist: f32::INFINITY,
            spot: Some(Spot {
                dir: (target - pos).normalize(),
                cos_outer: outer_deg.to_radians().cos(),
                cos_inner: inner_deg.to_radians().cos(),
            }),
            volumetric: 0.0,
            shadows: true,
        }
    }

    /// Direccion hacia la luz, distancia y atenuacion en el punto `p`.
    #[inline]
    fn illum(&self, p: Vec3) -> Option<(Vec3, f32, f32)> {
        let to = self.pos - p;
        let dist = to.length();
        if dist > self.max_dist || dist < 1e-4 {
            return None;
        }
        let l = to / dist;
        let mut att = 1.0 / (1.0 + (dist / self.range).powi(2));
        if self.max_dist.is_finite() {
            let x = dist / self.max_dist;
            att *= (1.0 - x * x).max(0.0).powi(2);
        }
        if let Some(s) = &self.spot {
            let c = (-l).dot(s.dir);
            if c <= s.cos_outer {
                return None;
            }
            att *= smoothstep(s.cos_outer, s.cos_inner, c);
        }
        Some((l, dist, att))
    }
}

// ---------------------------------------------------------------- escena

pub struct Scene {
    pub world: World,
    pub lights: Vec<Light>,
    pub skybox: Skybox,
    pub ambient_sky: Vec3,
    pub ambient_ground: Vec3,
    /// Centro de las ondas del agua (debajo del barco).
    pub ripple_center: Vec3,
    pub dragon: Dragon,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub beams: bool,
    pub bloom: bool,
    pub exposure: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { beams: true, bloom: true, exposure: 1.0 }
    }
}

fn schlick(cosi: f32, n1: f32, n2: f32) -> f32 {
    let mut cos = cosi;
    if n1 > n2 {
        let eta = n1 / n2;
        let sin2 = eta * eta * (1.0 - cosi * cosi);
        if sin2 >= 1.0 {
            return 1.0;
        }
        cos = (1.0 - sin2).sqrt();
    }
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cos).powi(5)
}

impl Scene {
    /// Normal de la superficie del agua con ondas.
    fn ripple_normal(&self, p: Vec3, n: Vec3) -> Vec3 {
        let dx = p.x - self.ripple_center.x;
        let dz = p.z - self.ripple_center.z;
        let r = (dx * dx + dz * dz).sqrt().max(1e-3);
        let fade = 1.0 / (1.0 + r * 0.06);
        let dr = -0.14 * 1.3 * (r * 1.3).sin() * fade;
        let w1 = (p.x * 0.7 + p.z * 0.3).cos() * 0.05;
        let w2 = (p.x * 1.9 - p.z * 1.3).cos() * 0.03;
        let gx = dr * dx / r + w1 * 0.7 + w2 * 1.9;
        let gz = dr * dz / r + w1 * 0.3 - w2 * 1.3;
        let up = Vec3::new(-gx, 1.0, -gz).normalize();
        if n.y > 0.0 {
            up
        } else {
            -up
        }
    }

    /// Cuanta luz llega de `from` a `to` (el agua y el vidrio la tinen).
    fn transmittance(&self, from: Vec3, to: Vec3) -> Vec3 {
        let delta = to - from;
        let mut remaining = delta.length();
        if remaining < 1e-5 {
            return Vec3::ONE;
        }
        let d = delta / remaining;
        let mut o = from;
        let mut t = Vec3::ONE;
        for _ in 0..8 {
            let medium = self.world.medium_at(o);
            match self.world.ray_intersect(o, d, remaining) {
                None => {
                    if medium != AIR {
                        t *= (self.world.absorption_of(medium) * -remaining).exp();
                    }
                    return t;
                }
                Some(h) => {
                    if h.medium != AIR {
                        t *= (self.world.absorption_of(h.medium) * -h.distance).exp();
                    }
                    if h.block != AIR {
                        let m = &self.world.materials[h.material];
                        if m.transparency <= 0.0 {
                            return Vec3::ZERO;
                        }
                        t *= m.transparency;
                    }
                    if t.max_elem() < 0.01 {
                        return Vec3::ZERO;
                    }
                    let adv = h.distance + 2e-3;
                    o = o + d * adv;
                    remaining -= adv;
                    if remaining <= 0.0 {
                        return t;
                    }
                }
            }
        }
        t
    }

    /// Luz de los haces volumetricos a lo largo del rayo.
    fn beams(&self, o: Vec3, d: Vec3, t_hit: f32, rng: &mut Rng) -> Vec3 {
        let Some((t0, t1)) = self.world.grid_interval(o, d) else {
            return Vec3::ZERO;
        };
        let t1 = t1.min(t_hit);
        if t1 <= t0 {
            return Vec3::ZERO;
        }
        const STEPS: usize = 14;
        let dt = (t1 - t0) / STEPS as f32;
        let j = rng.next_f32();
        let mut acc = Vec3::ZERO;
        for light in self.lights.iter().filter(|l| l.volumetric > 0.0) {
            for i in 0..STEPS {
                let p = o + d * (t0 + (i as f32 + j) * dt);
                let Some((l, _, att)) = light.illum(p) else {
                    continue;
                };
                let vis = self.transmittance(p, light.pos);
                if vis.max_elem() <= 0.0 {
                    continue;
                }
                let phase = 0.5 + 1.5 * l.dot(d).max(0.0).powi(4);
                acc += light.color * vis * (att * phase * light.volumetric * dt);
            }
        }
        acc
    }

    /// Equivalente a `cast_ray` de la clase: color que trae el rayo.
    fn cast_ray(&self, o: Vec3, d: Vec3, depth: u32, weight: f32, rng: &mut Rng, beams: bool) -> Vec3 {
        let hit = self.world.ray_intersect(o, d, f32::INFINITY);
        let mut col = match &hit {
            None => self.skybox.sample(d),
            Some(h) => {
                let mut c = self.shade(h, d, depth, weight, rng);
                if h.medium != AIR {
                    // Beer-Lambert: el agua absorbe mas rojo, por eso se ve azul
                    c *= (self.world.absorption_of(h.medium) * -h.distance).exp();
                }
                c
            }
        };
        if beams {
            col += self.beams(o, d, hit.map_or(f32::INFINITY, |h| h.distance), rng);
        }
        col
    }

    fn shade(&self, h: &Intersect, d: Vec3, depth: u32, weight: f32, rng: &mut Rng) -> Vec3 {
        let w = &self.world;
        let m = &w.materials[h.material];
        let (u, v) = face_uv(h.point, h.normal, m.tex_scale);
        let tex = w.textures[m.texture].sample(u, v);
        let base = Vec3::new(tex[0], tex[1], tex[2]);

        let mut n = h.normal;
        if m.ripple && n.y.abs() > 0.5 {
            n = self.ripple_normal(h.point, n);
        }
        let p_out = h.point + h.normal * EPS;
        let view = -d;

        // Luz ambiental (cielo arriba, suelo abajo) con oclusion ambiental
        let ao = if h.full_face && depth <= 1 { w.ambient_occlusion(h.cell, h.normal, h.point) } else { 1.0 };
        let hemi = n.y * 0.5 + 0.5;
        let mut diffuse = self.ambient_ground.lerp(self.ambient_sky, hemi) * ao;
        let mut spec = Vec3::ZERO;

        if m.albedo > 0.0 || m.specular > 0.0 {
            for light in &self.lights {
                let Some((l, _, att)) = light.illum(h.point) else {
                    continue;
                };
                let ndl = n.dot(l);
                if ndl <= 0.0 || h.normal.dot(l) <= 0.0 {
                    continue;
                }
                let vis = if light.shadows { self.transmittance(p_out, light.pos) } else { Vec3::ONE };
                if vis.max_elem() <= 1e-3 {
                    continue;
                }
                let lc = light.color * vis * att;
                diffuse += lc * ndl;
                let r = (-l).reflect(n);
                spec += lc * r.dot(view).max(0.0).powf(m.shininess);
            }
        }

        // Fresnel: en angulos bajos se refleja mas y se refracta menos
        let mut kr = m.reflectivity;
        let mut kt = m.transparency;
        let mut emission = m.emission;
        if kt > 0.0 && !m.cutout {
            // El alfa de la textura es la opacidad de cada pixel (vetas del portal)
            let cover = tex[3];
            kt = 1.0 - (1.0 - kt) * cover;
            emission *= cover;
        }
        let n1 = w.ior_of(h.medium);
        let n2 = if h.block == AIR { 1.0 } else { m.ior };
        if kt > 0.0 {
            let f = schlick((-d.dot(n)).clamp(0.0, 1.0), n1, n2);
            kr += kt * f;
            kt *= 1.0 - f;
        }
        let kd = (1.0 - kr - kt).max(0.0);
        let mut col = (base * diffuse * m.albedo + spec * m.specular) * kd + base * emission;

        if depth < MAX_DEPTH {
            if kr * weight > 0.01 {
                let mut rd = d.reflect(n);
                if rd.dot(h.normal) <= 0.0 {
                    rd = d.reflect(h.normal);
                }
                col += self.cast_ray(p_out, rd, depth + 1, weight * kr, rng, false) * kr;
            }
            if kt * weight > 0.01 {
                let tint = Vec3::ONE.lerp(base, 0.25);
                match d.refract(n, n1 / n2) {
                    Some(td) if td.dot(h.normal) < 0.0 => {
                        let p_in = h.point - h.normal * EPS;
                        col += self.cast_ray(p_in, td, depth + 1, weight * kt, rng, false) * tint * kt;
                    }
                    _ => {
                        // Reflexion interna total
                        let rd = d.reflect(h.normal);
                        col += self.cast_ray(p_out, rd, depth + 1, weight * kt, rng, false) * kt;
                    }
                }
            }
        }
        col
    }

    #[allow(clippy::too_many_arguments)]
    fn trace_pixel(&self, view: &View, settings: &Settings, x: usize, y: usize, w: usize, h: usize, sample: u32) -> Vec3 {
        let mut rng = Rng::new(hash((y * w + x) as u32) ^ hash(sample.wrapping_mul(0x9e37_79b9)));
        let (jx, jy) = if sample == 0 { (0.5, 0.5) } else { (halton(sample, 2), halton(sample, 3)) };
        let d = view.ray_dir(x as f32 + jx, y as f32 + jy, w, h);
        let c = self.cast_ray(view.eye, d, 0, 1.0, &mut rng, settings.beams);
        // Evitar pixeles demasiado brillantes
        c.min_each(40.0)
    }
}

// ---------------------------------------------------------------- render multihilo

pub fn thread_count() -> usize {
    thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

/// Renderiza las filas [y0, y1) y las mezcla en `buf` con peso `blend`.
#[allow(clippy::too_many_arguments)]
pub fn render_rows(
    scene: &Scene,
    cam: &Camera,
    settings: &Settings,
    w: usize,
    h: usize,
    y0: usize,
    y1: usize,
    sample: u32,
    buf: &mut [Vec3],
    blend: f32,
) {
    let view = cam.view(w, h);
    let rows = &mut buf[y0 * w..y1 * w];
    const BAND: usize = 2;
    let jobs: Vec<(usize, &mut [Vec3])> =
        rows.chunks_mut(w * BAND).enumerate().map(|(i, c)| (y0 + i * BAND, c)).collect();
    let queue = Mutex::new(jobs);
    thread::scope(|s| {
        for _ in 0..thread_count() {
            s.spawn(|| loop {
                let job = queue.lock().unwrap().pop();
                let Some((ystart, chunk)) = job else {
                    break;
                };
                for (k, px) in chunk.iter_mut().enumerate() {
                    let c = scene.trace_pixel(&view, settings, k % w, ystart + k / w, w, h, sample);
                    *px = if blend >= 1.0 { c } else { *px * (1.0 - blend) + c * blend };
                }
            });
        }
    });
}

// ---------------------------------------------------------------- post-proceso

fn gaussian_blur(src: &[Vec3], w: usize, h: usize, sigma: f32) -> Vec<Vec3> {
    let r = (sigma * 3.0).ceil() as i32;
    let kernel: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let norm: f32 = kernel.iter().sum();
    let mut tmp = vec![Vec3::ZERO; w * h];
    let mut out = vec![Vec3::ZERO; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = Vec3::ZERO;
            for (k, &wt) in kernel.iter().enumerate() {
                let xx = (x as i32 + k as i32 - r).clamp(0, w as i32 - 1) as usize;
                acc += src[y * w + xx] * wt;
            }
            tmp[y * w + x] = acc / norm;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut acc = Vec3::ZERO;
            for (k, &wt) in kernel.iter().enumerate() {
                let yy = (y as i32 + k as i32 - r).clamp(0, h as i32 - 1) as usize;
                acc += tmp[yy * w + x] * wt;
            }
            out[y * w + x] = acc / norm;
        }
    }
    out
}

fn aces(x: f32) -> f32 {
    ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
}

/// Imagen HDR -> pixeles finales: bloom, tonemapping, gamma y vineta.
pub fn post_process(hdr: &[Vec3], w: usize, h: usize, out: &mut [u32], settings: &Settings) {
    const F: usize = 4;
    let sw = w.div_ceil(F);
    let sh = h.div_ceil(F);
    let bloom = if settings.bloom {
        let mut bright = vec![Vec3::ZERO; sw * sh];
        for sy in 0..sh {
            for sx in 0..sw {
                let mut acc = Vec3::ZERO;
                let mut n = 0.0f32;
                for y in sy * F..((sy + 1) * F).min(h) {
                    for x in sx * F..((sx + 1) * F).min(w) {
                        let c = hdr[y * w + x];
                        let l = c.luminance();
                        if l > 0.9 {
                            acc += c * ((l - 0.9) / l);
                        }
                        n += 1.0;
                    }
                }
                bright[sy * sw + sx] = acc / n.max(1.0);
            }
        }
        let a = gaussian_blur(&bright, sw, sh, 1.5);
        let b = gaussian_blur(&bright, sw, sh, 6.0);
        Some(a.iter().zip(&b).map(|(&a, &b)| a * 0.6 + b * 0.9).collect::<Vec<_>>())
    } else {
        None
    };

    for y in 0..h {
        for x in 0..w {
            let mut c = hdr[y * w + x];
            if let Some(bl) = &bloom {
                let fx = ((x as f32 + 0.5) / F as f32 - 0.5).clamp(0.0, (sw - 1) as f32);
                let fy = ((y as f32 + 0.5) / F as f32 - 0.5).clamp(0.0, (sh - 1) as f32);
                let (x0, y0) = (fx as usize, fy as usize);
                let (x1, y1) = ((x0 + 1).min(sw - 1), (y0 + 1).min(sh - 1));
                let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
                let top = bl[y0 * sw + x0].lerp(bl[y0 * sw + x1], tx);
                let bot = bl[y1 * sw + x0].lerp(bl[y1 * sw + x1], tx);
                c += top.lerp(bot, ty);
            }
            let ux = x as f32 / w as f32 - 0.5;
            let uy = y as f32 / h as f32 - 0.5;
            let vignette = 1.0 - 0.45 * (ux * ux + uy * uy);
            c = c * (settings.exposure * vignette);
            let dither = ((hash((y * w + x) as u32) & 255) as f32 / 255.0 - 0.5) / 255.0;
            let to8 = |v: f32| ((aces(v).powf(1.0 / 2.2) + dither) * 255.0).clamp(0.0, 255.0) as u32;
            out[y * w + x] = (to8(c.x) << 16) | (to8(c.y) << 8) | to8(c.z);
        }
    }
}

/// Agranda una imagen pequena (para no mostrar filas vacias al refinar).
pub fn upscale_into(src: &[Vec3], sw: usize, sh: usize, dst: &mut [Vec3], w: usize, h: usize) {
    for y in 0..h {
        let sy = (y * sh / h).min(sh - 1);
        for x in 0..w {
            let sx = (x * sw / w).min(sw - 1);
            dst[y * w + x] = src[sy * sw + sx];
        }
    }
}
