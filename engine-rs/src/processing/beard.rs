//! Módulo de Resaltado, Tinte y Visagismo de Barba (Beard Enhancement & Barbershop Tint).
//!
//! Responsabilidad única (SRP): Cálculo de pesos anatómicos UV de vello facial,
//! perfilado de barbería (lineup) y oscurecimiento/densificación de folículos y canas.

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct BeardConfig {
    pub intensity: f32,
    pub tint_rgb: (f32, f32, f32),
}

#[inline(always)]
pub fn get_beard_color_rgb(color_name: &str) -> (f32, f32, f32) {
    match color_name {
        "black" => (0.05, 0.04, 0.04),        // Negro azabache barbería
        "dark_brown" => (0.15, 0.10, 0.07),   // Castaño oscuro cálido
        "medium_brown" => (0.24, 0.16, 0.11), // Castaño medio
        _ => (0.11, 0.08, 0.07),              // Natural (tono profundo neutro)
    }
}

/// Calcula el peso anatómico del vello facial para las coordenadas UV canónicas del rostro.
/// Retorna un valor entre 0.0 (piel limpia/ojos/labios) y 1.0 (zona densa de barba).
#[inline(always)]
pub fn calc_beard_uv_weight(u: f32, v: f32, region: u8) -> f32 {
    // Excluir labios y boca interior (región 1 y 2 de la malla o rango central de labios)
    if region == 1 || region == 2 || region == 3 {
        return 0.0;
    }
    if v >= 0.24 && v <= 0.33 && u >= 0.37 && u <= 0.63 {
        return 0.0;
    }

    // 1. ZONA DEL BIGOTE (Mustache)
    if v >= 0.33 && v <= 0.405 && u >= 0.28 && u <= 0.72 {
        let v_top = (0.405 - v) / 0.015;
        let v_bot = (v - 0.325) / 0.015;
        let v_weight = v_top.min(v_bot).clamp(0.0, 1.0);
        let u_dist = (u - 0.50).abs();
        let u_weight = 1.0 - ((u_dist - 0.12) / 0.09).clamp(0.0, 1.0);
        return (v_weight * u_weight).clamp(0.0, 1.0);
    }

    // 2. ZONA DEL MENTÓN / PERILLA / SOUL PATCH (Chin & Goatee)
    if v < 0.24 && u >= 0.20 && u <= 0.80 {
        // Soul patch central conecta con el labio inferior
        if v >= 0.20 && (u < 0.41 || u > 0.59) {
            return 0.0;
        }
        let u_dist = (u - 0.50).abs();
        let u_weight = 1.0 - ((u_dist - 0.22) / 0.08).clamp(0.0, 1.0);
        let v_bottom_fade = (v / 0.02).clamp(0.0, 1.0);
        return (u_weight * v_bottom_fade).clamp(0.0, 1.0);
    }

    // 3. MANDÍBULA Y MEJILLAS IZQUIERDAS (Viewer's left: u < 0.36)
    if u < 0.36 && v >= 0.11 && v <= 0.52 {
        // Línea de corte diagonal de barbería (razor lineup) desde patilla (u=0.08, v=0.52)
        // hasta comisura del bigote (u=0.32, v=0.34)
        let v_line = 0.52 + (u - 0.08) * (-0.18 / 0.24);
        let weight = if v > v_line {
            let dist = v - v_line;
            if dist > 0.035 {
                0.0
            } else {
                1.0 - dist / 0.035
            }
        } else {
            1.0
        };
        let v_fade = ((v - 0.10) / 0.03).clamp(0.0, 1.0);
        return (weight * v_fade).clamp(0.0, 1.0);
    }

    // 4. MANDÍBULA Y MEJILLAS DERECHAS (Viewer's right: u > 0.64)
    if u > 0.64 && v >= 0.11 && v <= 0.52 {
        // Línea de corte diagonal de barbería desde patilla (u=0.92, v=0.52)
        // hasta comisura del bigote (u=0.68, v=0.34)
        let v_line = 0.52 + (0.92 - u) * (-0.18 / 0.24);
        let weight = if v > v_line {
            let dist = v - v_line;
            if dist > 0.035 {
                0.0
            } else {
                1.0 - dist / 0.035
            }
        } else {
            1.0
        };
        let v_fade = ((v - 0.10) / 0.03).clamp(0.0, 1.0);
        return (weight * v_fade).clamp(0.0, 1.0);
    }

    0.0
}

/// Aplica el sombreado, oscurecimiento de canas y densificación de vello facial.
#[inline(always)]
pub fn apply_beard_shading(
    r: &mut f32,
    g: &mut f32,
    b: &mut f32,
    beard_weight: f32,
    beard_enhancement: f32,
    tint_rgb: (f32, f32, f32),
) {
    let beard_strength = (beard_weight * beard_enhancement).clamp(0.0, 1.0);
    if beard_strength < 0.001 {
        return;
    }

    let (tint_r, tint_g, tint_b) = tint_rgb;
    let cur_lum = 0.2126 * *r + 0.7152 * *g + 0.0722 * *b;
    let chroma = (*r).max(*g).max(*b) - (*r).min(*g).min(*b);

    // 1. Tinte y oscurecimiento inteligente de canas / vellos claros:
    // Las canas tienen luminancia moderada/alta (>0.30) pero saturación reducida (chroma < 0.24)
    let is_cana = cur_lum > 0.30 && chroma < 0.24;
    if is_cana {
        let grey_factor = ((cur_lum - 0.26) / 0.35).clamp(0.0, 1.0) * (1.0 - (chroma / 0.22).clamp(0.0, 1.0));
        let cana_blend = (grey_factor * 0.78 * beard_strength).clamp(0.0, 0.90);
        let target_tone_r = tint_r * (0.35 + 0.65 * cur_lum);
        let target_tone_g = tint_g * (0.35 + 0.65 * cur_lum);
        let target_tone_b = tint_b * (0.35 + 0.65 * cur_lum);
        *r = *r * (1.0 - cana_blend) + target_tone_r * cana_blend;
        *g = *g * (1.0 - cana_blend) + target_tone_g * cana_blend;
        *b = *b * (1.0 - cana_blend) + target_tone_b * cana_blend;
    }

    // 2. Realce y densificación de vello oscuro existente (folículos y textura de barbería):
    if cur_lum <= 0.40 {
        let hair_contrast = ((0.40 - cur_lum) / 0.35).clamp(0.0, 1.0);
        let follicle_darken = (hair_contrast * 0.55 * beard_strength).clamp(0.0, 0.70);
        let custom_r = tint_r * (0.25 + 0.75 * *r);
        let custom_g = tint_g * (0.25 + 0.75 * *g);
        let custom_b = tint_b * (0.25 + 0.75 * *b);
        *r = *r * (1.0 - follicle_darken) + custom_r * follicle_darken;
        *g = *g * (1.0 - follicle_darken) + custom_g * follicle_darken;
        *b = *b * (1.0 - follicle_darken) + custom_b * follicle_darken;
    } else if !is_cana {
        // 3. Sombreado de base / relleno sutil de piel (fondo de barbería):
        // Oscurecimiento suave para dar volumen y eliminar parches o calvas
        let shadow_fill = 0.16 * beard_strength;
        *r = *r * (1.0 - shadow_fill) + tint_r * shadow_fill;
        *g = *g * (1.0 - shadow_fill) + tint_g * shadow_fill;
        *b = *b * (1.0 - shadow_fill) + tint_b * shadow_fill;
    }
}
