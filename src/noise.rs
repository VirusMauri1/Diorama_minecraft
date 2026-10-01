// Ruido y numeros aleatorios hechos a mano (sin el crate `rand`).

pub fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

pub fn hash3(x: i32, y: i32, z: i32) -> u32 {
    hash(
        (x as u32).wrapping_mul(0x8da6_b343)
            ^ hash((y as u32).wrapping_mul(0xd816_3841))
            ^ hash((z as u32).wrapping_mul(0xcb1a_b31f)),
    )
}

/// Numero aleatorio en [0, 1), siempre el mismo para la misma celda.
pub fn rand3(x: i32, y: i32, z: i32) -> f32 {
    (hash3(x, y, z) >> 8) as f32 / 16_777_216.0
}

pub fn rand2(x: i32, y: i32, seed: u32) -> f32 {
    rand3(x, y, seed as i32)
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Ruido suave 2D. Con `period > 0` se repite (texturas sin costuras).
pub fn vnoise_wrap(x: f32, y: f32, period: i32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = smooth(x - ix as f32);
    let fy = smooth(y - iy as f32);
    let w = |v: i32| if period > 0 { v.rem_euclid(period) } else { v };
    let a = rand2(w(ix), w(iy), seed);
    let b = rand2(w(ix + 1), w(iy), seed);
    let c = rand2(w(ix), w(iy + 1), seed);
    let d = rand2(w(ix + 1), w(iy + 1), seed);
    let top = a + (b - a) * fx;
    let bot = c + (d - c) * fx;
    top + (bot - top) * fy
}

pub fn vnoise(x: f32, y: f32, seed: u32) -> f32 {
    vnoise_wrap(x, y, 0, seed)
}

/// Generador de numeros aleatorios rapido (xorshift).
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Rng(hash(seed) | 1)
    }

    pub fn next_f32(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / 16_777_216.0
    }
}

/// Secuencia de Halton: posiciones de muestreo bien repartidas en el pixel.
pub fn halton(mut i: u32, base: u32) -> f32 {
    let mut f = 1.0;
    let mut r = 0.0;
    while i > 0 {
        f /= base as f32;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}
