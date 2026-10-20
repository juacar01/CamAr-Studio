//! Módulo de Gestión y Muestreo Sub-píxel de Sprites RGBA.
//!
//! Responsabilidad única (SRP): Carga de texturas desde disco o recursos embebidos,
//! e interpolación bilineal sub-píxel continua en texturas 2D.

/// Carga un Sprite RGBA (.png) de dimensiones (target_w, target_h) desde disco
/// (`sprites/{folder}/{name}.png`) o desde el buffer binario embebido si el archivo no existe.
pub fn load_rgba_sprite(
    folder: &str,
    name: &str,
    embedded_png: &[u8],
    target_w: usize,
    target_h: usize,
) -> Vec<[f32; 4]> {
    let mut disk_img = None;
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let candidate = parent.join("sprites").join(folder).join(format!("{}.png", name));
            if candidate.exists() {
                if let Ok(img) = image::open(&candidate) {
                    disk_img = Some(img);
                }
            }
        }
    }
    let dyn_img = disk_img.or_else(|| image::load_from_memory(embedded_png).ok());
    let mut buf = vec![[0.0f32; 4]; target_w * target_h];
    if let Some(img) = dyn_img {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut non_zero_alpha = 0;
        for y in 0..target_h {
            let sy = ((y * h as usize) / target_h).min(h.saturating_sub(1) as usize) as u32;
            for x in 0..target_w {
                let sx = ((x * w as usize) / target_w).min(w.saturating_sub(1) as usize) as u32;
                let px = rgba.get_pixel(sx, sy).0;
                let a = px[3] as f32 / 255.0;
                if a > 0.01 { non_zero_alpha += 1; }
                buf[y * target_w + x] = [
                    px[0] as f32 / 255.0,
                    px[1] as f32 / 255.0,
                    px[2] as f32 / 255.0,
                    a,
                ];
            }
        }
        crate::log_info!("Sprites", "Sprite '{}/{}' cargado ({}x{}, {} píxeles visibles)", folder, name, target_w, target_h, non_zero_alpha);
    } else {
        crate::log_error!("Sprites", "ERROR: Falló carga de sprite '{}/{}'", folder, name);
    }
    buf
}

/// Muestreo bilineal sub-píxel continuo para Sprites RGBA 256x256 de Pupilentes y Sombras.
#[inline(always)]
pub fn sample_sprite_bilinear_256(sprite: &[[f32; 4]], fx: f32, fy: f32) -> [f32; 4] {
    let x = fx.clamp(0.0, 255.0);
    let y = fy.clamp(0.0, 255.0);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(255);
    let y1 = (y0 + 1).min(255);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;

    let c00 = sprite[y0 * 256 + x0];
    let c10 = sprite[y0 * 256 + x1];
    let c01 = sprite[y1 * 256 + x0];
    let c11 = sprite[y1 * 256 + x1];

    let mut out = [0.0f32; 4];
    for c in 0..4 {
        let top = c00[c] * (1.0 - tx) + c10[c] * tx;
        let bot = c01[c] * (1.0 - tx) + c11[c] * tx;
        out[c] = top * (1.0 - ty) + bot * ty;
    }
    out
}

/// Muestreo bilineal sub-píxel continuo para el Sprite Deformable de Labios 128x64 (0..31 upper, 32..63 lower).
#[inline(always)]
pub fn sample_lip_sprite_bilinear(
    sprite: &[[f32; 4]],
    u_norm: f32,
    v_norm: f32,
    row_offset: usize,
) -> [f32; 4] {
    let fx = (u_norm * 127.0).clamp(0.0, 127.0);
    let fy = (v_norm * 31.0).clamp(0.0, 31.0);
    let x0 = fx.floor() as usize;
    let y0 = fy.floor() as usize;
    let x1 = (x0 + 1).min(127);
    let y1 = (y0 + 1).min(31);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;

    let r0 = row_offset + y0;
    let r1 = row_offset + y1;
    let c00 = sprite[r0 * 128 + x0];
    let c10 = sprite[r0 * 128 + x1];
    let c01 = sprite[r1 * 128 + x0];
    let c11 = sprite[r1 * 128 + x1];

    let mut out = [0.0f32; 4];
    for c in 0..4 {
        let top = c00[c] * (1.0 - tx) + c10[c] * tx;
        let bot = c01[c] * (1.0 - tx) + c11[c] * tx;
        out[c] = top * (1.0 - ty) + bot * ty;
    }
    out
}

/// Muestreo bilineal sub-píxel continuo para Sprites RGBA 256x128 de Cejas con orientación UV.
#[inline(always)]
#[allow(dead_code)]
pub fn sample_eyebrow_sprite_bilinear(sprite: &[[f32; 4]], u_norm: f32, v_norm: f32) -> [f32; 4] {
    let fx = (u_norm.clamp(0.0, 1.0) * 255.0).clamp(0.0, 255.0);
    let fy = ((1.0 - v_norm.clamp(0.0, 1.0)) * 127.0).clamp(0.0, 127.0);
    let x0 = fx.floor() as usize;
    let y0 = fy.floor() as usize;
    let x1 = (x0 + 1).min(255);
    let y1 = (y0 + 1).min(127);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;

    let c00 = sprite[y0 * 256 + x0];
    let c10 = sprite[y0 * 256 + x1];
    let c01 = sprite[y1 * 256 + x0];
    let c11 = sprite[y1 * 256 + x1];

    let mut out = [0.0f32; 4];
    for c in 0..4 {
        let top = c00[c] * (1.0 - tx) + c10[c] * tx;
        let bot = c01[c] * (1.0 - tx) + c11[c] * tx;
        out[c] = top * (1.0 - ty) + bot * ty;
    }
    out
}

/// Muestreo bilineal sub-píxel continuo para Texturas Maestras RGBA 1024x1024 de Máscara Facial 3D.
#[inline(always)]
pub fn sample_face_mask_bilinear_1024(sprite: &[[f32; 4]], u: f32, v: f32) -> [f32; 4] {
    let fx = (u.clamp(0.0, 1.0) * 1023.0).clamp(0.0, 1023.0);
    let fy = ((1.0 - v.clamp(0.0, 1.0)) * 1023.0).clamp(0.0, 1023.0);
    let x0 = fx.floor() as usize;
    let y0 = fy.floor() as usize;
    let x1 = (x0 + 1).min(1023);
    let y1 = (y0 + 1).min(1023);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;

    let c00 = sprite[y0 * 1024 + x0];
    let c10 = sprite[y0 * 1024 + x1];
    let c01 = sprite[y1 * 1024 + x0];
    let c11 = sprite[y1 * 1024 + x1];

    let mut out = [0.0f32; 4];
    for c in 0..4 {
        let top = c00[c] * (1.0 - tx) + c10[c] * tx;
        let bot = c01[c] * (1.0 - tx) + c11[c] * tx;
        out[c] = top * (1.0 - ty) + bot * ty;
    }
    out
}

/// Pre-generación procedimental del Sprite 128x128 de soporte (Luz de Iris sin color).
pub fn generate_default_contact_lens_sprite() -> Vec<[f32; 4]> {
    let mut contact_lens_sprite = vec![[0.0f32; 4]; 128 * 128];
    for iy in 0..128 {
        let v = (iy as f32 - 63.5) / 60.0;
        for ix in 0..128 {
            let u = (ix as f32 - 63.5) / 60.0;
            let r = (u * u + v * v).sqrt();
            if r <= 1.03 {
                let theta = v.atan2(u);
                let t_pupil = ((r - 0.38) / 0.10).clamp(0.0, 1.0);
                let pupil_open = t_pupil * t_pupil * (3.0 - 2.0 * t_pupil);
                let t_outer = ((1.02 - r) / 0.10).clamp(0.0, 1.0);
                let outer_edge = t_outer * t_outer * (3.0 - 2.0 * t_outer);
                let lens_alpha = pupil_open * outer_edge;

                let f1 = (theta * 28.0 + (r * 6.5).sin() * 0.35).sin();
                let f2 = (theta * 52.0 - (r * 9.0).cos() * 0.28).cos();
                let f3 = (theta * 84.0 + r * 4.0).sin();
                let radial_spokes = (0.50 + 0.28 * f1 + 0.14 * f2 + 0.08 * f3).clamp(0.10, 1.0);

                let ring_t = ((r - 0.38) / 0.56).clamp(0.0, 1.0);
                let ring_bell = (ring_t * std::f32::consts::PI).sin();
                let fiber_lum = (radial_spokes * (0.42 + 0.58 * ring_bell)).clamp(0.0, 1.0);

                let col_t = (1.0 - ((r - 0.46).abs() / 0.08)).clamp(0.0, 1.0);
                let collarette = col_t * col_t * (0.65 + 0.35 * f1.abs());

                let limb_in = ((r - 0.82) / 0.12).clamp(0.0, 1.0);
                let limb_dots = 0.78 + 0.22 * (theta * 64.0).cos();
                let limbal_darkness = (limb_in * limb_in * outer_edge * limb_dots).clamp(0.0, 1.0);

                contact_lens_sprite[iy * 128 + ix] = [fiber_lum, collarette, limbal_darkness, lens_alpha];
            }
        }
    }
    contact_lens_sprite
}

/// Pre-generación procedimental del Sprite 128x64 de Labios (Deformable en 4 curvas).
pub fn generate_default_lip_sprite() -> Vec<[f32; 4]> {
    let mut lip_sprite = vec![[0.0f32; 4]; 128 * 64];
    for iy in 0..64 {
        let is_upper = iy < 32;
        let v_local = if is_upper {
            iy as f32 / 31.0
        } else {
            (iy - 32) as f32 / 31.0
        };

        for ix in 0..128 {
            let u = ix as f32 / 127.0;
            let u_sin = (u * std::f32::consts::PI).sin();
            let u_env = u_sin.powf(0.50);

            let (out_feather, liner_t) = if is_upper {
                let t_out = (v_local / 0.28).clamp(0.0, 1.0);
                let l_t = (1.0 - ((v_local - 0.16).abs() / 0.14)).clamp(0.0, 1.0);
                (t_out * t_out * (3.0 - 2.0 * t_out), l_t * l_t)
            } else {
                let t_out = ((1.0 - v_local) / 0.30).clamp(0.0, 1.0);
                let l_t = (1.0 - ((v_local - 0.82).abs() / 0.14)).clamp(0.0, 1.0);
                (t_out * t_out * (3.0 - 2.0 * t_out), l_t * l_t)
            };

            let v_bell = (v_local * std::f32::consts::PI).sin();
            let outer_alpha = (u_env * out_feather * (0.76 + 0.24 * v_bell)).clamp(0.0, 1.0);

            let crease1 = (u * 38.0 * std::f32::consts::PI).sin();
            let crease2 = (u * 74.0 * std::f32::consts::PI + 0.6).cos();
            let sulci = 0.93 + (0.045 * crease1 + 0.020 * crease2) * v_bell;
            let cushion_3d = (0.84 + 0.22 * v_bell * u_sin) * sulci;

            let gloss_spec = if !is_upper {
                let gx = (1.0 - ((u - 0.50).abs() / 0.26)).clamp(0.0, 1.0);
                let gy = (1.0 - ((v_local - 0.40).abs() / 0.20)).clamp(0.0, 1.0);
                let bead = (0.65 + 0.35 * crease1).max(0.0);
                (gx * gx * gy * gy * bead).clamp(0.0, 1.0)
            } else {
                let gx = (1.0 - ((u - 0.50).abs() / 0.18)).clamp(0.0, 1.0);
                let gy = (1.0 - ((v_local - 0.35).abs() / 0.18)).clamp(0.0, 1.0);
                (gx * gx * gy * gy * 0.45).clamp(0.0, 1.0)
            };

            let liner_shade = (liner_t * u_env * 0.85).clamp(0.0, 1.0);

            lip_sprite[iy * 128 + ix] = [outer_alpha, cushion_3d, gloss_spec, liner_shade];
        }
    }
    lip_sprite
}
