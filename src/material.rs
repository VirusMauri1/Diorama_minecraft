use crate::math::Vec3;

/// Material de un bloque. Cada uno tiene su propia textura y sus propios
/// parametros de albedo, especular, transparencia y reflectividad.
#[derive(Clone, Debug)]
pub struct Material {
    pub name: &'static str,
    /// Indice de la textura difusa.
    pub texture: usize,
    /// Cuantos bloques cubre una repeticion de la textura.
    pub tex_scale: f32,
    /// Peso de la luz difusa (Lambert).
    pub albedo: f32,
    /// Peso del brillo especular (Phong).
    pub specular: f32,
    /// Exponente especular (que tan concentrado es el brillo).
    pub shininess: f32,
    /// Fraccion de luz reflejada (espejo).
    pub reflectivity: f32,
    /// Fraccion de luz que atraviesa el material (refraccion).
    pub transparency: f32,
    /// Indice de refraccion.
    pub ior: f32,
    /// Emision propia (portal, amatista, velas).
    pub emission: f32,
    /// Cuanta luz absorbe por bloque recorrido (Beer-Lambert).
    pub absorption: Vec3,
    /// Recorta la geometria con el alfa de la textura (hojas).
    pub cutout: bool,
    /// Perturba la normal con ondas (superficie del agua).
    pub ripple: bool,
}

impl Material {
    pub fn new(name: &'static str, texture: usize) -> Self {
        Material {
            name,
            texture,
            tex_scale: 1.0,
            albedo: 0.9,
            specular: 0.1,
            shininess: 16.0,
            reflectivity: 0.0,
            transparency: 0.0,
            ior: 1.0,
            emission: 0.0,
            absorption: Vec3::ZERO,
            cutout: false,
            ripple: false,
        }
    }

    pub fn albedo(mut self, v: f32) -> Self {
        self.albedo = v;
        self
    }
    pub fn specular(mut self, strength: f32, shininess: f32) -> Self {
        self.specular = strength;
        self.shininess = shininess;
        self
    }
    pub fn reflective(mut self, v: f32) -> Self {
        self.reflectivity = v;
        self
    }
    pub fn transparent(mut self, transparency: f32, ior: f32, absorption: Vec3) -> Self {
        self.transparency = transparency;
        self.ior = ior;
        self.absorption = absorption;
        self
    }
    pub fn emissive(mut self, v: f32) -> Self {
        self.emission = v;
        self
    }
    pub fn scale(mut self, s: f32) -> Self {
        self.tex_scale = s;
        self
    }
    pub fn cutout(mut self) -> Self {
        self.cutout = true;
        self
    }
    pub fn ripple(mut self) -> Self {
        self.ripple = true;
        self
    }
}
