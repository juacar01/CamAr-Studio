//! Módulo de Ciencia de Color y Filtros LUT Cinematográficos.
//!
//! Responsabilidad única (SRP): Conversión de espacios de color (RGB, YCbCr, HSV),
//! compresión de altas luces (Filmic shoulder roll-off) y graduación de color (LUTs).

/// Conversión Rec. 709 RGB a YCbCr estándar HDTV/PC
#[inline(always)]
pub fn rgb_to_ycbcr(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let cb = -0.1146 * r - 0.3854 * g + 0.5000 * b;
    let cr = 0.5000 * r - 0.4542 * g - 0.0458 * b;
    (y, cb, cr)
}

/// Conversión Rec. 709 YCbCr a RGB estándar HDTV/PC
#[inline(always)]
pub fn ycbcr_to_rgb(y: f32, cb: f32, cr: f32) -> (f32, f32, f32) {
    let r = y + 1.5748 * cr;
    let g = y - 0.1873 * cb - 0.4681 * cr;
    let b = y + 1.8556 * cb;
    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

/// Compresión Filmic de Altas Luces (Soft Highlight Roll-off C^1 Continuo sin exp() transcendental)
/// Previene el quemado abrupto a 1.0 (#FFFFFF) en paredes o reflejos y elimina bandas de Mach
#[inline(always)]
pub fn filmic_soft_shoulder(c: f32) -> f32 {
    let knee = 0.62_f32;
    if c <= knee {
        c.max(0.0)
    } else {
        let x = c - knee;
        let s = 1.8 * x;
        let shoulder = knee + 0.32 * (s / (1.0 + 0.85 * s));
        let t = (x * (1.0 / 0.60)).clamp(0.0, 1.0);
        let blend = t * t * (3.0 - 2.0 * t);
        (c * (1.0 - blend) + shoulder * blend).clamp(0.0, 1.0)
    }
}

/// Conversión RGB a HSV
#[inline(always)]
pub fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let s = if max == 0.0 { 0.0 } else { d / max };
    let v = max;

    let h = if d == 0.0 {
        0.0
    } else if max == r {
        (g - b) / d + (if g < b { 6.0 } else { 0.0 })
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;

    (h, s, v)
}

/// Conversión HSV a RGB
#[inline(always)]
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let mut h_norm = h % 1.0;
    if h_norm < 0.0 {
        h_norm += 1.0;
    }

    let i = (h_norm * 6.0).floor() as i32;
    let f = h_norm * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);

    match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

/// Aplica un perfil de colorimetría / graduación LUT cinematográfico al pixel (r, g, b).
#[inline(always)]
pub fn apply_filter_lut(r: f32, g: f32, b: f32, lut_id: &str) -> (f32, f32, f32) {
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    match lut_id {
        // ==========================================
        // 1. CATEGORÍA: NATURAL
        // ==========================================
        "nat_porcelain" => {
            // Porcelana limpia: sutil realce rosa/frío, cero ictericia, sombras limpias
            let nr = r * 1.03;
            let ng = g * 0.985;
            let nb = b * 1.025;
            let bell = 4.0 * luma * (1.0 - luma);
            let lift = bell * 0.035;
            ((nr + lift).clamp(0.0, 1.0), (ng + lift).clamp(0.0, 1.0), (nb + lift).clamp(0.0, 1.0))
        }
        "nat_soft_day" => {
            // Luz suave diurna: calidez natural, medios tonos abiertos
            let nr = r * 1.04;
            let ng = g * 1.01;
            let nb = b * 0.98;
            let mid = 0.03 * (4.0 * luma * (1.0 - luma));
            ((nr + mid).clamp(0.0, 1.0), (ng + mid).clamp(0.0, 1.0), (nb + mid).clamp(0.0, 1.0))
        }
        "nat_peach_glow" => {
            // Melocotón saludable: tonos cálidos y rosados en piel
            let nr = r * 1.06;
            let ng = g * 0.99;
            let nb = b * 0.96;
            let sat = 1.08;
            let l = 0.2126 * nr + 0.7152 * ng + 0.0722 * nb;
            ((l + (nr - l) * sat).clamp(0.0, 1.0), (l + (ng - l) * sat).clamp(0.0, 1.0), (l + (nb - l) * sat).clamp(0.0, 1.0))
        }
        "nat_japanese" => {
            // Película japonesa: negros ligeramente levantados (toe roll-off), verdes pastel aireados
            let toe = 0.035 * (1.0 - luma).powi(2);
            let nr = r * 1.01 + toe;
            let ng = g * 0.96 + toe;
            let nb = b * 1.04 + toe;
            (nr.clamp(0.0, 1.0), ng.clamp(0.0, 1.0), nb.clamp(0.0, 1.0))
        }
        "nat_ivory_clean" => {
            // Marfil pulido: definición pura, contraste limpio
            let s_curve = |c: f32| c * c * (3.0 - 2.0 * c);
            let nr = r * 0.88 + s_curve(r.clamp(0.0, 1.0)) * 0.12;
            let ng = g * 0.88 + s_curve(g.clamp(0.0, 1.0)) * 0.12;
            let nb = b * 0.88 + s_curve(b.clamp(0.0, 1.0)) * 0.12;
            ((nr * 1.02).clamp(0.0, 1.0), (ng * 1.00).clamp(0.0, 1.0), (nb * 0.99).clamp(0.0, 1.0))
        }
        "nat_morning_dew" => {
            // Rocío matutino: sombras cian frías con altas luces doradas
            let shadow_cool = (1.0 - luma).clamp(0.0, 1.0) * 0.045;
            let high_warm = luma.clamp(0.0, 1.0) * 0.040;
            ((r + high_warm).clamp(0.0, 1.0), (g + shadow_cool * 0.5).clamp(0.0, 1.0), (b + shadow_cool).clamp(0.0, 1.0))
        }

        // ==========================================
        // 2. CATEGORÍA: EMOCIONAL
        // ==========================================
        "emo_golden_sunset" => {
            // Atardecer dorado: rayos ámbar intensos, contraste enriquecido
            let nr = r * 1.16 + 0.04;
            let ng = g * 1.03 + 0.015;
            let nb = b * 0.82;
            let l = 0.2126 * nr + 0.7152 * ng + 0.0722 * nb;
            let sat = 1.15;
            ((l + (nr - l) * sat).clamp(0.0, 1.0), (l + (ng - l) * sat).clamp(0.0, 1.0), (l + (nb - l) * sat).clamp(0.0, 1.0))
        }
        "emo_teal_orange" => {
            // Hollywood Teal & Orange: sombras azul-turquesa, piel naranja cálida
            let sh = (1.0 - luma / 0.5).clamp(0.0, 1.0);
            let hi = ((luma - 0.45) / 0.55).clamp(0.0, 1.0);
            let nr = r + hi * 0.085 - sh * 0.045;
            let ng = g + hi * 0.025 + sh * 0.020;
            let nb = b - hi * 0.055 + sh * 0.090;
            let s_curve = |c: f32| c * c * (3.0 - 2.0 * c);
            (
                (nr * 0.85 + s_curve(nr.clamp(0.0, 1.0)) * 0.15).clamp(0.0, 1.0),
                (ng * 0.85 + s_curve(ng.clamp(0.0, 1.0)) * 0.15).clamp(0.0, 1.0),
                (nb * 0.85 + s_curve(nb.clamp(0.0, 1.0)) * 0.15).clamp(0.0, 1.0)
            )
        }
        "emo_melancholy" => {
            // Melancolía: atmósfera crepuscular lavanda/violeta, tonos sutilmente desaturados
            let sh = (1.0 - luma).clamp(0.0, 1.0) * 0.055;
            let nr = (r + sh * 0.60) * 0.95;
            let ng = (g - sh * 0.20) * 0.92;
            let nb = (b + sh * 1.10) * 1.02;
            let l = 0.2126 * nr + 0.7152 * ng + 0.0722 * nb;
            let sat = 0.88;
            ((l + (nr - l) * sat).clamp(0.0, 1.0), (l + (ng - l) * sat).clamp(0.0, 1.0), (l + (nb - l) * sat).clamp(0.0, 1.0))
        }
        "emo_intimate_dusk" => {
            // Crepúsculo íntimo: tonos vino profundo y sombras envolventes
            let nr = r * 1.10;
            let ng = g * 0.92;
            let nb = b * 0.98;
            let toe = 0.025 * (1.0 - luma);
            ((nr + toe).clamp(0.0, 1.0), ng.clamp(0.0, 1.0), (nb + toe * 0.7).clamp(0.0, 1.0))
        }
        "emo_candlelight" => {
            // Luz de vela: incandescencia pura ámbar/fuego
            let nr = r * 1.22 + 0.05;
            let ng = g * 1.04 + 0.02;
            let nb = b * 0.75;
            (nr.clamp(0.0, 1.0), ng.clamp(0.0, 1.0), nb.clamp(0.0, 1.0))
        }
        "emo_midnight" => {
            // Medianoche: sombras azul profundo nocturno con piel suavemente iluminada
            let sh = (1.0 - luma).clamp(0.0, 1.0) * 0.09;
            let nr = (r - sh * 0.40 + 0.02).clamp(0.0, 1.0);
            let ng = (g + sh * 0.15).clamp(0.0, 1.0);
            let nb = (b + sh * 1.20).clamp(0.0, 1.0);
            (nr, ng, nb)
        }

        // ==========================================
        // 3. CATEGORÍA: VÍVIDO
        // ==========================================
        "viv_pop_glam" => {
            // Pop glam: impacto editorial, saturación y contraste cinematográfico
            let sat = 1.35;
            let nr = luma + (r - luma) * sat;
            let ng = luma + (g - luma) * sat;
            let nb = luma + (b - luma) * sat;
            let s_curve = |c: f32| c * c * (3.0 - 2.0 * c);
            (
                (nr * 0.82 + s_curve(nr.clamp(0.0, 1.0)) * 0.18 * 1.04).clamp(0.0, 1.0),
                (ng * 0.82 + s_curve(ng.clamp(0.0, 1.0)) * 0.18).clamp(0.0, 1.0),
                (nb * 0.82 + s_curve(nb.clamp(0.0, 1.0)) * 0.18 * 1.02).clamp(0.0, 1.0)
            )
        }
        "viv_summer" => {
            // Verano: cielo azul radiante y piel bronceada cálida
            let sat = 1.28;
            let nr = (luma + (r - luma) * sat) * 1.06;
            let ng = (luma + (g - luma) * sat) * 1.01;
            let nb = (luma + (b - luma) * sat) * 1.08;
            (nr.clamp(0.0, 1.0), ng.clamp(0.0, 1.0), nb.clamp(0.0, 1.0))
        }
        "viv_editorial" => {
            // Portada de revista: negros profundos, brillo especular definido
            let s_curve = |c: f32| c * c * (3.0 - 2.0 * c);
            let nr = s_curve(r.clamp(0.0, 1.0));
            let ng = s_curve(g.clamp(0.0, 1.0));
            let nb = s_curve(b.clamp(0.0, 1.0));
            let sat = 1.22;
            let l = 0.2126 * nr + 0.7152 * ng + 0.0722 * nb;
            ((l + (nr - l) * sat).clamp(0.0, 1.0), (l + (ng - l) * sat).clamp(0.0, 1.0), (l + (nb - l) * sat).clamp(0.0, 1.0))
        }
        "viv_candy" => {
            // Candy: magentas dulces y tonos dorados luminosos
            let nr = r * 1.08;
            let ng = g * 0.98;
            let nb = b * 1.08;
            let sat = 1.25;
            let l = 0.2126 * nr + 0.7152 * ng + 0.0722 * nb;
            ((l + (nr - l) * sat).clamp(0.0, 1.0), (l + (ng - l) * sat).clamp(0.0, 1.0), (l + (nb - l) * sat).clamp(0.0, 1.0))
        }
        "viv_tropical" => {
            // Tropical: vegetación exuberante y calidez luminosa
            let nr = r * 1.08;
            let ng = g * 1.07;
            let nb = b * 0.96;
            let sat = 1.26;
            let l = 0.2126 * nr + 0.7152 * ng + 0.0722 * nb;
            ((l + (nr - l) * sat).clamp(0.0, 1.0), (l + (ng - l) * sat).clamp(0.0, 1.0), (l + (nb - l) * sat).clamp(0.0, 1.0))
        }
        "viv_neon" => {
            // Neón Punch: alto contraste ciberpunk vibrante
            let sat = 1.40;
            let nr = (luma + (r - luma) * sat) * 1.05;
            let ng = (luma + (g - luma) * sat) * 1.00;
            let nb = (b + luma - luma) * sat * 1.10;
            let s_curve = |c: f32| c * c * (3.0 - 2.0 * c);
            (
                (nr * 0.75 + s_curve(nr.clamp(0.0, 1.0)) * 0.25).clamp(0.0, 1.0),
                (ng * 0.75 + s_curve(ng.clamp(0.0, 1.0)) * 0.25).clamp(0.0, 1.0),
                (nb * 0.75 + s_curve(nb.clamp(0.0, 1.0)) * 0.25).clamp(0.0, 1.0)
            )
        }

        // ==========================================
        // 4. CATEGORÍA: ONÍRICO (Dreamy / Ethereal)
        // ==========================================
        "oni_fairy_tale" => {
            // Cuento de hadas: niebla esmeralda y dorada sutil, difusión de altas luces
            let bloom = 4.0 * luma * (1.0 - luma) * 0.07;
            let nr = (r * 1.05 + bloom).clamp(0.0, 1.0);
            let ng = (g * 1.03 + bloom * 0.9).clamp(0.0, 1.0);
            let nb = (b * 0.95 + bloom * 0.5).clamp(0.0, 1.0);
            (nr, ng, nb)
        }
        "oni_champagne" => {
            // Champaña: velo dorado cremoso y elegante
            let nr = r * 1.07 + 0.035;
            let ng = g * 1.04 + 0.025;
            let nb = b * 0.94 + 0.010;
            (nr.clamp(0.0, 1.0), ng.clamp(0.0, 1.0), nb.clamp(0.0, 1.0))
        }
        "oni_rose_dream" => {
            // Rosa cuarzo: bruma pastel rosada romántica
            let mid = 4.0 * luma * (1.0 - luma) * 0.06;
            let nr = (r * 1.03 + mid).clamp(0.0, 1.0);
            let ng = (g * 0.98 + mid * 0.4).clamp(0.0, 1.0);
            let nb = (b * 1.04 + mid * 0.8).clamp(0.0, 1.0);
            (nr, ng, nb)
        }
        "oni_angelic" => {
            // Luz angelical: blancos etéreos elevados, halo luminoso suave
            let hi = (luma * luma) * 0.12;
            let nr = (r * 1.02 + hi).clamp(0.0, 1.0);
            let ng = (g * 1.01 + hi).clamp(0.0, 1.0);
            let nb = (b * 1.05 + hi).clamp(0.0, 1.0);
            (nr, ng, nb)
        }
        "oni_mist" => {
            // Niebla suave: negros mate elevados, contraste atenuado
            let toe = 0.055 * (1.0 - luma);
            let nr = (r * 0.94 + toe).clamp(0.0, 1.0);
            let ng = (g * 0.94 + toe).clamp(0.0, 1.0);
            let nb = (b * 0.98 + toe * 1.2).clamp(0.0, 1.0);
            (nr, ng, nb)
        }
        "oni_magic_glow" => {
            // Brillo mágico: resplandor dorado cálido en medios tonos
            let glow = 4.0 * luma * (1.0 - luma) * 0.085;
            let nr = (r + glow * 1.2).clamp(0.0, 1.0);
            let ng = (g + glow * 0.9).clamp(0.0, 1.0);
            let nb = (b + glow * 0.4).clamp(0.0, 1.0);
            (nr, ng, nb)
        }

        // ==========================================
        // 5. CATEGORÍA: VINTAGE
        // ==========================================
        "vin_kodak_portra" => {
            // Kodak Portra 400: calidez analógica, negros aterciopelados
            let toe = 0.032 * (1.0 - luma).powi(2);
            let nr = (r * 1.09 + toe).clamp(0.0, 1.0);
            let ng = (g * 1.02 + toe).clamp(0.0, 1.0);
            let nb = (b * 0.93 + toe * 0.6).clamp(0.0, 1.0);
            (nr, ng, nb)
        }
        "vin_fuji_velvia" => {
            // Fuji Velvia: contraste analógico fuerte, verdes esmeralda y sombras ricas
            let s_curve = |c: f32| c * c * (3.0 - 2.0 * c);
            let nr = s_curve(r.clamp(0.0, 1.0)) * 1.02;
            let ng = s_curve(g.clamp(0.0, 1.0)) * 1.05;
            let nb = s_curve(b.clamp(0.0, 1.0)) * 0.96;
            let sat = 1.18;
            let l = 0.2126 * nr + 0.7152 * ng + 0.0722 * nb;
            ((l + (nr - l) * sat).clamp(0.0, 1.0), (l + (ng - l) * sat).clamp(0.0, 1.0), (l + (nb - l) * sat).clamp(0.0, 1.0))
        }
        "vin_retro_90s" => {
            // Retro 90s: camcorder VHS nostálgico, tonalidad cálida vintage
            let toe = 0.04 * (1.0 - luma);
            let nr = (r * 1.04 + toe).clamp(0.0, 1.0);
            let ng = (g * 1.00 + toe * 0.9).clamp(0.0, 1.0);
            let nb = (b * 0.88 + toe * 0.5).clamp(0.0, 1.0);
            let sat = 0.92;
            let l = 0.2126 * nr + 0.7152 * ng + 0.0722 * nb;
            ((l + (nr - l) * sat).clamp(0.0, 1.0), (l + (ng - l) * sat).clamp(0.0, 1.0), (l + (nb - l) * sat).clamp(0.0, 1.0))
        }
        "vin_noir_classic" => {
            // Blanco y negro Noir clásico: alto contraste monocromático de película argéntica
            let mono = 0.299 * r + 0.587 * g + 0.114 * b;
            let c_mono = mono.clamp(0.0, 1.0);
            let s = c_mono * c_mono * (3.0 - 2.0 * c_mono);
            let final_mono = s.clamp(0.0, 1.0);
            (final_mono, final_mono, final_mono)
        }
        "vin_sepia_antique" => {
            // Sepia del siglo XIX: chocolate y ámbar cálido
            let mono = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 1.0);
            (
                (mono * 1.18).clamp(0.0, 1.0),
                (mono * 0.94).clamp(0.0, 1.0),
                (mono * 0.72).clamp(0.0, 1.0),
            )
        }
        "vin_cyberpunk" => {
            // Cyberpunk 80s: sombras cian profundo y altas luces magenta neón
            let sh = (1.0 - luma).clamp(0.0, 1.0);
            let hi = luma.clamp(0.0, 1.0);
            let nr = (r - sh * 0.06 + hi * 0.14).clamp(0.0, 1.0);
            let ng = (g + sh * 0.05 - hi * 0.03).clamp(0.0, 1.0);
            let nb = (b + sh * 0.14 + hi * 0.06).clamp(0.0, 1.0);
            (nr, ng, nb)
        }
        "vin_polaroid" => {
            // Polaroid 600: instantánea analógica, sombras mate levantadas, tinte cálido
            let toe = 0.045 * (1.0 - luma);
            let nr = (r * 1.06 + toe).clamp(0.0, 1.0);
            let ng = (g * 1.02 + toe * 0.8).clamp(0.0, 1.0);
            let nb = (b * 0.90 + toe * 0.4).clamp(0.0, 1.0);
            (nr, ng, nb)
        }

        _ => (r, g, b),
    }
}
