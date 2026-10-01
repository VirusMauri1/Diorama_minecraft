// Texturas procedurales estilo Minecraft (pixel art 16x16), una por material.

use crate::noise::{rand2, vnoise_wrap};
use crate::texture::Texture;
use std::f32::consts::TAU;

type C = [f32; 3];

fn mul(c: C, f: f32) -> C {
    [c[0] * f, c[1] * f, c[2] * f]
}

fn mix(a: C, b: C, t: f32) -> C {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn op(c: C) -> [f32; 4] {
    [c[0], c[1], c[2], 1.0]
}

const CLEAR: [f32; 4] = [0.0; 4];

fn grain(x: usize, y: usize, seed: u32, amt: f32) -> f32 {
    1.0 + (rand2(x as i32, y as i32, seed) - 0.5) * 2.0 * amt
}

/// Manchas suaves que se repiten sin costura.
fn blot(x: usize, y: usize, seed: u32) -> f32 {
    vnoise_wrap(x as f32 / 4.0, y as f32 / 4.0, 4, seed)
}

fn dist_center(x: usize, y: usize) -> f32 {
    ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt() / 8.0
}

const GRASS: C = [0.40, 0.64, 0.25];
const DIRT: C = [0.55, 0.39, 0.27];
const STONE: C = [0.50, 0.50, 0.50];

fn grass_px(x: usize, y: usize) -> C {
    mul(GRASS, grain(x, y, 1, 0.13) * (0.86 + 0.28 * blot(x, y, 2)))
}

fn dirt_px(x: usize, y: usize) -> C {
    let mut c = mul(DIRT, grain(x, y, 3, 0.12));
    if rand2(x as i32, y as i32, 4) > 0.88 {
        c = mul(c, 0.7);
    }
    c
}

fn stone_px(x: usize, y: usize) -> C {
    let mut f = grain(x, y, 5, 0.08) * (0.88 + 0.24 * blot(x, y, 6));
    if rand2(x as i32, y as i32, 7) > 0.9 {
        f *= 0.8;
    }
    mul(STONE, f)
}

pub fn grass_top() -> Texture {
    Texture::from_fn("grass_top", 16, 16, |x, y| op(grass_px(x, y)))
}

pub fn grass_side() -> Texture {
    Texture::from_fn("grass_side", 16, 16, |x, y| {
        let edge = 3 + (rand2(x as i32, 0, 8) * 2.5) as usize;
        if y < edge {
            op(grass_px(x, y))
        } else {
            op(dirt_px(x, y))
        }
    })
}

/// Pasto con petalos rosas caidos de los cerezos.
pub fn grass_petals() -> Texture {
    Texture::from_fn("grass_petals", 16, 16, |x, y| {
        if rand2(x as i32, y as i32, 9) > 0.80 && blot(x, y, 10) > 0.35 {
            let pink = if rand2(x as i32, y as i32, 11) > 0.5 { [0.98, 0.74, 0.86] } else { [0.92, 0.56, 0.74] };
            op(pink)
        } else {
            op(grass_px(x, y))
        }
    })
}

pub fn dirt() -> Texture {
    Texture::from_fn("dirt", 16, 16, |x, y| op(dirt_px(x, y)))
}

pub fn stone() -> Texture {
    Texture::from_fn("stone", 16, 16, |x, y| op(stone_px(x, y)))
}

/// Mineral: piedra con vetas de color.
pub fn ore(name: &str, color: C, seed: u32) -> Texture {
    Texture::from_fn(name, 16, 16, move |x, y| {
        let cluster = blot(x, y, seed) > 0.55 && rand2(x as i32, y as i32, seed + 1) > 0.45;
        if cluster {
            op(mul(color, grain(x, y, seed + 2, 0.15)))
        } else {
            op(stone_px(x, y))
        }
    })
}

pub fn sand() -> Texture {
    Texture::from_fn("sand", 16, 16, |x, y| op(mul([0.86, 0.80, 0.58], grain(x, y, 12, 0.07))))
}

/// Agua azul con ondas (32x32, cubre 2x2 bloques).
pub fn water() -> Texture {
    Texture::from_fn("water", 32, 32, |x, y| {
        let fx = x as f32 / 32.0;
        let fy = y as f32 / 32.0;
        let s = (TAU * (fy * 3.0 + 0.2 * (TAU * fx * 2.0).sin())).sin();
        let base = mul([0.22, 0.40, 0.80], grain(x, y, 13, 0.05));
        if s > 0.85 {
            op([0.42, 0.62, 0.95])
        } else if s < -0.9 {
            op(mul(base, 0.82))
        } else {
            op(base)
        }
    })
}

/// Hojas de cerezo con huecos de 2x2 pixeles (se ve a traves, no son translucidas).
pub fn cherry_leaves() -> Texture {
    Texture::from_fn("cherry_leaves", 16, 16, |x, y| {
        let (xi, yi) = (x as i32, y as i32);
        let clump = rand2(xi / 2, yi / 2, 17) > 0.58;
        if clump || rand2(xi, yi, 14) > 0.86 {
            return CLEAR;
        }
        let shades: [C; 4] = [[0.96, 0.72, 0.84], [0.90, 0.58, 0.74], [0.99, 0.84, 0.91], [0.80, 0.47, 0.63]];
        let i = (rand2(x as i32, y as i32, 15) * 4.0) as usize % 4;
        op(mul(shades[i], 0.9 + 0.2 * blot(x, y, 16)))
    })
}

/// Corteza de tronco (lado).
pub fn bark(name: &str, base: C, seed: u32) -> Texture {
    Texture::from_fn(name, 16, 16, move |x, y| {
        let mut f = grain(x, y, seed, 0.1);
        if (x + (rand2(x as i32, 0, seed + 1) * 3.0) as usize) % 4 == 0 {
            f *= 0.68;
        }
        if rand2(x as i32, y as i32 / 3, seed + 2) > 0.85 {
            f *= 1.2;
        }
        op(mul(base, f))
    })
}

/// Tapa del tronco con anillos.
pub fn log_top(name: &str, bark: C, inner: C, seed: u32) -> Texture {
    Texture::from_fn(name, 16, 16, move |x, y| {
        if x == 0 || y == 0 || x == 15 || y == 15 {
            return op(mul(bark, grain(x, y, seed, 0.1)));
        }
        let ring = ((dist_center(x, y) * 7.0) as i32) % 2 == 0;
        op(mul(inner, if ring { 1.0 } else { 0.82 } * grain(x, y, seed + 1, 0.05)))
    })
}

pub fn cherry_planks() -> Texture {
    Texture::from_fn("cherry_planks", 16, 16, |x, y| {
        let mut f = grain(x, y, 17, 0.06);
        if y % 4 == 0 {
            f *= 0.72;
        }
        if (x + (y / 4) * 5) % 16 == 0 {
            f *= 0.78;
        }
        op(mul([0.89, 0.66, 0.64], f))
    })
}

pub fn blackstone() -> Texture {
    Texture::from_fn("blackstone", 16, 16, |x, y| {
        let mut f = grain(x, y, 18, 0.12) * (0.85 + 0.3 * blot(x, y, 19));
        if x == 0 || y == 0 {
            f *= 0.7;
        }
        let mut c = mul([0.18, 0.14, 0.20], f);
        if rand2(x as i32, y as i32, 20) > 0.93 {
            c = [0.34, 0.24, 0.40];
        }
        op(c)
    })
}

pub fn obsidian() -> Texture {
    Texture::from_fn("obsidian", 16, 16, |x, y| {
        if rand2(x as i32, y as i32, 21) > 0.9 {
            op([0.30, 0.16, 0.46])
        } else {
            op(mul([0.07, 0.045, 0.12], grain(x, y, 22, 0.25)))
        }
    })
}

pub fn crying_obsidian() -> Texture {
    Texture::from_fn("crying_obsidian", 16, 16, |x, y| {
        if blot(x, y, 23) > 0.55 && rand2(x as i32, y as i32, 24) > 0.55 {
            op([0.68, 0.28, 1.0])
        } else if rand2(x as i32, y as i32, 25) > 0.9 {
            op([0.30, 0.16, 0.46])
        } else {
            op(mul([0.08, 0.05, 0.13], grain(x, y, 26, 0.25)))
        }
    })
}

pub fn end_stone_bricks() -> Texture {
    Texture::from_fn("end_stone_bricks", 16, 16, |x, y| {
        let mut f = grain(x, y, 27, 0.07);
        let shift = if (y / 8) % 2 == 1 { 8 } else { 0 };
        if y % 8 == 0 || (x + shift) % 16 == 0 {
            f *= 0.72;
        } else if y % 8 == 1 {
            f *= 1.08;
        }
        op(mul([0.88, 0.87, 0.63], f))
    })
}

pub fn wool(name: &str, base: C, seed: u32) -> Texture {
    Texture::from_fn(name, 16, 16, move |x, y| {
        let curl = if (x + y * 3) % 5 == 0 { 0.85 } else { 1.0 };
        op(mul(base, grain(x, y, seed, 0.08) * curl * (0.9 + 0.2 * blot(x, y, seed + 1))))
    })
}

pub fn amethyst() -> Texture {
    Texture::from_fn("amethyst", 16, 16, |x, y| {
        let (xi, yi) = (x as i32, y as i32);
        if (xi + yi) % 5 == 0 {
            op([0.86, 0.72, 1.0])
        } else if (xi - yi).rem_euclid(7) == 0 {
            op([0.40, 0.22, 0.60])
        } else {
            op(mul([0.62, 0.42, 0.86], grain(x, y, 28, 0.06)))
        }
    })
}

/// Vidrio morado de portal con remolinos.
pub fn portal() -> Texture {
    Texture::from_fn("portal", 16, 16, |x, y| {
        let fx = x as f32 / 16.0;
        let fy = y as f32 / 16.0;
        let s = (TAU * (fx * 2.0 + 0.35 * (TAU * fy * 2.0).sin())).sin() * 0.5 + 0.5;
        let c = mix([0.40, 0.08, 0.80], [0.88, 0.58, 1.0], s * s);
        // Alfa = opacidad: las vetas tapan mas, los huecos dejan ver atras
        [c[0], c[1], c[2], 0.08 + 0.6 * s * s]
    })
}

pub fn gold() -> Texture {
    Texture::from_fn("gold", 16, 16, |x, y| {
        let mut f = grain(x, y, 29, 0.04);
        if x == 0 || y == 0 {
            f *= 1.25;
        } else if x == 15 || y == 15 {
            f *= 0.72;
        }
        if x + y == 10 || x + y == 11 || x + y == 20 {
            f *= 1.3;
        }
        op(mul([0.98, 0.78, 0.22], f))
    })
}

pub fn nether_bricks() -> Texture {
    Texture::from_fn("nether_bricks", 16, 16, |x, y| {
        let mut f = grain(x, y, 30, 0.1);
        let shift = if (y / 4) % 2 == 1 { 4 } else { 0 };
        if y % 4 == 0 || (x + shift) % 8 == 0 {
            f *= 0.5;
        }
        op(mul([0.30, 0.10, 0.12], f))
    })
}

pub fn candle() -> Texture {
    Texture::from_fn("candle", 16, 16, |x, y| {
        if y < 4 {
            op([1.0, 0.85, 0.50])
        } else {
            op(mul([0.80, 0.14, 0.14], grain(x, y, 31, 0.08)))
        }
    })
}

pub fn chain() -> Texture {
    Texture::from_fn("chain", 16, 16, |x, y| {
        let link = if (y / 4) % 2 == 0 { x % 8 < 4 } else { x % 8 >= 4 };
        op(mul([0.30, 0.31, 0.36], if link { 1.2 } else { 0.7 } * grain(x, y, 32, 0.1)))
    })
}

/// Tablones de madera (casco y cubierta del barco).
pub fn planks(name: &str, base: C, seed: u32) -> Texture {
    Texture::from_fn(name, 16, 16, move |x, y| {
        let mut f = grain(x, y, seed, 0.07) * (0.92 + 0.16 * rand2(0, y as i32 / 4, seed + 1));
        if y % 4 == 0 {
            f *= 0.62;
        }
        if (x + (y / 4) * 7) % 16 == 0 {
            f *= 0.7;
        }
        op(mul(base, f))
    })
}

/// Lona de vela gastada.
pub fn sail() -> Texture {
    Texture::from_fn("sail", 16, 16, |x, y| {
        let mut f = grain(x, y, 33, 0.05) * (0.9 + 0.15 * blot(x, y, 34));
        if x % 8 == 0 {
            f *= 0.88; // costuras de la lona
        }
        op(mul([0.90, 0.86, 0.74], f))
    })
}

/// Farol: marco de hierro con la llama adentro.
pub fn lantern() -> Texture {
    Texture::from_fn("lantern", 16, 16, |x, y| {
        let frame = x < 2 || x > 13 || y < 3 || y > 13;
        if frame {
            op(mul([0.22, 0.22, 0.26], grain(x, y, 35, 0.15)))
        } else {
            let c = 1.0 - dist_center(x, y) * 0.6;
            op(mix([1.0, 0.55, 0.18], [1.0, 0.92, 0.62], c))
        }
    })
}

/// Ventana iluminada del camarote del capitan.
pub fn ship_window() -> Texture {
    Texture::from_fn("ship_window", 16, 16, |x, y| {
        let frame = x < 2 || x > 13 || y < 2 || y > 13 || x == 7 || x == 8 || y == 7 || y == 8;
        if frame {
            op(mul([0.20, 0.13, 0.08], grain(x, y, 36, 0.1)))
        } else {
            op(mul([1.0, 0.74, 0.36], grain(x, y, 37, 0.08)))
        }
    })
}

/// Hierro oscuro de los canones.
pub fn iron() -> Texture {
    Texture::from_fn("iron", 16, 16, |x, y| {
        let mut f = grain(x, y, 38, 0.1);
        if y == 0 || y == 15 {
            f *= 0.6;
        } else if y == 1 {
            f *= 1.4;
        }
        op(mul([0.17, 0.17, 0.19], f))
    })
}

/// Cuerda trenzada para la jarcia.
pub fn rope() -> Texture {
    Texture::from_fn("rope", 16, 16, |x, y| {
        let twist = if (x + y) % 4 < 2 { 1.15 } else { 0.8 };
        op(mul([0.55, 0.42, 0.26], twist * grain(x, y, 39, 0.08)))
    })
}

/// Luciernaga: punto de luz amarillo verdoso.
pub fn firefly() -> Texture {
    Texture::from_fn("firefly", 16, 16, |x, y| op(mix([0.95, 1.0, 0.45], [0.7, 1.0, 0.3], dist_center(x, y))))
}
