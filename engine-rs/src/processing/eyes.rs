//! Módulo de Estética Ocular, Detección de Iris, Malla 3D de Sombras y Morfología.
//!
//! Responsabilidad única (SRP): Geometría analítica del ojo (apertura palpebral, inclinación cantal),
//! localización óptica del iris sub-píxel, generación de mallas UV orbitales y deformación anatómica.

use crate::types::Point2f;
use super::geometry::{LineSegment2D, MeshTriangle, sample_lid_y};

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct EyeAccentParams {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub left_x: f32,
    pub right_x: f32,
    pub upper_pts: [(f32, f32); 9],
    pub lower_pts: [(f32, f32); 9],
    pub es_upper: [(f32, f32); 9],
    pub es_crease: [(f32, f32); 9],
    pub mesh_triangles: [MeshTriangle; 38],
    pub num_triangles: usize,
    pub pupil_x: f32,
    pub pupil_y: f32,
    pub iris_rad: f32,
    pub inv_iris_rad: f32,
    pub iris_lum: f32,
    pub sclera_lum: f32,
    pub hl1_x: f32,
    pub hl1_y: f32,
    pub r_spec1_sq: f32,
    pub inv_r_spec1: f32,
    pub hl2_x: f32,
    pub hl2_y: f32,
    pub r_spec2_sq: f32,
    pub inv_r_spec2: f32,
    pub segments: [LineSegment2D; 9],
    pub lower_segments: [LineSegment2D; 8],
    pub is_viewer_left: bool,
    pub inner_x: f32,
    pub outer_x: f32,
    pub inv_eye_width: f32,
    pub eye_w: f32,
    pub eye_h: f32,
    pub shape_id: u8, // 0=almond, 1=foxy, 2=asian, 3=doe, 4=hooded
    pub inter_canthal_ratio: f32, // Distancia inter-ocular / ancho de ojo
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct EyeMorphParams {
    pub cx: f32,
    pub cy: f32,
    pub rx: f32,
    pub h_top: f32,
    pub h_bot: f32,
    pub limit_y_top: f32,
    pub limit_y_bot: f32,
    pub is_viewer_left: bool,
    pub shape_id: u8,
}

pub fn eye_shape_to_id(shape: &str) -> u8 {
    match shape {
        "foxy" => 1,
        "asian" => 2,
        "doe" => 3,
        "hooded" => 4,
        _ => 0, // "almond"
    }
}

pub fn detect_eye_shape(lm: &[Point2f]) -> &'static str {
    if lm.len() < 468 {
        return "almond";
    }
    let wl = (lm[33].x - lm[133].x).abs();
    let wr = (lm[263].x - lm[362].x).abs();
    let w = ((wl + wr) * 0.5).max(1.0);

    let hl = (lm[145].y - lm[159].y).abs();
    let hr = (lm[374].y - lm[386].y).abs();
    let h = (hl + hr) * 0.5;

    let ratio = h / w;

    // Inclinación cantal (canthal tilt): diferencia vertical entre comisura interna y externa
    let dyl = lm[133].y - lm[33].y;
    let dxl = (lm[33].x - lm[133].x).abs().max(1.0);
    let tilt_l = dyl / dxl;

    let dyr = lm[362].y - lm[263].y;
    let dxr = (lm[263].x - lm[362].x).abs().max(1.0);
    let tilt_r = dyr / dxr;

    let tilt = (tilt_l + tilt_r) * 0.5;

    // Relación de pliegue palpebral a ceja (hooding ratio)
    let crease_dist_l = (lm[159].y - lm[223].y).abs();
    let brow_dist_l = (lm[159].y - lm[105].y).abs().max(1.0);
    let crease_dist_r = (lm[386].y - lm[443].y).abs();
    let brow_dist_r = (lm[386].y - lm[334].y).abs().max(1.0);
    let hood_ratio = ((crease_dist_l / brow_dist_l) + (crease_dist_r / brow_dist_r)) * 0.5;

    if tilt > 0.070 {
        "foxy"
    } else if ratio > 0.375 {
        "doe"
    } else if hood_ratio < 0.22 && ratio < 0.30 {
        "asian"
    } else if hood_ratio < 0.26 {
        "hooded"
    } else if tilt < -0.025 {
        // Downturned Eyes -> Corregidos con técnica lifting ascendente
        "foxy"
    } else {
        "almond"
    }
}

/// Localizador óptico de disco circular de iris (centro sub-píxel + radio exacto) inmune a reflejos (catchlights).
pub fn locate_iris_disc(
    upper_pts: &[(f32, f32); 9],
    lower_pts: &[(f32, f32); 9],
    input_rgba: &[u8],
    width: usize,
    height: usize,
) -> (f32, f32, f32, f32, f32) {
    let left_x = upper_pts[0].0.min(lower_pts[0].0);
    let right_x = upper_pts[8].0.max(lower_pts[8].0);
    let eye_w = (right_x - left_x).max(8.0);

    // Centro anatómico del arco corneal (excluyendo el sesgo horizontal del lagrimal interno 133/362)
    let arch_mid_x = (upper_pts[3].0
        + upper_pts[4].0 * 2.0
        + upper_pts[5].0
        + lower_pts[3].0
        + lower_pts[4].0 * 2.0
        + lower_pts[5].0)
        * 0.125;

    let top_mid_y = sample_lid_y(upper_pts, arch_mid_x);
    let bot_mid_y = sample_lid_y(lower_pts, arch_mid_x);
    let eye_h = (bot_mid_y - top_mid_y).max(2.0);
    let mid_y = (top_mid_y + bot_mid_y) * 0.5;
    let r_anat = (eye_w * 0.222).clamp(5.0, 32.0);

    if eye_h < eye_w * 0.12 || input_rgba.len() < width * height * 4 {
        return (arch_mid_x, mid_y, r_anat, 0.12, 0.38);
    }

    let sample_luma = |fx: f32, fy: f32| -> f32 {
        let ix = (fx.round() as i32).clamp(0, width as i32 - 1) as usize;
        let iy = (fy.round() as i32).clamp(0, height as i32 - 1) as usize;
        let idx = (iy * width + ix) * 4;
        let r = input_rgba[idx] as f32 * (1.0 / 255.0);
        let g = input_rgba[idx + 1] as f32 * (1.0 / 255.0);
        let b = input_rgba[idx + 2] as f32 * (1.0 / 255.0);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };

    const N: usize = 41;
    let x_start = left_x + eye_w * 0.10;
    let x_end = right_x - eye_w * 0.10;
    let x_span = (x_end - x_start).max(4.0);
    let mut xs = [0.0_f32; N];
    let mut profile = [0.0_f32; N];

    for i in 0..N {
        let cx = x_start + (i as f32 / (N - 1) as f32) * x_span;
        xs[i] = cx;
        let yt = sample_lid_y(upper_pts, cx);
        let yb = sample_lid_y(lower_pts, cx);
        let lh = (yb - yt).max(2.0);
        let y0 = yt + lh * 0.36;
        let y1 = (yb - lh * 0.12).max(y0 + 0.5);

        let mut v = [0.0_f32; 5];
        for k in 0..5 {
            let sy = y0 + (k as f32 * 0.25) * (y1 - y0);
            v[k] = sample_luma(cx, sy);
        }
        for a in 0..4 {
            for b in (a + 1)..5 {
                if v[b] < v[a] {
                    v.swap(a, b);
                }
            }
        }
        profile[i] = (v[1] + v[2]) * 0.5;
    }

    let mut min_val = 1e9_f32;
    let mut i_min = N / 2;
    let win_start = (N * 26) / 100;
    let win_end = (N * 74) / 100;
    for i in win_start..=win_end {
        if profile[i] < min_val {
            min_val = profile[i];
            i_min = i;
        }
    }
    let p_iris = min_val;

    let mut p_scl_l = p_iris;
    let mut i_scl_l = 0;
    for i in 0..i_min {
        if profile[i] > p_scl_l {
            p_scl_l = profile[i];
            i_scl_l = i;
        }
    }

    let mut p_scl_r = p_iris;
    let mut i_scl_r = N - 1;
    for i in (i_min + 1)..N {
        if profile[i] > p_scl_r {
            p_scl_r = profile[i];
            i_scl_r = i;
        }
    }

    let th_l = p_iris + 0.45 * (p_scl_l - p_iris).max(0.04);
    let mut x_l = xs[i_scl_l];
    let mut idx = i_min;
    while idx > i_scl_l {
        if profile[idx - 1] >= th_l {
            let denom = (profile[idx - 1] - profile[idx]).max(1e-4);
            let frac = ((th_l - profile[idx]) / denom).clamp(0.0, 1.0);
            x_l = xs[idx] - frac * (xs[idx] - xs[idx - 1]);
            break;
        }
        idx -= 1;
    }

    let th_r = p_iris + 0.45 * (p_scl_r - p_iris).max(0.04);
    let mut x_r = xs[i_scl_r];
    for i in i_min..i_scl_r {
        if profile[i + 1] >= th_r {
            let denom = (profile[i + 1] - profile[i]).max(1e-4);
            let frac = ((th_r - profile[i]) / denom).clamp(0.0, 1.0);
            x_r = xs[i] + frac * (xs[i + 1] - xs[i]);
            break;
        }
    }

    let c_l = p_scl_l - p_iris;
    let c_r = p_scl_r - p_iris;
    let (cx, iris_rad) = if c_l >= 0.035 && c_r >= 0.035 {
        let det_cx = 0.5 * (x_l + x_r);
        let det_r = 0.5 * (x_r - x_l) * 1.02;
        let r_blend = (0.70 * det_r + 0.30 * r_anat).clamp(eye_w * 0.200, eye_w * 0.245);
        (det_cx, r_blend)
    } else if c_l >= 0.035 {
        let r_use = r_anat.clamp(eye_w * 0.200, eye_w * 0.240);
        (x_l + r_use, r_use)
    } else if c_r >= 0.035 {
        let r_use = r_anat.clamp(eye_w * 0.200, eye_w * 0.240);
        (x_r - r_use, r_use)
    } else {
        let r_use = r_anat.clamp(eye_w * 0.200, eye_w * 0.240);
        (arch_mid_x, r_use)
    };

    let y_t_cx = sample_lid_y(upper_pts, cx);
    let y_b_cx = sample_lid_y(lower_pts, cx);
    let h_cx = (y_b_cx - y_t_cx).max(2.0);

    // Escaneo óptico vertical a lo largo de cx para localizar el centro real de la pupila oscura
    const M: usize = 25;
    let mut min_vert_lum = 1e9_f32;
    let mut best_cy = (y_t_cx + y_b_cx) * 0.5;
    let y_v_start = y_t_cx + h_cx * 0.10;
    let y_v_end = y_b_cx - h_cx * 0.10;
    let y_span = (y_v_end - y_v_start).max(2.0);

    for j in 0..M {
        let sy = y_v_start + (j as f32 / (M - 1) as f32) * y_span;
        let l0 = sample_luma(cx - 1.0, sy);
        let l1 = sample_luma(cx, sy);
        let l2 = sample_luma(cx + 1.0, sy);
        let lum_v = (l0 + l1 * 2.0 + l2) * 0.25;
        if lum_v < min_vert_lum {
            min_vert_lum = lum_v;
            best_cy = sy;
        }
    }

    let mid_y_cx = (y_t_cx + y_b_cx) * 0.5;
    let raw_cy = if min_vert_lum < p_iris + 0.16 {
        0.80 * best_cy + 0.20 * mid_y_cx
    } else {
        mid_y_cx
    };

    // Rango seguro: garantizar incondicionalmente que safe_min <= safe_max para evitar panics de clamp
    let min_limit = (y_t_cx - iris_rad * 0.35).min(y_b_cx);
    let max_limit = (y_b_cx + iris_rad * 0.35).max(y_t_cx);
    let (safe_min, safe_max) = if min_limit <= max_limit {
        (min_limit, max_limit)
    } else {
        (max_limit, min_limit)
    };
    let cy = raw_cy.clamp(safe_min, safe_max);

    let sclera_lum = c_l.min(c_r).max(0.06) + p_iris;
    (cx, cy, iris_rad.clamp(4.0, 32.0), p_iris, sclera_lum)
}

/// Construye la malla 3D de 38 triángulos para la cuenca orbital del ojo y párpado móvil.
pub fn build_eye_mesh_triangles(
    r0: &[(f32, f32); 9],
    r1: &[(f32, f32); 9],
    r2: &[(f32, f32); 9],
    wing_pt: (f32, f32),
) -> ([MeshTriangle; 38], usize) {
    let mut tris = [MeshTriangle::default(); 38];
    let mut count = 0;

    let u_coords = [1.0f32, 0.875, 0.75, 0.625, 0.50, 0.375, 0.25, 0.125, 0.0];

    // Strip 1: Párpado Móvil (Row 0 a Row 1)
    for i in 0..8 {
        let p0_0 = r0[i];
        let p0_1 = r0[i + 1];
        let p1_0 = r1[i];
        let p1_1 = r1[i + 1];

        let uv0_0 = (u_coords[i], 0.0);
        let uv0_1 = (u_coords[i + 1], 0.0);
        let uv1_0 = (u_coords[i], 0.45);
        let uv1_1 = (u_coords[i + 1], 0.45);

        if let Some(tri) = MeshTriangle::new(p0_0, p0_1, p1_0, uv0_0, uv0_1, uv1_0) {
            if count < 38 { tris[count] = tri; count += 1; }
        }
        if let Some(tri) = MeshTriangle::new(p1_0, p0_1, p1_1, uv1_0, uv0_1, uv1_1) {
            if count < 38 { tris[count] = tri; count += 1; }
        }
    }

    // Strip 2: Cuenca / Transición Orbital (Row 1 a Row 2)
    for i in 0..8 {
        let p1_0 = r1[i];
        let p1_1 = r1[i + 1];
        let p2_0 = r2[i];
        let p2_1 = r2[i + 1];

        let uv1_0 = (u_coords[i], 0.45);
        let uv1_1 = (u_coords[i + 1], 0.45);
        let uv2_0 = (u_coords[i], 0.95);
        let uv2_1 = (u_coords[i + 1], 0.95);

        if let Some(tri) = MeshTriangle::new(p1_0, p1_1, p2_0, uv1_0, uv1_1, uv2_0) {
            if count < 38 { tris[count] = tri; count += 1; }
        }
        if let Some(tri) = MeshTriangle::new(p2_0, p1_1, p2_1, uv2_0, uv1_1, uv2_1) {
            if count < 38 { tris[count] = tri; count += 1; }
        }
    }

    // Outer Wing (Extensión exterior felina hacia la sien)
    let uv_wing = (1.25f32, 0.30f32);
    if let Some(tri) = MeshTriangle::new(r0[0], wing_pt, r1[0], (1.0, 0.0), uv_wing, (1.0, 0.45)) {
        if count < 38 { tris[count] = tri; count += 1; }
    }
    if let Some(tri) = MeshTriangle::new(r1[0], wing_pt, r2[0], (1.0, 0.45), uv_wing, (1.0, 0.95)) {
        if count < 38 { tris[count] = tri; count += 1; }
    }

    (tris, count)
}

/// Deformación geométrica continua y suave de ojos (agrandamiento isotrópico y remodelado de forma).
#[inline(always)]
pub fn warp_eye(
    px: f32,
    py: f32,
    eye: &EyeMorphParams,
    eye_enlarge: f32,
    eye_round: f32,
    sx: &mut f32,
    sy: &mut f32,
) {
    let dx = px - eye.cx;
    let dy = py - eye.cy;

    let max_rx = eye.rx * 1.35;
    let abs_dx = dx.abs();
    if abs_dx >= max_rx {
        return;
    }

    let ry = if dy < 0.0 { eye.limit_y_top } else { eye.limit_y_bot };
    let abs_dy = dy.abs();
    if abs_dy >= ry {
        return;
    }

    let norm_x = abs_dx / max_rx;
    let norm_y = abs_dy / ry;
    let r_sq = norm_x * norm_x + norm_y * norm_y;
    if r_sq >= 1.0 {
        return;
    }

    let factor = (1.0 - r_sq).powi(2);
    let iso_scale = 0.32 * eye_enlarge;
    let mut wx = iso_scale * factor;
    let mut wy = iso_scale * factor;

    match eye.shape_id {
        1 => {
            // FOXY: Elevación y extensión del canto temporal exterior
            let is_outer = if eye.is_viewer_left { dx < 0.0 } else { dx > 0.0 };
            if is_outer {
                let outer_t = (abs_dx / max_rx).clamp(0.0, 1.0);
                wy += outer_t.powi(2) * 0.16 * factor;
                wx += outer_t * 0.12 * factor;
            }
        }
        2 => {
            // ASIÁTICO: Alargamiento horizontal
            wx += 0.12 * factor;
            wy -= 0.05 * factor;
        }
        3 => {
            // DOE: Apertura vertical centrada
            let center_t = (1.0 - norm_x).clamp(0.0, 1.0).powi(2);
            wy += center_t * (0.16 + 0.08 * eye_round) * factor;
        }
        4 => {
            // HOODED: Profundización de cuenca
            if dy < 0.0 {
                wy -= 0.05 * factor;
            }
            let is_outer = if eye.is_viewer_left { dx < 0.0 } else { dx > 0.0 };
            if is_outer {
                wx += 0.06 * factor;
            }
        }
        _ => {
            // ALMOND: Proporción clásica
            let center_t = (1.0 - norm_x).clamp(0.0, 1.0);
            wy += center_t * 0.04 * factor;
        }
    }

    *sx -= dx * wx;
    *sy -= dy * wy;
}

pub fn get_eyeshadow_color_rgb(color: &str) -> Option<(f32, f32, f32)> {
    match color {
        "bronze" => Some((0.72, 0.45, 0.30)),
        "rose" => Some((0.82, 0.38, 0.48)),
        "peach" => Some((0.88, 0.52, 0.42)),
        "plum" => Some((0.55, 0.22, 0.38)),
        "smoky_black" => Some((0.18, 0.18, 0.20)),
        "black_tea" => Some((0.26, 0.16, 0.13)),
        "gold" => Some((0.85, 0.68, 0.32)),
        "champagne" => Some((0.92, 0.82, 0.72)),
        "none" => None,
        _ => Some((0.72, 0.45, 0.30)),
    }
}

pub fn get_eyeshadow_style_index(style: &str) -> usize {
    match style {
        "natural_nude" => 0,
        "smoky_glam" => 1,
        "cat_wing" => 2,
        "sunset_glow" => 3,
        "grunge_kohl" => 4,
        "foxy_glam" => 5,
        "glitter_pop" => 6,
        _ => 0,
    }
}
