//! Módulo de Ojos y Boca Agrandados (Face Inset / Funny Face Effect).
//!
//! Responsabilidad única: Extracción dinámica e independiente de rasgos faciales (boca, ojo izq, ojo der)
//! con cobertura anatómica adaptativa al hablar y gesticular, separación ocular real y
//! composición bilineal con mezcla alfa difuminada (feathering).

use crate::types::Point2f;

#[derive(Clone, Copy, Debug)]
pub struct FeaturePatch {
    pub src_cx: f32,
    pub src_cy: f32,
    pub dst_cx: f32,
    pub dst_cy: f32,
    pub radius_x: f32,
    pub radius_y_top: f32,
    pub radius_y_bot: f32,
    pub angle: f32,
    pub scale: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct FaceInsetParams {
    pub left_eye: FeaturePatch,
    pub right_eye: FeaturePatch,
    pub mouth: FeaturePatch,
    pub intensity: f32,
}

#[derive(Clone, Copy, Debug)]
struct SourceInfo {
    min_x: usize,
    min_y: usize,
    src_w: usize,
    src_h: usize,
}

/// Precomputa los parámetros anatómicos dinámicos para los 3 rasgos faciales.
pub fn compute_face_inset_params(
    landmarks: &[Point2f],
    mouth_scale: f32,
    eyes_scale: f32,
    eyes_distance: f32,
    intensity: f32,
) -> Option<FaceInsetParams> {
    if landmarks.len() < 468 || intensity <= 0.001 {
        return None;
    }

    // --- 1. OBJETO BOCA (Centro focal dinámico y dilatación anatómica) ---
    let p_61 = landmarks[61];   // Comisura izquierda
    let p_291 = landmarks[291]; // Comisura derecha
    let p_0 = landmarks[0];     // Labio superior exterior
    let p_17 = landmarks[17];   // Labio inferior exterior
    let p_13 = landmarks[13];   // Labio superior interior
    let p_14 = landmarks[14];   // Labio inferior interior

    let mouth_cx = (p_61.x + p_291.x) * 0.5;
    // El centro vertical acompaña la apertura de la cavidad bucal al hablar
    let mouth_cy = (p_13.y + p_14.y) * 0.5;
    let mouth_dx = p_291.x - p_61.x;
    let mouth_dy = p_291.y - p_61.y;
    let mouth_w = (mouth_dx * mouth_dx + mouth_dy * mouth_dy).sqrt().max(10.0);
    let mouth_angle = mouth_dy.atan2(mouth_dx);

    let mouth_rx = (mouth_w * 0.5 * 1.35).max(15.0);
    let mouth_ry_top = ((mouth_cy - p_0.y).abs() * 1.45).max(12.0);
    // Margen inferior adaptativo que desciende con la mandíbula para no cortarse jamás
    let mouth_ry_bot = ((p_17.y - mouth_cy).abs() * 1.55).max(14.0);

    let mouth_patch = FeaturePatch {
        src_cx: mouth_cx,
        src_cy: mouth_cy,
        dst_cx: mouth_cx,
        dst_cy: mouth_cy,
        radius_x: mouth_rx,
        radius_y_top: mouth_ry_top,
        radius_y_bot: mouth_ry_bot,
        angle: mouth_angle,
        scale: mouth_scale.clamp(0.5, 4.0),
    };

    // --- 2. OBJETO OJO IZQUIERDO (Visor Izq) ---
    let p_33 = landmarks[33];   // Comisura externa
    let p_133 = landmarks[133]; // Comisura interna
    let p_159 = landmarks[159]; // Párpado superior
    let p_145 = landmarks[145]; // Párpado inferior

    // Centro geométrico real del ojo entre comisuras y párpados
    let eye_l_cx = (p_33.x + p_133.x) * 0.5;
    let eye_l_cy = (p_159.y + p_145.y) * 0.5;
    let eye_l_dx = p_133.x - p_33.x;
    let eye_l_dy = p_133.y - p_33.y;
    let eye_l_w = (eye_l_dx * eye_l_dx + eye_l_dy * eye_l_dy).sqrt().max(8.0);
    let eye_l_angle = eye_l_dy.atan2(eye_l_dx);

    let eye_l_rx = (eye_l_w * 0.5 * 1.35).max(10.0);
    let eye_l_ry_top = ((eye_l_cy - p_159.y).abs() * 1.85).max(eye_l_w * 0.40);
    let eye_l_ry_bot = ((p_145.y - eye_l_cy).abs() * 1.85).max(eye_l_w * 0.40);

    // --- 3. OBJETO OJO DERECHO (Visor Der) ---
    let p_263 = landmarks[263]; // Comisura externa
    let p_362 = landmarks[362]; // Comisura interna
    let p_386 = landmarks[386]; // Párpado superior
    let p_374 = landmarks[374]; // Párpado inferior

    // Centro geométrico real del ojo derecho
    let eye_r_cx = (p_263.x + p_362.x) * 0.5;
    let eye_r_cy = (p_386.y + p_374.y) * 0.5;
    let eye_r_dx = p_263.x - p_362.x;
    let eye_r_dy = p_263.y - p_362.y;
    let eye_r_w = (eye_r_dx * eye_r_dx + eye_r_dy * eye_r_dy).sqrt().max(8.0);
    let eye_r_angle = eye_r_dy.atan2(eye_r_dx);

    let eye_r_rx = (eye_r_w * 0.5 * 1.35).max(10.0);
    let eye_r_ry_top = ((eye_r_cy - p_386.y).abs() * 1.85).max(eye_r_w * 0.40);
    let eye_r_ry_bot = ((p_374.y - eye_r_cy).abs() * 1.85).max(eye_r_w * 0.40);

    // --- SEPARACIÓN OCULAR DINÁMICA SOBRE LOS OBJETOS DE OJO ---
    // Se desplazan simétricamente a lo largo del vector inter-ocular
    let eye_mid_x = (eye_l_cx + eye_r_cx) * 0.5;
    let eye_mid_y = (eye_l_cy + eye_r_cy) * 0.5;
    let eye_vec_x = eye_r_cx - eye_l_cx;
    let eye_vec_y = eye_r_cy - eye_l_cy;
    let dist_factor = eyes_distance.clamp(0.5, 2.5);

    let dst_eye_l_x = eye_mid_x - (eye_vec_x * 0.5) * dist_factor;
    let dst_eye_l_y = eye_mid_y - (eye_vec_y * 0.5) * dist_factor;

    let dst_eye_r_x = eye_mid_x + (eye_vec_x * 0.5) * dist_factor;
    let dst_eye_r_y = eye_mid_y + (eye_vec_y * 0.5) * dist_factor;

    let left_patch = FeaturePatch {
        src_cx: eye_l_cx,
        src_cy: eye_l_cy,
        dst_cx: dst_eye_l_x,
        dst_cy: dst_eye_l_y,
        radius_x: eye_l_rx,
        radius_y_top: eye_l_ry_top,
        radius_y_bot: eye_l_ry_bot,
        angle: eye_l_angle,
        scale: eyes_scale.clamp(0.5, 3.5),
    };

    let right_patch = FeaturePatch {
        src_cx: eye_r_cx,
        src_cy: eye_r_cy,
        dst_cx: dst_eye_r_x,
        dst_cy: dst_eye_r_y,
        radius_x: eye_r_rx,
        radius_y_top: eye_r_ry_top,
        radius_y_bot: eye_r_ry_bot,
        angle: eye_r_angle,
        scale: eyes_scale.clamp(0.5, 3.5),
    };

    Some(FaceInsetParams {
        left_eye: left_patch,
        right_eye: right_patch,
        mouth: mouth_patch,
        intensity: intensity.clamp(0.0, 1.0),
    })
}

/// Extrae el parche original de un objeto facial desde el frame sin modificar.
fn extract_source_patch(
    feat: &FeaturePatch,
    buffer: &[u8],
    width: usize,
    height: usize,
) -> Option<(Vec<u8>, SourceInfo)> {
    let src_min_x = (feat.src_cx - feat.radius_x * 1.25).max(0.0) as usize;
    let src_max_x = (feat.src_cx + feat.radius_x * 1.25).min((width - 1) as f32) as usize;
    let src_min_y = (feat.src_cy - feat.radius_y_top * 1.25).max(0.0) as usize;
    let src_max_y = (feat.src_cy + feat.radius_y_bot * 1.25).min((height - 1) as f32) as usize;

    if src_max_x <= src_min_x || src_max_y <= src_min_y {
        return None;
    }

    let src_w = src_max_x - src_min_x + 1;
    let src_h = src_max_y - src_min_y + 1;
    let mut pixels = vec![0u8; src_w * src_h * 4];

    for y in 0..src_h {
        let buf_y = src_min_y + y;
        let buf_row_start = (buf_y * width + src_min_x) * 4;
        let src_row_start = y * src_w * 4;
        pixels[src_row_start..src_row_start + src_w * 4]
            .copy_from_slice(&buffer[buf_row_start..buf_row_start + src_w * 4]);
    }

    Some((
        pixels,
        SourceInfo {
            min_x: src_min_x,
            min_y: src_min_y,
            src_w,
            src_h,
        },
    ))
}

/// Compone un objeto facial escalado sobre la posición de destino usando su parche limpio.
fn composite_feature(
    feat: &FeaturePatch,
    src_pixels: &[u8],
    info: SourceInfo,
    intensity: f32,
    buffer: &mut [u8],
    width: usize,
    height: usize,
) {
    let scale = feat.scale;
    let is_shifted = (feat.dst_cx - feat.src_cx).abs() > 0.5 || (feat.dst_cy - feat.src_cy).abs() > 0.5;
    if (scale - 1.0).abs() < 0.005 && !is_shifted {
        return;
    }

    let src_w = info.src_w;
    let src_h = info.src_h;
    let src_min_x = info.min_x;
    let src_min_y = info.min_y;

    // Bounding Box de destino escalada alrededor de dst_cx / dst_cy
    let dst_min_x = (feat.dst_cx - feat.radius_x * scale * 1.08).max(0.0) as usize;
    let dst_max_x = (feat.dst_cx + feat.radius_x * scale * 1.08).min((width - 1) as f32) as usize;
    let dst_min_y = (feat.dst_cy - feat.radius_y_top * scale * 1.08).max(0.0) as usize;
    let dst_max_y = (feat.dst_cy + feat.radius_y_bot * scale * 1.08).min((height - 1) as f32) as usize;

    if dst_max_x <= dst_min_x || dst_max_y <= dst_min_y {
        return;
    }

    let cos_a = feat.angle.cos();
    let sin_a = feat.angle.sin();
    let inv_scale = 1.0 / scale;
    let inv_rx = 1.0 / feat.radius_x;

    for dy in dst_min_y..=dst_max_y {
        let row_offset = dy * width * 4;
        let ty = dy as f32 - feat.dst_cy;

        for dx in dst_min_x..=dst_max_x {
            let tx = dx as f32 - feat.dst_cx;

            // Deshacer rotación hacia el espacio canónico
            let rx = tx * cos_a + ty * sin_a;
            let ry = -tx * sin_a + ty * cos_a;

            // Des-escalar al tamaño original del rasgo
            let sx_local = rx * inv_scale;
            let sy_local = ry * inv_scale;

            // Métrica radial normalizada con radio superior e inferior independiente
            let inv_ry = if sy_local >= 0.0 {
                1.0 / feat.radius_y_bot
            } else {
                1.0 / feat.radius_y_top
            };

            let nx = sx_local * inv_rx;
            let ny = sy_local * inv_ry;
            let norm_dist = (nx * nx + ny * ny).sqrt();

            if norm_dist >= 1.0 {
                continue; // Fuera del área de cobertura
            }

            // Feathering suave: 100% opaco hasta 0.68, transición cúbica suave hacia 1.0
            let alpha = if norm_dist <= 0.68 {
                1.0
            } else {
                let t = (norm_dist - 0.68) / 0.32;
                1.0 - t * t * (3.0 - 2.0 * t)
            } * intensity;

            if alpha <= 0.001 {
                continue;
            }

            // Reconstruir coordenadas fuente absolutas
            let abs_sx = feat.src_cx + (sx_local * cos_a - sy_local * sin_a);
            let abs_sy = feat.src_cy + (sx_local * sin_a + sy_local * cos_a);

            // Muestreo bilineal dentro del parche fuente
            let local_sx = abs_sx - src_min_x as f32;
            let local_sy = abs_sy - src_min_y as f32;

            if local_sx < 0.0 || local_sx >= (src_w - 1) as f32 || local_sy < 0.0 || local_sy >= (src_h - 1) as f32 {
                continue;
            }

            let x0 = local_sx.floor() as usize;
            let y0 = local_sy.floor() as usize;
            let x1 = (x0 + 1).min(src_w - 1);
            let y1 = (y0 + 1).min(src_h - 1);

            let fx = local_sx - x0 as f32;
            let fy = local_sy - y0 as f32;

            let idx00 = (y0 * src_w + x0) * 4;
            let idx10 = (y0 * src_w + x1) * 4;
            let idx01 = (y1 * src_w + x0) * 4;
            let idx11 = (y1 * src_w + x1) * 4;

            let w00 = (1.0 - fx) * (1.0 - fy);
            let w10 = fx * (1.0 - fy);
            let w01 = (1.0 - fx) * fy;
            let w11 = fx * fy;

            let sr = src_pixels[idx00] as f32 * w00
                + src_pixels[idx10] as f32 * w10
                + src_pixels[idx01] as f32 * w01
                + src_pixels[idx11] as f32 * w11;
            let sg = src_pixels[idx00 + 1] as f32 * w00
                + src_pixels[idx10 + 1] as f32 * w10
                + src_pixels[idx01 + 1] as f32 * w01
                + src_pixels[idx11 + 1] as f32 * w11;
            let sb = src_pixels[idx00 + 2] as f32 * w00
                + src_pixels[idx10 + 2] as f32 * w10
                + src_pixels[idx01 + 2] as f32 * w01
                + src_pixels[idx11 + 2] as f32 * w11;

            let dst_idx = row_offset + dx * 4;
            let dr = buffer[dst_idx] as f32;
            let dg = buffer[dst_idx + 1] as f32;
            let db = buffer[dst_idx + 2] as f32;

            buffer[dst_idx] = (sr * alpha + dr * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
            buffer[dst_idx + 1] = (sg * alpha + dg * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
            buffer[dst_idx + 2] = (sb * alpha + db * (1.0 - alpha)).clamp(0.0, 255.0) as u8;
        }
    }
}

/// Renderiza los rasgos faciales independizados sobre el búfer de salida.
pub fn render_face_inset(
    params: &FaceInsetParams,
    buffer: &mut [u8],
    width: usize,
    height: usize,
) {
    if params.intensity <= 0.001 {
        return;
    }

    // 1. Extraemos los parches fuente limpios de cada objeto ANTES de dibujar nada
    let left_src = extract_source_patch(&params.left_eye, buffer, width, height);
    let right_src = extract_source_patch(&params.right_eye, buffer, width, height);
    let mouth_src = extract_source_patch(&params.mouth, buffer, width, height);

    // 2. Componemos en orden: Ojo izquierdo y Ojo derecho sobre sus destinos, luego la Boca en primer plano
    if let Some((pixels, info)) = left_src {
        composite_feature(&params.left_eye, &pixels, info, params.intensity, buffer, width, height);
    }
    if let Some((pixels, info)) = right_src {
        composite_feature(&params.right_eye, &pixels, info, params.intensity, buffer, width, height);
    }
    if let Some((pixels, info)) = mouth_src {
        composite_feature(&params.mouth, &pixels, info, params.intensity, buffer, width, height);
    }
}
