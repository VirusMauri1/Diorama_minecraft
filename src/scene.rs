// Construccion del diorama: isla flotante de cerezos de noche, con un lago,
// un barco pirata, un muelle con faroles y una espada con hoja de portal.
// Coordenadas: 1 unidad = 1 bloque, con y hacia arriba.

use crate::material::Material;
use crate::math::{smoothstep, Vec3};
use crate::noise::{rand3, vnoise};
use crate::render::{Light, Scene};
use crate::skybox::{Skybox, MOON_DIR};
use crate::texture::Texture;
use crate::textures as tx;
use crate::world::{Aabb, BlockDef, Shape, World, AIR};
use std::f32::consts::PI;
use std::path::Path;

pub const NX: i32 = 96;
pub const NY: i32 = 92;
pub const NZ: i32 = 80;

/// Altura de la superficie del agua (las celdas con y < WATER son agua).
const WATER: i32 = 44;

struct Lib {
    textures: Vec<Texture>,
    materials: Vec<Material>,
    blocks: Vec<BlockDef>,
}

impl Lib {
    fn tex(&mut self, mut t: Texture) -> usize {
        t.try_override(Path::new("assets/textures"));
        self.textures.push(t);
        self.textures.len() - 1
    }

    fn mat(&mut self, m: Material) -> usize {
        self.materials.push(m);
        self.materials.len() - 1
    }

    fn block(&mut self, name: &'static str, side: usize, top: usize, bottom: usize, shape: Shape, fill: u8) -> u8 {
        self.blocks.push(BlockDef { name, side, top, bottom, shape, fill });
        (self.blocks.len() - 1) as u8
    }

    fn full(&mut self, name: &'static str, mat: usize) -> u8 {
        self.block(name, mat, mat, mat, Shape::Full, AIR)
    }
}

struct Ids {
    grass: u8,
    grass_petals: u8,
    dirt: u8,
    stone: u8,
    coal: u8,
    diamond: u8,
    sand: u8,
    water: u8,
    leaves: u8,
    cherry_log: u8,
    planks_slab: u8,
    spruce_log: u8,
    blackstone: u8,
    obsidian: u8,
    crying: u8,
    end_stone: u8,
    black_wool: u8,
    amethyst: u8,
    portal: u8,
    gold: u8,
    nether: u8,
    candle: u8,
    chain: u8,
    chain_wet: u8,
    dark_oak: u8,
    spruce_planks: u8,
    ochre_planks: u8,
    sail: u8,
    lantern: u8,
    window: u8,
    cannon: u8,
    rope: u8,
    firefly: u8,
}

fn v3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn build_library() -> (Lib, Ids) {
    let mut lib = Lib { textures: Vec::new(), materials: Vec::new(), blocks: Vec::new() };

    // Texturas
    let t_grass_top = lib.tex(tx::grass_top());
    let t_grass_side = lib.tex(tx::grass_side());
    let t_petals = lib.tex(tx::grass_petals());
    let t_dirt = lib.tex(tx::dirt());
    let t_stone = lib.tex(tx::stone());
    let t_coal = lib.tex(tx::ore("coal_ore", [0.12, 0.12, 0.13], 50));
    let t_diamond = lib.tex(tx::ore("diamond_ore", [0.45, 0.95, 0.92], 60));
    let t_sand = lib.tex(tx::sand());
    let t_water = lib.tex(tx::water());
    let t_leaves = lib.tex(tx::cherry_leaves());
    let t_cherry_bark = lib.tex(tx::bark("cherry_log", [0.26, 0.16, 0.19], 70));
    let t_cherry_top = lib.tex(tx::log_top("cherry_log_top", [0.26, 0.16, 0.19], [0.82, 0.55, 0.55], 71));
    let t_planks = lib.tex(tx::cherry_planks());
    let t_spruce_bark = lib.tex(tx::bark("spruce_log", [0.30, 0.21, 0.13], 72));
    let t_spruce_top = lib.tex(tx::log_top("spruce_log_top", [0.30, 0.21, 0.13], [0.55, 0.40, 0.25], 73));
    let t_blackstone = lib.tex(tx::blackstone());
    let t_obsidian = lib.tex(tx::obsidian());
    let t_crying = lib.tex(tx::crying_obsidian());
    let t_end = lib.tex(tx::end_stone_bricks());
    let t_black = lib.tex(tx::wool("black_wool", [0.11, 0.10, 0.13], 82));
    let t_amethyst = lib.tex(tx::amethyst());
    let t_portal = lib.tex(tx::portal());
    let t_gold = lib.tex(tx::gold());
    let t_nether = lib.tex(tx::nether_bricks());
    let t_candle = lib.tex(tx::candle());
    let t_chain = lib.tex(tx::chain());
    let t_dark_oak = lib.tex(tx::planks("dark_oak_planks", [0.30, 0.20, 0.12], 90));
    let t_spruce_planks = lib.tex(tx::planks("spruce_planks", [0.50, 0.36, 0.21], 92));
    let t_ochre = lib.tex(tx::planks("ochre_planks", [0.80, 0.60, 0.26], 94));
    let t_sail = lib.tex(tx::sail());
    let t_lantern = lib.tex(tx::lantern());
    let t_window = lib.tex(tx::ship_window());
    let t_iron = lib.tex(tx::iron());
    let t_rope = lib.tex(tx::rope());
    let t_firefly = lib.tex(tx::firefly());

    // Materiales: textura propia + albedo, especular, reflectividad, transparencia
    let m_grass_top = lib.mat(Material::new("pasto", t_grass_top).albedo(0.9).specular(0.05, 8.0));
    let m_grass_side = lib.mat(Material::new("pasto (lado)", t_grass_side).albedo(0.9).specular(0.05, 8.0));
    let m_petals = lib.mat(Material::new("pasto con petalos", t_petals).albedo(0.9).specular(0.05, 8.0));
    let m_dirt = lib.mat(Material::new("tierra", t_dirt).albedo(0.9).specular(0.02, 4.0));
    let m_stone = lib.mat(Material::new("piedra", t_stone).albedo(0.85).specular(0.1, 12.0));
    let m_coal = lib.mat(Material::new("mena de carbon", t_coal).albedo(0.85).specular(0.15, 16.0));
    let m_diamond = lib.mat(Material::new("mena de diamante", t_diamond).albedo(0.85).specular(0.7, 90.0).reflective(0.1));
    let m_sand = lib.mat(Material::new("arena", t_sand).albedo(0.9).specular(0.05, 8.0));
    let m_water = lib.mat(
        Material::new("agua", t_water)
            .scale(2.0)
            .albedo(0.5)
            .specular(1.0, 150.0)
            .reflective(0.05)
            .transparent(0.72, 1.33, v3(0.16, 0.06, 0.025))
            .ripple(),
    );
    let m_leaves = lib.mat(Material::new("hojas de cerezo", t_leaves).albedo(0.88).specular(0.05, 8.0).cutout());
    let m_cherry_bark = lib.mat(Material::new("tronco de cerezo", t_cherry_bark).albedo(0.85).specular(0.05, 8.0));
    let m_cherry_top = lib.mat(Material::new("tronco de cerezo (anillos)", t_cherry_top).albedo(0.85).specular(0.05, 8.0));
    let m_planks = lib.mat(Material::new("tablones de cerezo", t_planks).albedo(0.85).specular(0.15, 16.0));
    let m_spruce_bark = lib.mat(Material::new("tronco de abeto", t_spruce_bark).albedo(0.85).specular(0.05, 8.0));
    let m_spruce_top = lib.mat(Material::new("tronco de abeto (anillos)", t_spruce_top).albedo(0.85).specular(0.05, 8.0));
    let m_blackstone = lib.mat(Material::new("piedra negra", t_blackstone).albedo(0.8).specular(0.35, 30.0).reflective(0.04));
    let m_obsidian = lib.mat(Material::new("obsidiana", t_obsidian).albedo(0.6).specular(0.6, 120.0).reflective(0.12));
    let m_crying = lib.mat(Material::new("obsidiana llorona", t_crying).albedo(0.6).specular(0.9, 120.0).reflective(0.2).emissive(0.5));
    let m_end = lib.mat(Material::new("ladrillo de end", t_end).albedo(0.85).specular(0.1, 12.0));
    let m_black = lib.mat(Material::new("lana negra", t_black).albedo(0.9).specular(0.02, 4.0));
    let m_amethyst = lib.mat(Material::new("amatista", t_amethyst).albedo(0.7).specular(0.8, 80.0).reflective(0.1).emissive(0.35));
    let m_portal = lib.mat(
        Material::new("vidrio de portal", t_portal)
            .albedo(0.15)
            .specular(1.0, 200.0)
            .reflective(0.03)
            .transparent(0.55, 1.05, v3(0.05, 0.16, 0.02))
            .emissive(0.5),
    );
    let m_gold = lib.mat(Material::new("bloque de oro", t_gold).albedo(0.6).specular(1.0, 100.0).reflective(0.4));
    let m_nether = lib.mat(Material::new("ladrillo del nether", t_nether).albedo(0.85).specular(0.1, 12.0));
    let m_candle = lib.mat(Material::new("vela", t_candle).albedo(0.6).emissive(1.6));
    let m_chain = lib.mat(Material::new("cadena", t_chain).albedo(0.6).specular(0.6, 60.0).reflective(0.1));
    let m_dark_oak = lib.mat(Material::new("tablones de roble oscuro", t_dark_oak).albedo(0.85).specular(0.12, 14.0));
    let m_spruce_planks = lib.mat(Material::new("tablones de abeto", t_spruce_planks).albedo(0.85).specular(0.12, 14.0));
    let m_ochre = lib.mat(Material::new("tablones pintados", t_ochre).albedo(0.85).specular(0.2, 20.0));
    let m_sail = lib.mat(Material::new("vela de lona", t_sail).albedo(0.9).specular(0.02, 4.0));
    let m_lantern = lib.mat(Material::new("farol", t_lantern).albedo(0.6).specular(0.5, 40.0).emissive(2.4));
    let m_window = lib.mat(Material::new("ventana iluminada", t_window).albedo(0.6).specular(0.6, 80.0).emissive(1.5));
    let m_iron = lib.mat(Material::new("hierro (canon)", t_iron).albedo(0.6).specular(0.7, 60.0).reflective(0.08));
    let m_rope = lib.mat(Material::new("cuerda", t_rope).albedo(0.85).specular(0.02, 4.0));
    let m_firefly = lib.mat(Material::new("luciernaga", t_firefly).albedo(0.2).emissive(5.0));

    let air = lib.full("aire", m_stone);
    assert_eq!(air, AIR);

    let slab = Aabb::new(v3(0.0, 0.0, 0.0), v3(1.0, 0.5, 1.0));
    let chain_box = Aabb::new(v3(0.44, 0.0, 0.44), v3(0.56, 1.0, 0.56));
    let link_box = Aabb::new(v3(0.38, 0.38, 0.38), v3(0.62, 0.62, 0.62));
    let candle_box = Aabb::new(v3(0.40, 0.0, 0.40), v3(0.60, 0.55, 0.60));
    let lantern_box = Aabb::new(v3(0.30, 0.0, 0.30), v3(0.70, 0.55, 0.70));
    let cannon_box = Aabb::new(v3(0.28, 0.22, 0.0), v3(0.72, 0.66, 1.0));
    let firefly_box = Aabb::new(v3(0.44, 0.44, 0.44), v3(0.56, 0.56, 0.56));

    let water = lib.full("agua", m_water);
    let ids = Ids {
        grass: lib.block("pasto", m_grass_side, m_grass_top, m_dirt, Shape::Full, AIR),
        grass_petals: lib.block("pasto con petalos", m_grass_side, m_petals, m_dirt, Shape::Full, AIR),
        dirt: lib.full("tierra", m_dirt),
        stone: lib.full("piedra", m_stone),
        coal: lib.full("mena de carbon", m_coal),
        diamond: lib.full("mena de diamante", m_diamond),
        sand: lib.full("arena", m_sand),
        water,
        leaves: lib.full("hojas de cerezo", m_leaves),
        cherry_log: lib.block("tronco de cerezo", m_cherry_bark, m_cherry_top, m_cherry_top, Shape::Full, AIR),
        planks_slab: lib.block("losa de cerezo", m_planks, m_planks, m_planks, Shape::Boxes(vec![slab]), AIR),
        spruce_log: lib.block("tronco de abeto", m_spruce_bark, m_spruce_top, m_spruce_top, Shape::Full, AIR),
        blackstone: lib.full("piedra negra", m_blackstone),
        obsidian: lib.full("obsidiana", m_obsidian),
        crying: lib.full("obsidiana llorona", m_crying),
        end_stone: lib.full("ladrillo de end", m_end),
        black_wool: lib.full("lana negra", m_black),
        amethyst: lib.full("amatista", m_amethyst),
        portal: lib.full("vidrio de portal", m_portal),
        gold: lib.full("bloque de oro", m_gold),
        nether: lib.full("ladrillo del nether", m_nether),
        candle: lib.block("vela", m_candle, m_candle, m_candle, Shape::Boxes(vec![candle_box]), AIR),
        chain: lib.block("cadena", m_chain, m_chain, m_chain, Shape::Boxes(vec![chain_box]), AIR),
        chain_wet: lib.block("cadena sumergida", m_chain, m_chain, m_chain, Shape::Boxes(vec![chain_box]), water),
        dark_oak: lib.full("tablones de roble oscuro", m_dark_oak),
        spruce_planks: lib.full("tablones de abeto", m_spruce_planks),
        ochre_planks: lib.full("tablones pintados", m_ochre),
        sail: lib.full("vela de lona", m_sail),
        lantern: lib.block("farol", m_lantern, m_lantern, m_lantern, Shape::Boxes(vec![lantern_box]), AIR),
        window: lib.full("ventana iluminada", m_window),
        cannon: lib.block("canon", m_iron, m_iron, m_iron, Shape::Boxes(vec![cannon_box]), AIR),
        rope: lib.block("cuerda", m_rope, m_rope, m_rope, Shape::Boxes(vec![link_box]), AIR),
        firefly: lib.block("luciernaga", m_firefly, m_firefly, m_firefly, Shape::Boxes(vec![firefly_box]), AIR),
    };
    (lib, ids)
}

// ---------------------------------------------------------------- terreno

const ISLAND_C: (f32, f32) = (48.0, 40.0);
const ISLAND_R: (f32, f32) = (46.0, 38.0);
const LAKE_C: (f32, f32) = (44.0, 46.0);
const LAKE_R: (f32, f32) = (30.0, 18.0);

const SWORD_X: i32 = 76;
const SWORD_Z: i32 = 22;

/// Islotes flotantes pequenos alrededor de la isla: (x, z, radio, altura).
const ISLETS: [(f32, f32, f32, i32); 2] = [(7.0, 72.0, 4.5, WATER + 7), (89.0, 72.0, 4.0, WATER - 4)];

/// Distancia al centro de la isla, normalizada (< 1 = dentro de la isla).
fn island_edge(fx: f32, fz: f32) -> f32 {
    let dx = (fx - ISLAND_C.0) / ISLAND_R.0;
    let dz = (fz - ISLAND_C.1) / ISLAND_R.1;
    (dx * dx + dz * dz).sqrt() + vnoise(fx * 0.08, fz * 0.08, 11) * 0.14 + vnoise(fx * 0.3, fz * 0.3, 12) * 0.04
}

/// Distancia al borde del lago (negativo = dentro del agua).
fn lake_field(x: f32, z: f32) -> f32 {
    let dx = (x - LAKE_C.0) / LAKE_R.0;
    let dz = (z - LAKE_C.1) / LAKE_R.1;
    ((dx * dx + dz * dz).sqrt() - 1.0) * LAKE_R.1 + (vnoise(x * 0.12, z * 0.12, 7) - 0.5) * 4.0
}

fn gauss(x: f32, z: f32, cx: f32, cz: f32, s: f32) -> f32 {
    (-((x - cx).powi(2) + (z - cz).powi(2)) / s).exp()
}

fn sword_dist(fx: f32, fz: f32) -> f32 {
    ((fx - SWORD_X as f32 - 0.5).powi(2) + (fz - SWORD_Z as f32 - 0.5).powi(2)).sqrt()
}

/// Altura del bloque superior del terreno de la isla principal en (x, z).
fn ground_height(x: i32, z: i32) -> i32 {
    let (fx, fz) = (x as f32 + 0.5, z as f32 + 0.5);
    let f = lake_field(fx, fz);
    if f < 0.0 {
        let depth = (2.0 + -f * 0.5).min(9.0);
        return WATER - 1 - depth.round() as i32;
    }
    if f < 1.2 {
        return WATER - 1;
    }
    // Meseta plana para el monumento
    if sword_dist(fx, fz) < 9.0 {
        return WATER + 3;
    }
    let shore = ((f - 1.2) / 6.0).clamp(0.0, 1.0);
    let hills = 9.0 * gauss(fx, fz, 18.0, 16.0, 260.0)
        + 6.0 * gauss(fx, fz, 36.0, 9.0, 160.0)
        + 3.0 * gauss(fx, fz, SWORD_X as f32, SWORD_Z as f32, 260.0)
        + 4.0 * gauss(fx, fz, 60.0, 8.0, 120.0)
        + 2.5 * (vnoise(fx * 0.09, fz * 0.09, 3) - 0.3);
    // El borde de la isla baja un poco (orilla redondeada)
    let rim = smoothstep(0.85, 1.0, island_edge(fx, fz)) * 2.0;
    (WATER as f32 + shore * hills.max(0.0) - rim).round() as i32
}

/// Fondo de la isla principal: un cono invertido de roca con estalactitas.
fn island_bottom(x: i32, z: i32, edge: f32, top: i32) -> i32 {
    let (fx, fz) = (x as f32 + 0.5, z as f32 + 0.5);
    let k = (1.0 - edge).clamp(0.0, 1.0);
    let n = vnoise(fx * 0.11, fz * 0.11, 21);
    let mut depth = 3.0 + 26.0 * k.powf(0.65) * (0.7 + 0.5 * n);
    // Estalactitas de 2x2 bloques colgando
    let (sx, sz) = (x.div_euclid(2), z.div_euclid(2));
    if rand3(sx, 31, sz) > 0.8 {
        depth += rand3(sx, 32, sz) * 8.0 * k.sqrt();
    }
    let b = (WATER as f32 - 1.0 - depth).round() as i32;
    b.min(top - 4).max(1)
}

/// Bloque de arriba y de abajo de la columna (x, z), o None si ahi no hay tierra.
fn column(x: i32, z: i32) -> Option<(i32, i32)> {
    let (fx, fz) = (x as f32 + 0.5, z as f32 + 0.5);
    let mut e = island_edge(fx, fz);
    if sword_dist(fx, fz) < 9.5 {
        e = e.min(0.9); // el pedestal nunca queda colgando sobre el vacio
    }
    if e < 1.0 {
        let top = ground_height(x, z);
        return Some((top, island_bottom(x, z, e, top)));
    }
    for (i, &(cx, cz, r, top)) in ISLETS.iter().enumerate() {
        let d = ((fx - cx).powi(2) + (fz - cz).powi(2)).sqrt() / r + (vnoise(fx * 0.4, fz * 0.4, 50 + i as u32) - 0.5) * 0.3;
        if d < 1.0 {
            let spike = rand3(x, 33, z) * 2.0;
            let depth = 2.0 + (1.0 - d).powf(0.7) * 8.0 + spike;
            return Some((top, top - depth.round() as i32));
        }
    }
    None
}

fn surface(x: i32, z: i32) -> Option<i32> {
    column(x, z).map(|c| c.0)
}

fn is_beach(x: i32, z: i32) -> bool {
    let f = lake_field(x as f32 + 0.5, z as f32 + 0.5);
    (0.0..2.6).contains(&f)
}

fn build_terrain(w: &mut World, ids: &Ids) {
    for z in 0..NZ {
        for x in 0..NX {
            let Some((g, bottom)) = column(x, z) else {
                continue;
            };
            let on_island = island_edge(x as f32 + 0.5, z as f32 + 0.5) < 1.0 || sword_dist(x as f32 + 0.5, z as f32 + 0.5) < 9.5;
            let in_lake = on_island && lake_field(x as f32 + 0.5, z as f32 + 0.5) < 0.0;
            let sandy = in_lake || (on_island && is_beach(x, z));
            // Capa de tierra irregular bajo el pasto
            let soil = 3 + (vnoise(x as f32 * 0.2, z as f32 * 0.2, 13) * 3.0) as i32;
            for y in bottom..=g {
                let r = rand3(x, y, z);
                let b = if y == g {
                    if sandy {
                        ids.sand
                    } else if rand3(x, 99, z) > 0.88 {
                        ids.grass_petals
                    } else {
                        ids.grass
                    }
                } else if y >= g - 3 && sandy {
                    ids.sand
                } else if y >= g - soil {
                    ids.dirt
                } else if y == bottom && rand3(x, 98, z) > 0.96 {
                    // Cristales que brillan en la punta de las estalactitas
                    ids.amethyst
                } else if r > 0.985 && y < WATER - 14 {
                    ids.diamond
                } else if r > 0.95 {
                    ids.coal
                } else {
                    ids.stone
                };
                w.set(x, y, z, b);
            }
            if in_lake {
                for y in g + 1..WATER {
                    w.set(x, y, z, ids.water);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- cerezos

fn build_tree(w: &mut World, ids: &Ids, x: i32, z: i32, base: i32, seed: i32) {
    let height = 4 + (rand3(x, seed, z) * 3.0) as i32;
    let lean = if rand3(x, seed + 1, z) > 0.5 { 1 } else { -1 };
    for i in 0..height {
        let dx = if i > height / 2 { lean } else { 0 };
        w.set(x + dx, base + i, z, ids.cherry_log);
    }
    let top = base + height;
    let cx = x + lean;
    let r = 3.5 + rand3(x, seed + 2, z) * 1.8;
    let ry = 2.3;
    let ir = r.ceil() as i32 + 1;
    for dy in -3..=3 {
        for dz in -ir..=ir {
            for dx in -ir..=ir {
                let (fx, fy, fz) = (dx as f32, dy as f32, dz as f32);
                let d = (fx / r).powi(2) + (fy / ry).powi(2) + (fz / r).powi(2);
                let jitter = rand3(cx + dx, top + dy, z + dz) * 0.35;
                let (px, py, pz) = (cx + dx, top + dy, z + dz);
                if d <= 1.0 - jitter * 0.6 && w.get(px, py, pz) == AIR {
                    w.set(px, py, pz, ids.leaves);
                }
            }
        }
    }
    // Hojas colgantes en el borde de la copa
    for dz in -ir..=ir {
        for dx in -ir..=ir {
            let d = ((dx * dx + dz * dz) as f32).sqrt();
            if d > r - 1.5 && d < r + 0.3 && rand3(cx + dx, seed + 3, z + dz) > 0.6 {
                let len = 1 + (rand3(cx + dx, seed + 4, z + dz) * 2.0) as i32;
                for k in 1..=len {
                    let (px, py, pz) = (cx + dx, top - 1 - k, z + dz);
                    if w.get(px, py, pz) == AIR {
                        w.set(px, py, pz, ids.leaves);
                    }
                }
            }
        }
    }
    // Petalos en el pasto bajo el arbol
    for dz in -6..=6 {
        for dx in -6..=6 {
            let (px, pz) = (x + dx, z + dz);
            if dx * dx + dz * dz > 36 {
                continue;
            }
            let Some(g) = surface(px, pz) else {
                continue;
            };
            if w.get(px, g, pz) == ids.grass && rand3(px, seed + 5, pz) > 0.45 {
                w.set(px, g, pz, ids.grass_petals);
            }
        }
    }
}

fn build_trees(w: &mut World, ids: &Ids) {
    let mut n = 0;
    for gz in 0..12 {
        for gx in 0..14 {
            let x = gx * 7 + 3 + ((rand3(gx, 1, gz) - 0.5) * 4.0) as i32;
            let z = gz * 7 + 3 + ((rand3(gx, 2, gz) - 0.5) * 4.0) as i32;
            if rand3(gx, 3, gz) > 0.8 {
                continue;
            }
            let (fx, fz) = (x as f32 + 0.5, z as f32 + 0.5);
            // Solo sobre la isla, lejos del borde, del lago y del monumento
            if island_edge(fx, fz) > 0.86 || lake_field(fx, fz) < 4.0 || sword_dist(fx, fz) < 13.0 {
                continue;
            }
            let near_dock = (56..72).contains(&x) && (56..74).contains(&z);
            // Dejar libre la vista al lago desde la camara inicial
            let (vx, vz) = (fx - LAKE_C.0, fz - LAKE_C.1);
            let along = -0.75 * vx + 0.66 * vz;
            let lateral = (0.66 * vx + 0.75 * vz).abs();
            let view_corridor = along > 0.0 && lateral < 20.0;
            if near_dock || view_corridor {
                continue;
            }
            build_tree(w, ids, x, z, ground_height(x, z) + 1, n * 17 + 5);
            n += 1;
        }
    }
    // Un cerezo pequeno en el islote mas alto
    let (ix, iz, _, itop) = ISLETS[0];
    build_tree(w, ids, ix as i32, iz as i32, itop + 1, 999);
}

// ---------------------------------------------------------------- faroles y luciernagas

/// Coloca un farol y guarda la posicion de su luz (justo encima del farol).
fn lantern(w: &mut World, ids: &Ids, x: i32, y: i32, z: i32, lights: &mut Vec<Vec3>) {
    w.set(x, y, z, ids.lantern);
    lights.push(v3(x as f32 + 0.5, y as f32 + 0.8, z as f32 + 0.5));
}

/// Poste de abeto de 2 bloques con un farol encima.
fn lantern_post(w: &mut World, ids: &Ids, x: i32, z: i32, lights: &mut Vec<Vec3>) {
    let Some(g) = surface(x, z) else {
        return;
    };
    w.set(x, g + 1, z, ids.spruce_log);
    w.set(x, g + 2, z, ids.spruce_log);
    lantern(w, ids, x, g + 3, z, lights);
}

fn build_fireflies(w: &mut World, ids: &Ids) {
    let mut placed = 0;
    for i in 0..600 {
        if placed >= 70 {
            break;
        }
        let x = (rand3(i, 61, 0) * NX as f32) as i32;
        let z = (rand3(i, 62, 0) * NZ as f32) as i32;
        let Some(g) = surface(x, z) else {
            continue;
        };
        let y = g.max(WATER - 1) + 2 + (rand3(i, 63, 0) * 6.0) as i32;
        if w.get(x, y, z) == AIR {
            w.set(x, y, z, ids.firefly);
            placed += 1;
        }
    }
}

// ---------------------------------------------------------------- barco pirata

const BOW: i32 = 21;
const STERN: i32 = 59;
/// Altura de la cubierta (3 bloques sobre el agua).
const DECK: i32 = WATER + 3;
/// Eje del barco (centro de las celdas z = MID_Z).
const SHIP_Z: f32 = 46.5;
const MID_Z: i32 = 46;
/// Castillo de proa (x <= FORE_END) y alcazar de popa (x >= QUARTER).
const FORE_END: i32 = BOW + 6;
const QUARTER: i32 = STERN - 9;
const FORE_X: i32 = BOW + 11;
const MAIN_X: i32 = BOW + 21;
const MIZZEN_X: i32 = STERN - 5;

fn ship_t(x: i32) -> f32 {
    (x - BOW) as f32 / (STERN - BOW) as f32
}

fn hull_half_width(t: f32) -> f32 {
    if t < 0.3 {
        5.5 * (1.0 - (1.0 - t / 0.3).powi(2)).max(0.12)
    } else if t < 0.86 {
        5.5
    } else {
        5.5 - (t - 0.86) * 9.0
    }
}

/// Fondo del casco: bajo el agua en el centro, sube hacia la proa y la popa.
fn hull_bottom(t: f32) -> f32 {
    WATER as f32 - 3.0 + 4.5 * ((0.22 - t) / 0.22).max(0.0).powf(1.6) + 2.0 * ((t - 0.85) / 0.15).max(0.0).powi(2)
}

/// Medio ancho del casco a la altura y (se estrecha hacia la quilla).
fn hull_width_at(t: f32, y: i32) -> f32 {
    let yb = hull_bottom(t);
    let h = ((y as f32 - yb) / (DECK as f32 - yb)).clamp(0.0, 1.0);
    hull_half_width(t) * (0.45 + 0.55 * h.sqrt())
}

/// Bloque mas exterior del casco en la fila (x, y) hacia el lado `side`.
fn outer_z(w: &World, x: i32, y: i32, side: i32) -> i32 {
    let mut z = MID_Z + side * 8;
    while w.get(x, y, z) == AIR && z != MID_Z {
        z -= side;
    }
    z
}

/// Cuerda recta entre dos puntos (solo en celdas vacias).
fn rope_line(w: &mut World, ids: &Ids, a: Vec3, b: Vec3) {
    let n = ((b - a).length() * 3.0).ceil() as i32;
    for i in 0..=n {
        let p = a + (b - a) * (i as f32 / n as f32);
        let (x, y, z) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        if w.get(x, y, z) == AIR {
            w.set(x, y, z, ids.rope);
        }
    }
}

/// Calavera con huesos cruzados para la vela mayor (9 x 11).
const SKULL: [&str; 11] = [
    "..XXXXX..",
    ".XXXXXXX.",
    ".X..X..X.",
    ".XXXXXXX.",
    "..XX.XX..",
    "..X.X.X..",
    "X.......X",
    ".XX...XX.",
    "...XXX...",
    ".XX...XX.",
    "X.......X",
];

/// Bandera pirata (negra con calavera blanca).
const FLAG: [&str; 6] = ["#########", "###...###", "##.#.#.##", "###...###", "##.#.#.##", "#########"];

/// Vela cuadrada colgada de una verga, inflada hacia la proa.
fn build_sail(w: &mut World, ids: &Ids, mx: i32, yard_y: i32, half: i32, height: i32, skull: bool, seed: i32) {
    let bulge = |i: i32, dzi: i32| -> Option<i32> {
        if i < 0 || i >= height || dzi.abs() > half {
            return None;
        }
        let v = (i as f32 + 0.5) / height as f32;
        let u = dzi as f32 / (half as f32 + 1.0);
        Some((2.4 * (PI * v).sin() * (1.0 - u * u)).round() as i32)
    };
    for i in 0..height {
        let y = yard_y - 1 - i;
        for dzi in -half..=half {
            let z = MID_Z + dzi;
            let pixel = skull
                && (1..=SKULL.len() as i32).contains(&i)
                && dzi.abs() <= 4
                && SKULL[(i - 1) as usize].as_bytes()[(dzi + 4) as usize] == b'X';
            // Borde inferior rasgado y algunos agujeros de bala
            let edge = dzi.abs() == half;
            let torn = (i == height - 1 && rand3(z, y, seed) > 0.55) || (i > 0 && rand3(z, y, seed + 1) > 0.985);
            if torn && !edge && !pixel {
                continue;
            }
            let b = bulge(i, dzi).unwrap_or(0);
            // Rellenar hasta el vecino para que la tela no tenga huecos
            let lo = [bulge(i - 1, dzi), bulge(i + 1, dzi), bulge(i, dzi - 1), bulge(i, dzi + 1)]
                .into_iter()
                .flatten()
                .fold(b, i32::min);
            let blk = if pixel { ids.black_wool } else { ids.sail };
            for k in lo..=b {
                w.set(mx - 1 - k, y, z, blk);
            }
        }
    }
    // Verga
    for z in MID_Z - half - 1..=MID_Z + half + 1 {
        w.set(mx, yard_y, z, ids.spruce_log);
    }
}

fn in_triangle(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let s = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| (p.0 - r.0) * (q.1 - r.1) - (q.0 - r.0) * (p.1 - r.1);
    let (d1, d2, d3) = (s(p, a, b), s(p, b, c), s(p, c, a));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

fn build_ship(w: &mut World, ids: &Ids, lights: &mut Vec<Vec3>) {
    // Casco (se construye despues del lago para que desplace el agua)
    for x in BOW..=STERN {
        let t = ship_t(x);
        let top = if x <= FORE_END {
            DECK + 2
        } else if x >= QUARTER {
            DECK + 3
        } else {
            DECK
        };
        for y in hull_bottom(t).ceil() as i32..=top {
            let hw = hull_width_at(t, y);
            for z in MID_Z - 7..=MID_Z + 7 {
                let dz = (z as f32 + 0.5 - SHIP_Z).abs();
                if dz > hw {
                    continue;
                }
                let outer = dz > hw - 1.0 || x == STERN;
                let b = if y == top && !outer {
                    ids.spruce_planks
                } else if y == WATER + 1 && outer {
                    ids.ochre_planks
                } else {
                    ids.dark_oak
                };
                w.set(x, y, z, b);
            }
        }
        // Borda
        let hw = hull_half_width(t);
        for z in MID_Z - 7..=MID_Z + 7 {
            let dz = (z as f32 + 0.5 - SHIP_Z).abs();
            if dz <= hw && (dz > hw - 1.0 || x == STERN) {
                w.set(x, top + 1, z, ids.dark_oak);
            }
        }
    }

    // Portas y canones a ambos costados
    for x in (BOW + 9..QUARTER - 1).step_by(4) {
        for side in [-1, 1] {
            let y = WATER + 1;
            let z = outer_z(w, x, y, side);
            w.set(x, y, z, ids.blackstone);
            w.set(x, y, z + side, ids.cannon);
        }
    }

    // Ventanas iluminadas del camarote
    for x in (QUARTER + 1..STERN).step_by(2) {
        for side in [-1, 1] {
            let z = outer_z(w, x, DECK + 2, side);
            w.set(x, DECK + 2, z, ids.window);
        }
    }
    for z in [MID_Z - 3, MID_Z - 1, MID_Z + 1, MID_Z + 3] {
        w.set(STERN, DECK + 1, z, ids.window);
        w.set(STERN, DECK + 2, z, ids.window);
        w.set(STERN, DECK - 1, z, ids.window);
    }
    w.set(QUARTER, DECK + 1, MID_Z, ids.spruce_planks);
    w.set(QUARTER, DECK + 2, MID_Z, ids.spruce_planks);
    w.set(QUARTER, DECK + 2, MID_Z - 3, ids.window);
    w.set(QUARTER, DECK + 2, MID_Z + 3, ids.window);

    // Baupres y foque
    let tip = (BOW - 8, DECK + 6);
    for i in 0..=9 {
        let y = DECK + 2 + (i as f32 * 0.45).round() as i32;
        w.set(BOW + 1 - i, y, MID_Z, ids.spruce_log);
    }
    let (a, b, c) = ((BOW as f32 - 5.0, DECK as f32 + 8.0), (FORE_X as f32 - 1.5, DECK as f32 + 20.0), (FORE_END as f32 + 0.5, DECK as f32 + 5.0));
    for x in BOW - 6..FORE_X {
        for y in DECK + 4..DECK + 21 {
            if in_triangle((x as f32 + 0.5, y as f32 + 0.5), a, b, c) && w.get(x, y, MID_Z) == AIR {
                w.set(x, y, MID_Z, ids.sail);
            }
        }
    }

    // Mastiles: (x, base, punta, [(altura de la verga, medio ancho, alto de la vela)])
    let masts = [
        (FORE_X, DECK + 1, DECK + 25, [(DECK + 13, 6, 9), (DECK + 22, 4, 7)]),
        (MAIN_X, DECK + 1, DECK + 36, [(DECK + 16, 7, 13), (DECK + 27, 5, 9)]),
        (MIZZEN_X, DECK + 4, DECK + 24, [(DECK + 15, 5, 8), (DECK + 22, 3, 5)]),
    ];
    for (i, (mx, base, top, sails)) in masts.iter().enumerate() {
        for y in *base..=*top {
            w.set(*mx, y, MID_Z, ids.spruce_log);
        }
        for (k, (yard, half, height)) in sails.iter().enumerate() {
            let skull = *mx == MAIN_X && k == 0;
            build_sail(w, ids, *mx, *yard, *half, *height, skull, 90 + (i * 2 + k) as i32);
        }
        // Obenques: cuerdas desde la verga baja hasta la borda
        let (yard0, _, _) = sails[0];
        for side in [-1.0f32, 1.0] {
            for dx in [-1, 2] {
                let x = mx + dx;
                let hw = hull_half_width(ship_t(x));
                let rail_y = w.get(x, DECK + 4, MID_Z + side as i32 * 3) != AIR;
                let y = if rail_y { DECK + 5 } else { DECK + 2 };
                rope_line(w, ids, v3(*mx as f32 + 0.5, yard0 as f32 + 0.5, SHIP_Z), v3(x as f32 + 0.5, y as f32 + 0.5, SHIP_Z + side * (hw - 0.5)));
            }
        }
    }

    // Cofa (puesto de vigia) en el palo mayor
    let nest = DECK + 28;
    for dz in -2..=2 {
        for dx in -2..=2 {
            if dx == 0 && dz == 0 {
                continue;
            }
            w.set(MAIN_X + dx, nest, MID_Z + dz, ids.dark_oak);
            if dx.abs() == 2 || dz.abs() == 2 {
                w.set(MAIN_X + dx, nest + 1, MID_Z + dz, ids.dark_oak);
            }
        }
    }

    // Bandera pirata en lo alto del palo mayor, ondeando hacia la proa
    for (r, row) in FLAG.iter().enumerate() {
        for (c, ch) in row.bytes().enumerate() {
            let x = MAIN_X - 1 - c as i32;
            let y = DECK + 36 - r as i32;
            let z = MID_Z + ((c as f32 * 0.8).sin() * 0.7).round() as i32;
            w.set(x, y, z, if ch == b'#' { ids.black_wool } else { ids.sail });
        }
    }

    // Estays: cuerdas entre mastiles y hacia el baupres
    let mast_pt = |x: i32, y: i32| v3(x as f32 + 0.5, y as f32 + 0.5, SHIP_Z);
    rope_line(w, ids, mast_pt(FORE_X, DECK + 25), mast_pt(tip.0, tip.1));
    rope_line(w, ids, mast_pt(MAIN_X, DECK + 31), mast_pt(FORE_X, DECK + 15));
    rope_line(w, ids, mast_pt(MIZZEN_X, DECK + 24), mast_pt(MAIN_X, DECK + 20));
    rope_line(w, ids, mast_pt(MIZZEN_X, DECK + 24), mast_pt(STERN, DECK + 5));

    // Faroles: popa, proa y junto al palo mayor
    for side in [-1, 1] {
        let z = outer_z(w, STERN, DECK + 4, side);
        lantern(w, ids, STERN, DECK + 5, z, lights);
        let z = outer_z(w, FORE_END, DECK + 3, side);
        lantern(w, ids, FORE_END, DECK + 4, z, lights);
    }
    lantern(w, ids, MAIN_X + 1, DECK + 1, MID_Z, lights);

    // Cadena del ancla desde la proa hasta el fondo del lago
    let (ax, az) = (BOW + 4, MID_Z + 4);
    let g = ground_height(ax, az);
    for y in g + 2..=DECK + 1 {
        w.set(ax, y, az, if y < WATER { ids.chain_wet } else { ids.chain });
    }
    w.set(ax, g + 1, az, ids.obsidian);
    w.set(ax - 1, g + 1, az, ids.obsidian);
    w.set(ax + 1, g + 1, az, ids.obsidian);
}

// ---------------------------------------------------------------- muelle

fn build_dock(w: &mut World, ids: &Ids, lights: &mut Vec<Vec3>) {
    let (x0, x1, z0, z1) = (63, 66, 55, 70);
    for x in x0..x1 {
        for z in z0..z1 {
            if w.get(x, WATER, z) == AIR {
                w.set(x, WATER, z, ids.planks_slab);
            }
        }
    }
    for z in (z0..z1).step_by(4) {
        for x in [x0 - 1, x1] {
            let g = ground_height(x, z);
            for y in g + 1..=WATER + 1 {
                w.set(x, y, z, ids.spruce_log);
            }
        }
    }
    lantern(w, ids, x0 - 1, WATER + 2, z0, lights);
    lantern(w, ids, x1, WATER + 2, z0, lights);
}

// ---------------------------------------------------------------- espada

fn build_sword(w: &mut World, ids: &Ids) {
    let (cx, cz) = (SWORD_X, SWORD_Z);
    let gy = ground_height(cx, cz) + 1;

    // Pedestal escalonado
    let layers: [(i32, u8); 5] = [(6, ids.nether), (5, ids.nether), (4, ids.nether), (3, ids.end_stone), (2, ids.obsidian)];
    for (i, (r, b)) in layers.iter().enumerate() {
        let y = gy + i as i32;
        for dz in -r..=*r {
            for dx in -r..=*r {
                let edge = dx.abs() == *r || dz.abs() == *r;
                let blk = if i == 2 && edge {
                    ids.gold
                } else if i == 3 && edge && (dx + dz) % 2 == 0 {
                    ids.crying
                } else if (i == 0 || i == 1) && dx.abs() == *r && dz.abs() == *r {
                    ids.gold
                } else {
                    *b
                };
                w.set(cx + dx, y, cz + dz, blk);
            }
        }
        // Velas en el borde expuesto de cada escalon
        if i < 3 {
            for dz in -r..=*r {
                for dx in -r..=*r {
                    let edge = dx.abs() == *r || dz.abs() == *r;
                    if edge && (dx + dz).rem_euclid(3) == 0 {
                        w.set(cx + dx, y + 1, cz + dz, ids.candle);
                    }
                }
            }
        }
    }

    // Hoja: marco de obsidiana con vidrio de portal adentro (punta hacia abajo)
    let blade_base = gy + 5;
    let blade_top = blade_base + 23;
    for y in gy + 4..=blade_top {
        let half: i32 = match y - (gy + 4) {
            0 => 0,
            1 | 2 => 1,
            _ => 2,
        };
        for dx in -half..=half {
            let frame = dx.abs() == half;
            w.set(cx + dx, y, cz, if frame { ids.obsidian } else { ids.portal });
        }
    }

    // Guarda
    let gy2 = blade_top + 1;
    w.fill(cx - 8, gy2, cz - 1, cx + 9, gy2 + 1, cz + 2, ids.gold);
    for s in [-1, 1] {
        w.set(cx + s * 8, gy2 + 1, cz, ids.amethyst);
        w.set(cx + s * 9, gy2 + 1, cz, ids.amethyst);
        w.set(cx + s * 9, gy2 + 2, cz, ids.amethyst);
    }
    w.set(cx, gy2, cz + 2, ids.amethyst);
    w.set(cx, gy2, cz - 2, ids.amethyst);

    // Empunadura y pomo
    w.fill(cx - 1, gy2 + 1, cz - 1, cx + 2, gy2 + 7, cz + 2, ids.spruce_log);
    w.fill(cx - 1, gy2 + 3, cz - 1, cx + 2, gy2 + 4, cz + 2, ids.gold);
    let py = gy2 + 9;
    for y in py - 2..=py + 2 {
        for dx in -2i32..=2 {
            let m = dx.abs() + (y - py).abs();
            if m <= 2 {
                w.set(cx + dx, y, cz, ids.amethyst);
            }
            if m <= 1 {
                w.set(cx + dx, y, cz - 1, ids.amethyst);
                w.set(cx + dx, y, cz + 1, ids.amethyst);
            }
        }
    }
    w.set(cx, py + 3, cz, ids.obsidian);
}

pub fn build() -> Scene {
    let (lib, ids) = build_library();
    let mut world = World::new(NX, NY, NZ);
    world.textures = lib.textures;
    world.materials = lib.materials;
    world.blocks = lib.blocks;

    let mut lanterns = Vec::new();
    build_terrain(&mut world, &ids);
    build_sword(&mut world, &ids);
    build_dock(&mut world, &ids, &mut lanterns);
    build_ship(&mut world, &ids, &mut lanterns);
    build_trees(&mut world, &ids);
    lantern_post(&mut world, &ids, SWORD_X - 9, SWORD_Z + 9, &mut lanterns);
    lantern_post(&mut world, &ids, 26, 64, &mut lanterns);
    lantern_post(&mut world, &ids, 46, 67, &mut lanterns);
    let (ix, iz, _, _) = ISLETS[1];
    lantern_post(&mut world, &ids, ix as i32, iz as i32, &mut lanterns);
    build_fireflies(&mut world, &ids);
    world.finalize();

    // Luz de luna y un relleno azulado sin sombras
    let center = v3(ISLAND_C.0, WATER as f32, ISLAND_C.1);
    let moon = Light::directional(center, v3(MOON_DIR.x, 0.62, MOON_DIR.z), v3(0.55, 0.66, 1.0) * 0.7);
    let mut fill = Light::directional(center, v3(-0.5, 0.6, 0.65), v3(0.10, 0.12, 0.26) * 0.6);
    fill.shadows = false;
    let blade_y = (ground_height(SWORD_X, SWORD_Z) + 18) as f32;
    let mut blade = Light::point(v3(SWORD_X as f32 + 0.5, blade_y, SWORD_Z as f32 + 1.8), v3(0.7, 0.3, 1.0) * 2.5, 6.0);
    blade.max_dist = 22.0;
    let candle_y = (ground_height(SWORD_X, SWORD_Z) + 3) as f32;
    let mut candles = Light::point(v3(SWORD_X as f32 + 0.5, candle_y, SWORD_Z as f32 + 8.5), v3(1.0, 0.6, 0.3) * 1.4, 4.0);
    candles.max_dist = 12.0;
    // Luz calida que sale de las ventanas de popa
    let mut cabin = Light::point(v3(STERN as f32 + 1.6, DECK as f32 + 1.5, SHIP_Z), v3(1.0, 0.65, 0.3) * 1.3, 4.0);
    cabin.max_dist = 14.0;

    let mut lights = vec![moon, fill, blade, candles, cabin];
    for p in lanterns {
        let mut l = Light::point(p, v3(1.0, 0.62, 0.28) * 1.6, 3.5);
        l.max_dist = 13.0;
        lights.push(l);
    }

    Scene {
        world,
        lights,
        skybox: Skybox::generate(512),
        ambient_sky: v3(0.04, 0.05, 0.10),
        ambient_ground: v3(0.02, 0.018, 0.025),
        ripple_center: v3((BOW + STERN) as f32 / 2.0, WATER as f32, SHIP_Z),
    }
}
