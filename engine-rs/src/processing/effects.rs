//! Módulo de Efectos Especiales, Props y Máscaras 3D (Halloween, Accesorios).
//!
//! Responsabilidad única (SRP): Posicionamiento espacial 3D multi-perspectiva,
//! seguimiento angular (Yaw, Pitch, Roll) y renderizado de accesorios sobre el cráneo.

#![allow(dead_code)]

use crate::types::Point2f;

#[derive(Clone, Copy, Debug)]
pub struct SingleHornInstance {
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub scale: f32,
    pub cos_a: f32,
    pub sin_a: f32,
    pub intensity: f32,
    pub sprite_idx: usize, // 0 = front, 1 = quarter, 2 = profile
    pub mirror: bool,
    pub min_x: usize,
    pub max_x: usize,
    pub min_y: usize,
    pub max_y: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct DevilHornsParams {
    pub left: Option<SingleHornInstance>,
    pub right: Option<SingleHornInstance>,
    pub yaw_ratio: f32,
    pub min_x: usize,
    pub max_x: usize,
    pub min_y: usize,
    pub max_y: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SmoothedHornsState {
    pub left_anchor: (f32, f32),
    pub right_anchor: (f32, f32),
    pub scale: f32,
    pub roll: f32,
    pub yaw_ratio: f32,
}

/// Precomputación por frame de los dos cuernos independientes con perspectiva 3D (Yaw/Roll)
pub fn compute_devil_horns_params(
    landmarks: &[Point2f],
    intensity: f32,
    width: usize,
    height: usize,
    smoothed_pose: &mut Option<SmoothedHornsState>,
) -> Option<DevilHornsParams> {
    if landmarks.len() < 468 || intensity <= 0.001 {
        *smoothed_pose = None;
        return None;
    }

    let p_109 = landmarks[109]; // Sien izquierda (pantalla)
    let p_338 = landmarks[338]; // Sien derecha (pantalla)
    let p_10 = landmarks[10];   // Centro superior de la frente (nacimiento del cabello)
    let p_1 = landmarks[1];     // Punta de la nariz

    let raw_dx = p_338.x - p_109.x;
    let raw_dy = p_338.y - p_109.y;
    let temple_dist = (raw_dx * raw_dx + raw_dy * raw_dy).sqrt().max(10.0);
    let raw_scale = (temple_dist / 175.0).clamp(0.2, 5.0);
    let raw_roll = raw_dy.atan2(raw_dx);

    let cos_roll = raw_roll.cos();
    let sin_roll = raw_roll.sin();

    // Vector 'arriba' perpendicular a la línea de las sienes en pantalla
    let up_x = sin_roll * (18.0 * raw_scale);
    let up_y = -cos_roll * (18.0 * raw_scale);

    // Anclas anatómicas separadas para cada cuerno (lado izquierdo y derecho del cráneo)
    let raw_left_x = p_109.x * 0.72 + p_10.x * 0.28 + up_x;
    let raw_left_y = p_109.y * 0.72 + p_10.y * 0.28 + up_y;

    let raw_right_x = p_338.x * 0.72 + p_10.x * 0.28 + up_x;
    let raw_right_y = p_338.y * 0.72 + p_10.y * 0.28 + up_y;

    // Cálculo del Yaw 3D (giro de la cabeza):
    // Desplazamiento de la punta de la nariz respecto al punto medio de las sienes
    let mid_x = (p_109.x + p_338.x) * 0.5;
    let mid_y = (p_109.y + p_338.y) * 0.5;
    let nose_proj = (p_1.x - mid_x) * cos_roll + (p_1.y - mid_y) * sin_roll;
    let raw_yaw_ratio = (nose_proj / (temple_dist * 0.45)).clamp(-1.5, 1.5);

    // Estabilización temporal adaptativa (elimina jitter y mantiene suavidad a 60 FPS)
    let (cur_left, cur_right, scale, roll, yaw_ratio) = match *smoothed_pose {
        Some(prev) => {
            let dist_sq = (raw_left_x - prev.left_anchor.0).powi(2)
                + (raw_left_y - prev.left_anchor.1).powi(2)
                + (raw_right_x - prev.right_anchor.0).powi(2)
                + (raw_right_y - prev.right_anchor.1).powi(2);

            let alpha = if dist_sq < 4.0 {
                0.25
            } else if dist_sq < 49.0 {
                0.55
            } else {
                0.85
            };

            let nl_x = prev.left_anchor.0 * (1.0 - alpha) + raw_left_x * alpha;
            let nl_y = prev.left_anchor.1 * (1.0 - alpha) + raw_left_y * alpha;
            let nr_x = prev.right_anchor.0 * (1.0 - alpha) + raw_right_x * alpha;
            let nr_y = prev.right_anchor.1 * (1.0 - alpha) + raw_right_y * alpha;
            let ns = prev.scale * (1.0 - alpha) + raw_scale * alpha;
            let nr = prev.roll * (1.0 - alpha) + raw_roll * alpha;
            let ny = prev.yaw_ratio * (1.0 - alpha) + raw_yaw_ratio * alpha;

            let updated = SmoothedHornsState {
                left_anchor: (nl_x, nl_y),
                right_anchor: (nr_x, nr_y),
                scale: ns,
                roll: nr,
                yaw_ratio: ny,
            };
            *smoothed_pose = Some(updated);
            ((nl_x, nl_y), (nr_x, nr_y), ns, nr, ny)
        }
        None => {
            let init = SmoothedHornsState {
                left_anchor: (raw_left_x, raw_left_y),
                right_anchor: (raw_right_x, raw_right_y),
                scale: raw_scale,
                roll: raw_roll,
                yaw_ratio: raw_yaw_ratio,
            };
            *smoothed_pose = Some(init);
            (
                (raw_left_x, raw_left_y),
                (raw_right_x, raw_right_y),
                raw_scale,
                raw_roll,
                raw_yaw_ratio,
            )
        }
    };

    let cos_a = roll.cos();
    let sin_a = roll.sin();

    let reach_x = 240.0 * scale;
    let reach_y_up = 440.0 * scale;
    let reach_y_down = 80.0 * scale;

    let eff_intensity = intensity.clamp(0.0, 1.0);

    // 1. Configuración del Cuerno Izquierdo (pantalla izquierda)
    let left_instance = {
        let eff_yaw = -yaw_ratio;
        let alpha = if yaw_ratio > 0.45 {
            ((1.0 - (yaw_ratio - 0.45) / 0.25).clamp(0.0, 1.0)) * eff_intensity
        } else {
            eff_intensity
        };

        if alpha > 0.005 {
            let abs_y = eff_yaw.abs();
            let sprite_idx = if abs_y < 0.28 {
                0 // Front
            } else if abs_y < 0.65 {
                1 // Quarter
            } else {
                2 // Profile
            };

            // En vista de perfil con cabeza girada a la derecha, debe curvar hacia la izquierda
            let mirror = sprite_idx != 2 || yaw_ratio > 0.0;

            let min_x = (cur_left.0 - reach_x).floor().max(0.0) as usize;
            let max_x = (cur_left.0 + reach_x).ceil().min(width as f32 - 1.0) as usize;
            let min_y = (cur_left.1 - reach_y_up).floor().max(0.0) as usize;
            let max_y = (cur_left.1 + reach_y_down).ceil().min(height as f32 - 1.0) as usize;

            if min_x < max_x && min_y < max_y {
                Some(SingleHornInstance {
                    anchor_x: cur_left.0,
                    anchor_y: cur_left.1,
                    scale,
                    cos_a,
                    sin_a,
                    intensity: alpha,
                    sprite_idx,
                    mirror,
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                })
            } else {
                None
            }
        } else {
            None
        }
    };

    // 2. Configuración del Cuerno Derecho (pantalla derecha)
    let right_instance = {
        let eff_yaw = yaw_ratio;
        let alpha = if yaw_ratio < -0.45 {
            ((1.0 - (-yaw_ratio - 0.45) / 0.25).clamp(0.0, 1.0)) * eff_intensity
        } else {
            eff_intensity
        };

        if alpha > 0.005 {
            let abs_y = eff_yaw.abs();
            let sprite_idx = if abs_y < 0.28 {
                0 // Front
            } else if abs_y < 0.65 {
                1 // Quarter
            } else {
                2 // Profile
            };

            // En vista de perfil con cabeza girada a la derecha, debe curvar hacia la izquierda (hacia atrás)
            let mirror = (sprite_idx == 2 && yaw_ratio > 0.0) || (sprite_idx != 2);

            let min_x = (cur_right.0 - reach_x).floor().max(0.0) as usize;
            let max_x = (cur_right.0 + reach_x).ceil().min(width as f32 - 1.0) as usize;
            let min_y = (cur_right.1 - reach_y_up).floor().max(0.0) as usize;
            let max_y = (cur_right.1 + reach_y_down).ceil().min(height as f32 - 1.0) as usize;

            if min_x < max_x && min_y < max_y {
                Some(SingleHornInstance {
                    anchor_x: cur_right.0,
                    anchor_y: cur_right.1,
                    scale,
                    cos_a,
                    sin_a,
                    intensity: alpha,
                    sprite_idx,
                    mirror,
                    min_x,
                    max_x,
                    min_y,
                    max_y,
                })
            } else {
                None
            }
        } else {
            None
        }
    };

    if left_instance.is_none() && right_instance.is_none() {
        return None;
    }

    let mut ov_min_x = width - 1;
    let mut ov_max_x = 0;
    let mut ov_min_y = height - 1;
    let mut ov_max_y = 0;

    if let Some(ref l) = left_instance {
        ov_min_x = ov_min_x.min(l.min_x);
        ov_max_x = ov_max_x.max(l.max_x);
        ov_min_y = ov_min_y.min(l.min_y);
        ov_max_y = ov_max_y.max(l.max_y);
    }
    if let Some(ref r) = right_instance {
        ov_min_x = ov_min_x.min(r.min_x);
        ov_max_x = ov_max_x.max(r.max_x);
        ov_min_y = ov_min_y.min(r.min_y);
        ov_max_y = ov_max_y.max(r.max_y);
    }

    if ov_min_x >= ov_max_x || ov_min_y >= ov_max_y {
        return None;
    }

    Some(DevilHornsParams {
        left: left_instance,
        right: right_instance,
        yaw_ratio,
        min_x: ov_min_x,
        max_x: ov_max_x,
        min_y: ov_min_y,
        max_y: ov_max_y,
    })
}

/// Muestreo bilineal sub-píxel de textura 384x512 RGBA
#[inline(always)]
pub fn sample_sprite_bilinear_384x512(sprite: &[[f32; 4]], sx: f32, sy: f32) -> [f32; 4] {
    let x0 = sx.floor() as usize;
    let y0 = sy.floor() as usize;
    let x1 = (x0 + 1).min(383);
    let y1 = (y0 + 1).min(511);
    let tx = sx - x0 as f32;
    let ty = sy - y0 as f32;

    let c00 = sprite[y0 * 384 + x0];
    let c10 = sprite[y0 * 384 + x1];
    let c01 = sprite[y1 * 384 + x0];
    let c11 = sprite[y1 * 384 + x1];

    let mut out = [0.0f32; 4];
    for c in 0..4 {
        let top = c00[c] * (1.0 - tx) + c10[c] * tx;
        let bot = c01[c] * (1.0 - tx) + c11[c] * tx;
        out[c] = top * (1.0 - ty) + bot * ty;
    }
    out
}

/// Muestra un cuerno individual en el píxel (px, py)
#[inline(always)]
pub fn sample_single_horn_pixel(
    horn: &SingleHornInstance,
    sprites: &[Vec<[f32; 4]>; 3],
    px: f32,
    py: f32,
) -> Option<[f32; 4]> {
    let rel_x = px - horn.anchor_x;
    let rel_y = py - horn.anchor_y;

    let mut unrot_x = rel_x * horn.cos_a + rel_y * horn.sin_a;
    let unrot_y = -rel_x * horn.sin_a + rel_y * horn.cos_a;

    if horn.mirror {
        unrot_x = -unrot_x;
    }

    let inv_scale = 1.0 / horn.scale;
    let sx = 192.0 + unrot_x * inv_scale;
    let sy = 455.0 + unrot_y * inv_scale;

    if sx >= 0.0 && sx < 383.0 && sy >= 0.0 && sy < 511.0 {
        let sample = sample_sprite_bilinear_384x512(&sprites[horn.sprite_idx], sx, sy);
        if sample[3] > 0.005 {
            let mut out = sample;
            out[3] = (out[3] * horn.intensity).clamp(0.0, 1.0);
            return Some(out);
        }
    }
    None
}

/// Renderiza la composición de ambos cuernos con orden Z en el píxel (px, py)
#[inline(always)]
pub fn sample_devil_horns_pixel(
    params: &DevilHornsParams,
    sprites: &[Vec<[f32; 4]>; 3],
    px: f32,
    py: f32,
) -> Option<[f32; 4]> {
    // Orden Z: si yaw_ratio > 0 (cabeza girada a la derecha), el cuerno derecho está adelante
    let (first_horn, second_horn) = if params.yaw_ratio > 0.0 {
        (params.left.as_ref(), params.right.as_ref())
    } else {
        (params.right.as_ref(), params.left.as_ref())
    };

    let sample_first = first_horn.and_then(|h| sample_single_horn_pixel(h, sprites, px, py));
    let sample_second = second_horn.and_then(|h| sample_single_horn_pixel(h, sprites, px, py));

    match (sample_first, sample_second) {
        (Some(c1), Some(c2)) => {
            // Mezclar c2 (adelante) sobre c1 (atrás)
            let a2 = c2[3];
            let inv_a2 = 1.0 - a2;
            let out_a = a2 + c1[3] * inv_a2;
            if out_a > 0.001 {
                let out_r = (c2[0] * a2 + c1[0] * c1[3] * inv_a2) / out_a;
                let out_g = (c2[1] * a2 + c1[1] * c1[3] * inv_a2) / out_a;
                let out_b = (c2[2] * a2 + c1[2] * c1[3] * inv_a2) / out_a;
                Some([out_r, out_g, out_b, out_a])
            } else {
                None
            }
        }
        (None, Some(c2)) => Some(c2),
        (Some(c1), None) => Some(c1),
        (None, None) => None,
    }
}
