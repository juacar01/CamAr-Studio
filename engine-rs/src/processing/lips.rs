//! Módulo de Estética Labial y Parámetros Deformables.
//!
//! Responsabilidad única (SRP): Definición geométrica y límites de las 4 curvas labiales.

#[derive(Clone, Copy)]
pub struct LipSpriteParams {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub left_x: f32,
    pub right_x: f32,
    pub inv_lip_width: f32,
    pub outer_top: [(f32, f32); 11],
    pub inner_top: [(f32, f32); 11],
    pub inner_bot: [(f32, f32); 11],
    pub outer_bot: [(f32, f32); 11],
}
