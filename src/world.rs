// Mundo de voxeles: los bloques estan en una cuadricula 3D y el rayo la
// recorre celda por celda (DDA), asi el primer bloque que toca es el mas cercano.

use crate::material::Material;
use crate::math::Vec3;
use crate::texture::Texture;

pub const AIR: u8 = 0;

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Aabb { min, max }
    }
}

/// Forma de un bloque: cubo completo o cajas mas pequenas (losas, cadenas, faroles).
pub enum Shape {
    Full,
    Boxes(Vec<Aabb>),
}

pub struct BlockDef {
    pub name: &'static str,
    pub side: usize,
    pub top: usize,
    pub bottom: usize,
    pub shape: Shape,
    /// Lo que llena el resto de la celda (ej. agua alrededor de una cadena). AIR si nada.
    pub fill: u8,
}

impl BlockDef {
    /// Material de la cara cuya normal (hacia afuera) es `n`.
    pub fn face_material(&self, n: Vec3) -> usize {
        if n.y > 0.5 {
            self.top
        } else if n.y < -0.5 {
            self.bottom
        } else {
            self.side
        }
    }
}

/// Resultado de lanzar un rayo (como `Intersect` en la clase).
#[derive(Clone, Copy, Debug)]
pub struct Intersect {
    pub distance: f32,
    pub point: Vec3,
    /// Normal de la superficie, mirando hacia el rayo.
    pub normal: Vec3,
    /// Bloque en el que entra el rayo (AIR si sale de un liquido/vidrio).
    pub block: u8,
    /// Medio por el que viajaba el rayo (AIR, agua o vidrio).
    pub medium: u8,
    pub material: usize,
    pub cell: [i32; 3],
    /// Si golpeo la cara de un cubo completo (para la oclusion ambiental).
    pub full_face: bool,
}

pub trait RayIntersect {
    fn ray_intersect(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<Intersect>;
}

pub struct World {
    pub nx: i32,
    pub ny: i32,
    pub nz: i32,
    cells: Vec<u8>,
    pub blocks: Vec<BlockDef>,
    pub materials: Vec<Material>,
    pub textures: Vec<Texture>,
    medium_of: Vec<u8>,
    is_occluder: Vec<bool>,
}

/// Interseccion rayo-caja. Devuelve (t_cerca, t_lejos, eje de entrada).
#[inline]
pub fn ray_aabb(o: Vec3, inv: Vec3, bmin: Vec3, bmax: Vec3) -> Option<(f32, f32, usize)> {
    let mut tn = f32::NEG_INFINITY;
    let mut tf = f32::INFINITY;
    let mut ax = 0;
    for i in 0..3 {
        let t1 = (bmin.get(i) - o.get(i)) * inv.get(i);
        let t2 = (bmax.get(i) - o.get(i)) * inv.get(i);
        let (a, b) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        if a > tn {
            tn = a;
            ax = i;
        }
        if b < tf {
            tf = b;
        }
    }
    if tf < tn || tf < 0.0 {
        None
    } else {
        Some((tn, tf, ax))
    }
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

/// Coordenadas UV de un punto sobre una cara con normal `n`.
#[inline]
pub fn face_uv(p: Vec3, n: Vec3, scale: f32) -> (f32, f32) {
    let q = p / scale;
    if n.x.abs() > 0.5 {
        (fract(q.z), 1.0 - fract(q.y))
    } else if n.y.abs() > 0.5 {
        (fract(q.x), fract(q.z))
    } else {
        (fract(q.x), 1.0 - fract(q.y))
    }
}

fn axis_normal(axis: usize, sign: f32) -> Vec3 {
    match axis {
        0 => Vec3::new(sign, 0.0, 0.0),
        1 => Vec3::new(0.0, sign, 0.0),
        _ => Vec3::new(0.0, 0.0, sign),
    }
}

fn sanitize(d: Vec3) -> Vec3 {
    let f = |v: f32| if v.abs() < 1e-8 { 1e-8 } else { v };
    Vec3::new(f(d.x), f(d.y), f(d.z))
}

impl World {
    pub fn new(nx: i32, ny: i32, nz: i32) -> Self {
        World {
            nx,
            ny,
            nz,
            cells: vec![AIR; (nx * ny * nz) as usize],
            blocks: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            medium_of: Vec::new(),
            is_occluder: Vec::new(),
        }
    }

    /// Precalcula datos por bloque. Se llama al terminar la escena.
    pub fn finalize(&mut self) {
        self.medium_of = self
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| match b.shape {
                Shape::Full if self.materials[b.side].transparency > 0.0 => i as u8,
                Shape::Boxes(_) => b.fill,
                _ => AIR,
            })
            .collect();
        self.is_occluder = self
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let m = &self.materials[b.side];
                i != AIR as usize && matches!(b.shape, Shape::Full) && m.transparency == 0.0 && !m.cutout
            })
            .collect();
    }

    #[inline]
    fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && y >= 0 && z >= 0 && x < self.nx && y < self.ny && z < self.nz
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if self.in_bounds(x, y, z) {
            self.cells[((y * self.nz + z) * self.nx + x) as usize]
        } else {
            AIR
        }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, b: u8) {
        if self.in_bounds(x, y, z) {
            let i = ((y * self.nz + z) * self.nx + x) as usize;
            self.cells[i] = b;
        }
    }

    /// Llena la caja [x0, x1) x [y0, y1) x [z0, z1).
    pub fn fill(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32, b: u8) {
        for y in y0..y1 {
            for z in z0..z1 {
                for x in x0..x1 {
                    self.set(x, y, z, b);
                }
            }
        }
    }

    pub fn count_blocks(&self) -> usize {
        self.cells.iter().filter(|&&c| c != AIR).count()
    }

    pub fn material_of_block(&self, b: u8) -> &Material {
        &self.materials[self.blocks[b as usize].side]
    }

    pub fn ior_of(&self, b: u8) -> f32 {
        if b == AIR {
            1.0
        } else {
            self.material_of_block(b).ior
        }
    }

    pub fn absorption_of(&self, b: u8) -> Vec3 {
        if b == AIR {
            Vec3::ZERO
        } else {
            self.material_of_block(b).absorption
        }
    }

    /// Medio (aire, agua, vidrio) en el que se encuentra el punto `p`.
    pub fn medium_at(&self, p: Vec3) -> u8 {
        let b = self.get(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        self.medium_of[b as usize]
    }

    fn is_occluder_at(&self, c: [i32; 3]) -> bool {
        self.is_occluder[self.get(c[0], c[1], c[2]) as usize]
    }

    /// Intervalo [t0, t1] en el que el rayo esta dentro de la cuadricula.
    pub fn grid_interval(&self, o: Vec3, d: Vec3) -> Option<(f32, f32)> {
        let d = sanitize(d);
        let inv = Vec3::new(1.0 / d.x, 1.0 / d.y, 1.0 / d.z);
        let gmax = Vec3::new(self.nx as f32, self.ny as f32, self.nz as f32);
        ray_aabb(o, inv, Vec3::ZERO, gmax).map(|(a, b, _)| (a.max(0.0), b))
    }

    pub fn alpha_at(&self, mat: usize, p: Vec3, n: Vec3) -> f32 {
        let m = &self.materials[mat];
        let (u, v) = face_uv(p, n, m.tex_scale);
        self.textures[m.texture].sample(u, v)[3]
    }

    /// Oclusion ambiental: oscurece las esquinas con bloques vecinos (como Minecraft).
    pub fn ambient_occlusion(&self, cell: [i32; 3], n: Vec3, p: Vec3) -> f32 {
        let (a, s) = if n.x.abs() > 0.5 {
            (0, n.x.signum() as i32)
        } else if n.y.abs() > 0.5 {
            (1, n.y.signum() as i32)
        } else {
            (2, n.z.signum() as i32)
        };
        let (ua, va) = match a {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        let mut front = cell;
        front[a] += s;
        let occ = |du: i32, dv: i32| -> f32 {
            let mut c = front;
            c[ua] += du;
            c[va] += dv;
            if self.is_occluder_at(c) {
                1.0
            } else {
                0.0
            }
        };
        let corner = |su: i32, sv: i32| -> f32 {
            let s1 = occ(su, 0);
            let s2 = occ(0, sv);
            let c = occ(su, sv);
            if s1 + s2 >= 2.0 {
                0.0
            } else {
                (3.0 - s1 - s2 - c) / 3.0
            }
        };
        let fu = (p.get(ua) - cell[ua] as f32).clamp(0.0, 1.0);
        let fv = (p.get(va) - cell[va] as f32).clamp(0.0, 1.0);
        let a00 = corner(-1, -1);
        let a10 = corner(1, -1);
        let a01 = corner(-1, 1);
        let a11 = corner(1, 1);
        let top = a00 + (a10 - a00) * fu;
        let bot = a01 + (a11 - a01) * fu;
        let v = top + (bot - top) * fv;
        0.35 + 0.65 * v
    }

    /// Prueba el rayo contra el contenido de una celda.
    #[allow(clippy::too_many_arguments)]
    fn hit_cell(
        &self,
        b: u8,
        medium: u8,
        c: [i32; 3],
        origin: Vec3,
        dir: Vec3,
        inv: Vec3,
        t_enter: f32,
        tmax: [f32; 3],
        axis: i32,
        step: [i32; 3],
    ) -> Option<Intersect> {
        let entry_normal = if axis >= 0 {
            axis_normal(axis as usize, -step[axis as usize] as f32)
        } else {
            -dir
        };

        if b == AIR {
            // El rayo sale del agua o vidrio hacia el aire
            let mdef = &self.blocks[medium as usize];
            return Some(Intersect {
                distance: t_enter,
                point: origin + dir * t_enter,
                normal: entry_normal,
                block: AIR,
                medium,
                material: mdef.face_material(-entry_normal),
                cell: c,
                full_face: false,
            });
        }

        let def = &self.blocks[b as usize];
        match &def.shape {
            Shape::Full => {
                let mat = def.face_material(entry_normal);
                let p = origin + dir * t_enter;
                if self.materials[mat].cutout && self.alpha_at(mat, p, entry_normal) < 0.5 {
                    // Hueco en la textura: probar la cara de atras del mismo bloque
                    let ea = if tmax[0] < tmax[1] {
                        if tmax[0] < tmax[2] {
                            0
                        } else {
                            2
                        }
                    } else if tmax[1] < tmax[2] {
                        1
                    } else {
                        2
                    };
                    let t_exit = tmax[ea];
                    let n = axis_normal(ea, -step[ea] as f32);
                    let pe = origin + dir * t_exit;
                    let mat2 = def.face_material(-n);
                    if self.alpha_at(mat2, pe, n) >= 0.5 {
                        return Some(Intersect {
                            distance: t_exit,
                            point: pe,
                            normal: n,
                            block: b,
                            medium,
                            material: mat2,
                            cell: c,
                            full_face: false,
                        });
                    }
                    return None;
                }
                Some(Intersect {
                    distance: t_enter,
                    point: p,
                    normal: entry_normal,
                    block: b,
                    medium,
                    material: mat,
                    cell: c,
                    full_face: axis >= 0,
                })
            }
            Shape::Boxes(_) if def.fill != AIR && def.fill != medium => {
                // Entrar a una celda con agua desde afuera = cruzar la superficie
                let fdef = &self.blocks[def.fill as usize];
                Some(Intersect {
                    distance: t_enter,
                    point: origin + dir * t_enter,
                    normal: entry_normal,
                    block: def.fill,
                    medium,
                    material: fdef.face_material(entry_normal),
                    cell: c,
                    full_face: false,
                })
            }
            Shape::Boxes(boxes) => {
                let base = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
                let mut best: Option<Intersect> = None;
                for bx in boxes {
                    let Some((tn, _, ax)) = ray_aabb(origin, inv, base + bx.min, base + bx.max) else {
                        continue;
                    };
                    if tn < 1e-4 || best.as_ref().is_some_and(|h| tn >= h.distance) {
                        continue;
                    }
                    let n = axis_normal(ax, -dir.get(ax).signum());
                    let mat = def.face_material(n);
                    let p = origin + dir * tn;
                    if self.materials[mat].cutout && self.alpha_at(mat, p, n) < 0.5 {
                        continue;
                    }
                    best = Some(Intersect {
                        distance: tn,
                        point: p,
                        normal: n,
                        block: b,
                        medium,
                        material: mat,
                        cell: c,
                        full_face: false,
                    });
                }
                best
            }
        }
    }
}

impl RayIntersect for World {
    /// Recorre la cuadricula y devuelve la primera superficie que toca el rayo.
    fn ray_intersect(&self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<Intersect> {
        let dir = sanitize(dir);
        let inv = Vec3::new(1.0 / dir.x, 1.0 / dir.y, 1.0 / dir.z);
        let medium = self.medium_at(origin);
        let dims = [self.nx, self.ny, self.nz];
        let gmax = Vec3::new(self.nx as f32, self.ny as f32, self.nz as f32);
        let (t0, t1, entry_axis) = ray_aabb(origin, inv, Vec3::ZERO, gmax)?;
        let (mut t_enter, mut axis) = if t0 > 0.0 { (t0, entry_axis as i32) } else { (0.0, -1) };
        let t_end = t1.min(max_t);
        if t_enter > t_end {
            return None;
        }

        let p = origin + dir * t_enter;
        let step = [
            if dir.x > 0.0 { 1 } else { -1 },
            if dir.y > 0.0 { 1 } else { -1 },
            if dir.z > 0.0 { 1 } else { -1 },
        ];
        let mut c = [
            (p.x.floor() as i32).clamp(0, self.nx - 1),
            (p.y.floor() as i32).clamp(0, self.ny - 1),
            (p.z.floor() as i32).clamp(0, self.nz - 1),
        ];
        let tdelta = [inv.x.abs(), inv.y.abs(), inv.z.abs()];
        let mut tmax = [0.0f32; 3];
        for i in 0..3 {
            let o = origin.get(i);
            tmax[i] = if step[i] > 0 {
                (c[i] as f32 + 1.0 - o) * inv.get(i)
            } else {
                (c[i] as f32 - o) * inv.get(i)
            };
        }

        loop {
            let b = self.get(c[0], c[1], c[2]);
            if b != medium {
                if let Some(h) = self.hit_cell(b, medium, c, origin, dir, inv, t_enter, tmax, axis, step) {
                    return if h.distance <= max_t { Some(h) } else { None };
                }
            }
            let a = if tmax[0] < tmax[1] {
                if tmax[0] < tmax[2] {
                    0
                } else {
                    2
                }
            } else if tmax[1] < tmax[2] {
                1
            } else {
                2
            };
            t_enter = tmax[a];
            if t_enter > t_end {
                return None;
            }
            c[a] += step[a];
            if c[a] < 0 || c[a] >= dims[a] {
                return None;
            }
            tmax[a] += tdelta[a];
            axis = a as i32;
        }
    }
}
