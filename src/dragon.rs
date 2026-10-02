// Dragon del End hecho de voxeles que vuela en circulos sobre la isla.
// Cada vez que se mueve se borran sus bloques y se vuelven a colocar.

use crate::math::Vec3;
use crate::world::{World, AIR};
use std::f32::consts::PI;

/// Velocidad del vuelo (radianes por segundo alrededor de la isla).
const SPEED: f32 = 0.22;
/// Angulo inicial: detras de la isla, visto desde la camara inicial.
const START_ANGLE: f32 = -1.05;
/// Inclinacion hacia el centro del circulo (como un avion al girar).
const BANK: f32 = 0.14;

/// Bloques con los que se arma el dragon.
#[derive(Clone, Copy)]
pub struct DragonBlocks {
    pub skin: u8,
    pub wing: u8,
    pub bone: u8,
    pub eye: u8,
}

pub struct Dragon {
    blocks: DragonBlocks,
    /// Centro del circulo de vuelo.
    center: Vec3,
    radius: f32,
    /// Celdas que ocupa ahora (para borrarlas al moverlo).
    cells: Vec<(i32, i32, i32)>,
}

/// Distancia de `p` al segmento a-b y posicion (0 a 1) del punto mas cercano.
fn segment(p: Vec3, a: Vec3, b: Vec3) -> (f32, f32) {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
    ((p - (a + ab * t)).length(), t)
}

impl Dragon {
    pub fn new(blocks: DragonBlocks, center: Vec3, radius: f32) -> Self {
        Dragon { blocks, center, radius, cells: Vec::new() }
    }

    /// Coloca el dragon en su posicion del segundo `t` del vuelo.
    pub fn place(&mut self, w: &mut World, t: f32) {
        for &(x, y, z) in &self.cells {
            w.set(x, y, z, AIR);
        }
        self.cells.clear();

        let a = START_ANGLE + t * SPEED;
        let pos = self.center + Vec3::new(a.cos() * self.radius, (2.0 * a).sin() * 1.5, a.sin() * self.radius);
        // Mira en la direccion del vuelo (tangente al circulo)
        let (sh, ch) = (a + PI / 2.0).sin_cos();
        let (sr, cr) = BANK.sin_cos();
        let flap = 0.3 + 0.7 * (t * 2.2).sin();
        let sway = 0.4 * (t * 1.1 + 1.0).sin();

        let (px, py, pz) = (pos.x as i32, pos.y as i32, pos.z as i32);
        for y in py - 9..=py + 11 {
            for z in pz - 22..=pz + 22 {
                for x in px - 22..=px + 22 {
                    if w.get(x, y, z) != AIR {
                        continue;
                    }
                    // Pasar la celda al espacio del modelo (deshacer giro e inclinacion)
                    let d = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) - pos;
                    let mx = d.x * ch + d.z * sh;
                    let mz = -d.x * sh + d.z * ch;
                    let my = d.y * cr + mz * sr;
                    let mz = -d.y * sr + mz * cr;
                    if let Some(b) = self.part(Vec3::new(mx, my, mz), flap, sway) {
                        w.set(x, y, z, b);
                        self.cells.push((x, y, z));
                    }
                }
            }
        }
    }

    /// Bloque del dragon en el punto `p` del modelo (x hacia la cabeza, y arriba).
    fn part(&self, p: Vec3, flap: f32, sway: f32) -> Option<u8> {
        let b = self.blocks;
        let (x, y, az) = (p.x, p.y, p.z.abs());
        let inside = |x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32| {
            x >= x0 && x <= x1 && y >= y0 && y <= y1 && az >= z0 && az <= z1
        };

        // Cabeza: ojos, cuernos, craneo, hocico y mandibula
        if inside(12.8, 2.3, 0.7, 14.2, 3.3, 1.7) {
            return Some(b.eye);
        }
        if inside(10.8, 3.3, 0.4, 12.4, 5.4, 1.6) {
            return Some(b.bone);
        }
        if inside(10.5, 1.2, 0.0, 14.6, 3.4, 1.7) || inside(14.6, 1.2, 0.0, 17.2, 2.6, 1.1) || inside(11.5, 0.2, 0.0, 16.6, 1.2, 1.1) {
            return Some(b.skin);
        }

        // Cuello
        let n0 = Vec3::new(4.0, 0.5, 0.0);
        let n1 = Vec3::new(8.0, 1.6, 0.0);
        let n2 = Vec3::new(11.0, 2.2, 0.0);
        if segment(p, n0, n1).0 < 1.2 || segment(p, n1, n2).0 < 1.1 {
            return Some(b.skin);
        }

        // Cuerpo, patas y puas del lomo
        if inside(-4.5, -1.6, 0.0, 4.5, 1.6, 2.2) || inside(-3.6, -3.4, 1.0, -2.2, -1.6, 2.2) || inside(1.8, -3.0, 1.0, 3.0, -1.6, 2.0) {
            return Some(b.skin);
        }
        if inside(-4.0, 1.6, 0.0, 4.0, 2.8, 0.6) && (x + 10.0).rem_euclid(2.0) < 1.0 {
            return Some(b.bone);
        }

        // Cola que se mece de lado a lado, con puas
        let tail = [
            (Vec3::new(-4.5, 0.3, 0.0), 1.1),
            (Vec3::new(-8.0, 0.0, sway), 0.95),
            (Vec3::new(-12.0, -0.6, sway * 2.5), 0.8),
            (Vec3::new(-16.0, -1.1, sway * 3.2), 0.65),
            (Vec3::new(-20.0, -1.3, sway * 2.6), 0.45),
        ];
        for s in tail.windows(2) {
            let ((a, ra), (c, rc)) = (s[0], s[1]);
            let (dist, t) = segment(p, a, c);
            let r = ra + (rc - ra) * t;
            if dist < r {
                return Some(b.skin);
            }
            let q = a + (c - a) * t;
            let above = p.y - q.y;
            let side = ((p.x - q.x).powi(2) + (p.z - q.z).powi(2)).sqrt();
            if above > r && above < r + 1.0 && side < 0.5 && (x + 30.0).rem_euclid(3.0) < 1.0 {
                return Some(b.bone);
            }
        }

        // Alas: hueso en el borde de adelante y membrana con puntas atras
        if (2.0..=18.0).contains(&az) {
            let s = (az - 2.0) / 16.0;
            let wing_y = 0.8 + flap * 4.0 * s.powf(1.3);
            let lead = 3.0 + 1.5 * (PI * s).sin() - 4.0 * s * s;
            let trail = -5.0 + 4.0 * s + 1.3 * (4.0 * PI * s).sin().abs() * (1.0 - s);
            let dy = (y - wing_y).abs();
            if x <= lead && x >= lead - 1.0 && dy <= 0.75 {
                return Some(b.skin);
            }
            if x >= trail && x <= lead && dy <= 0.6 {
                return Some(b.wing);
            }
        }
        None
    }
}
