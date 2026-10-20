#![allow(dead_code)]
//! Módulo de Cejas, Malla 3D UV y Visagismo de Salón.
//!
//! Responsabilidad única (SRP): Generación de mallas anatómicas de cejas,
//! perfiles de colorimetría para vellos y estilos de visagismo.

use super::geometry::MeshTriangle;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct EyebrowAccentParams {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub mesh_triangles: [MeshTriangle; 12],
    pub num_triangles: usize,
    pub is_viewer_left: bool,
    pub lower_pts: [(f32, f32); 5],
    pub brow_len: f32,
    pub head_x: f32,
    pub tail_x: f32,
}

pub fn build_eyebrow_mesh_triangles(
    lower: &[(f32, f32); 5],
    upper: &[(f32, f32); 5],
) -> ([MeshTriangle; 12], usize) {
    let mut tris = [MeshTriangle::default(); 12];
    let mut count = 0;

    let u_coords = [0.0f32, 0.25, 0.55, 0.80, 1.0];

    // Perfilado estético de salón (visagismo):
    // El borde inferior coincide exactamente con el contorno de navaja (ligeramente expandido 8% hacia abajo para cubrir raíces).
    // El borde superior tiene una elevación de arco estilizada (+52% en la cúspide) para el diseño arqueado de salón.
    let mut exp_low = [(0.0f32, 0.0f32); 5];
    let mut exp_high = [(0.0f32, 0.0f32); 5];

    for i in 0..5 {
        let p_l = lower[i];
        let p_h = upper[i];
        let vx = p_h.0 - p_l.0;
        let vy = p_h.1 - p_l.1;
        exp_low[i] = (p_l.0 - vx * 0.08, p_l.1 - vy * 0.08);

        let arch_lift = match i {
            0 => 0.25, // Cabeza / nacimiento
            1 => 0.38, // Subida del cuerpo
            2 => 0.52, // Cúspide del arco (visagismo elevado Bigo)
            3 => 0.42, // Bajada del arco
            4 => 0.25, // Transición a la cola
            _ => 0.30,
        };
        exp_high[i] = (p_h.0 + vx * arch_lift, p_h.1 + vy * arch_lift);
    }

    // 4 Quads centrales = 8 triángulos
    for i in 0..4 {
        let p0_0 = exp_low[i];
        let p0_1 = exp_low[i + 1];
        let p1_0 = exp_high[i];
        let p1_1 = exp_high[i + 1];

        let uv0_0 = (u_coords[i], 0.0);
        let uv0_1 = (u_coords[i + 1], 0.0);
        let uv1_0 = (u_coords[i], 1.0);
        let uv1_1 = (u_coords[i + 1], 1.0);

        if let Some(tri) = MeshTriangle::new(p0_0, p0_1, p1_0, uv0_0, uv0_1, uv1_0) {
            if count < 12 { tris[count] = tri; count += 1; }
        }
        if let Some(tri) = MeshTriangle::new(p1_0, p0_1, p1_1, uv1_0, uv0_1, uv1_1) {
            if count < 12 { tris[count] = tri; count += 1; }
        }
    }

    // Extensión exterior hacia la cola (Tail extension - punta afilada y definida)
    let tail_mid3 = ((exp_low[3].0 + exp_high[3].0) * 0.5, (exp_low[3].1 + exp_high[3].1) * 0.5);
    let tail_mid4 = ((exp_low[4].0 + exp_high[4].0) * 0.5, (exp_low[4].1 + exp_high[4].1) * 0.5);
    let tail_dir_x = tail_mid4.0 - tail_mid3.0;
    let tail_dir_y = tail_mid4.1 - tail_mid3.1;
    let p_tail = (tail_mid4.0 + tail_dir_x * 0.42, tail_mid4.1 + tail_dir_y * 0.42);
    if let Some(tri) = MeshTriangle::new(exp_low[4], p_tail, exp_high[4], (1.0, 0.0), (1.18, 0.50), (1.0, 1.0)) {
        if count < 12 { tris[count] = tri; count += 1; }
    }

    // Extensión interior hacia el inicio (Head extension para difuminado ombré)
    let head_mid1 = ((exp_low[1].0 + exp_high[1].0) * 0.5, (exp_low[1].1 + exp_high[1].1) * 0.5);
    let head_mid0 = ((exp_low[0].0 + exp_high[0].0) * 0.5, (exp_low[0].1 + exp_high[0].1) * 0.5);
    let head_dir_x = head_mid0.0 - head_mid1.0;
    let head_dir_y = head_mid0.1 - head_mid1.1;
    let p_head = (head_mid0.0 + head_dir_x * 0.22, head_mid0.1 + head_dir_y * 0.22);
    if let Some(tri) = MeshTriangle::new(p_head, exp_low[0], exp_high[0], (-0.14, 0.50), (0.0, 0.0), (0.0, 1.0)) {
        if count < 12 { tris[count] = tri; count += 1; }
    }

    (tris, count)
}

pub fn get_eyebrow_color_rgb(color: &str) -> Option<(f32, f32, f32)> {
    match color {
        "natural_black" => Some((0.08, 0.08, 0.09)),
        "dark_brown" => Some((0.14, 0.10, 0.08)),
        "medium_brown" => Some((0.20, 0.14, 0.10)),
        "taupe" => Some((0.26, 0.20, 0.16)),
        "ash_blonde" => Some((0.34, 0.28, 0.22)),
        "none" => None,
        _ => Some((0.14, 0.10, 0.08)),
    }
}

pub fn get_eyebrow_style_index(style: &str) -> usize {
    match style {
        "natural_fill" => 0,
        "defined_arch" => 1,
        "korean_straight" => 2,
        "fluffy_feathered" => 3,
        _ => 0,
    }
}
