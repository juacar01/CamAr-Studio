//! Módulo de Fotometría, Medición de Luminancia y Calibración de Exposición.
//!
//! Responsabilidad única (SRP): Métricas fotométricas de luminancia y crominancia ambiental,
//! zona de retrato facial y cálculo de exposición / balance de blancos sugeridos.

use crate::types::{FaceLandmarks, FaceLighting};

#[derive(Clone, Copy, Debug)]
pub struct PhotometricStats {
    pub detected_luma: f32,
    pub auto_exposure: f32,
    pub auto_wb: f32,
    #[allow(dead_code)]
    pub auto_contrast: f32,
    pub recommended_exposure: f32,
    pub recommended_contrast: f32,
    pub recommended_saturation: f32,
    pub recommended_sharpness: f32,
    pub recommended_wb: f32,
}

pub struct PhotometricSample {
    pub avg_luma: f32,
    pub avg_r: f32,
    pub avg_g: f32,
    pub avg_b: f32,
    pub face_sample_count: usize,
    pub face_luma_sum: f32,
}

/// Analiza la luminancia y crominancia ambiental y de la región facial mediante muestreo por pasos.
pub fn analyze_frame_luma(
    input_rgba: &[u8],
    width: usize,
    height: usize,
    faces: &[FaceLandmarks],
) -> PhotometricSample {
    let active_face = faces.iter().find(|f| f.detected && f.bounding_box.width > 20.0);
    let (face_min_x, face_max_x, face_min_y, face_max_y) = if let Some(face) = active_face {
        let bb = &face.bounding_box;
        let margin_x = bb.width * 0.15;
        let margin_y = bb.height * 0.15;
        (bb.x + margin_x, bb.x + bb.width - margin_x, bb.y + margin_y, bb.y + bb.height - margin_y)
    } else {
        (-1.0, -1.0, -1.0, -1.0)
    };

    let sample_step = 16;
    let mut sample_count = 0;
    let mut total_luma = 0.0_f32;
    let mut total_r = 0.0_f32;
    let mut total_g = 0.0_f32;
    let mut total_b = 0.0_f32;

    let mut face_sample_count = 0;
    let mut face_luma_sum = 0.0_f32;

    for y in (0..height).step_by(sample_step) {
        let row_offset = y * width * 4;
        let fy = y as f32;
        for x in (0..width).step_by(sample_step) {
            let idx = row_offset + x * 4;
            if idx + 2 < input_rgba.len() {
                let r = input_rgba[idx] as f32 / 255.0;
                let g = input_rgba[idx + 1] as f32 / 255.0;
                let b = input_rgba[idx + 2] as f32 / 255.0;
                let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                total_luma += luma;
                total_r += r;
                total_g += g;
                total_b += b;
                sample_count += 1;

                let fx = x as f32;
                if fx >= face_min_x && fx <= face_max_x && fy >= face_min_y && fy <= face_max_y {
                    face_luma_sum += luma;
                    face_sample_count += 1;
                }
            }
        }
    }

    let avg_luma = if sample_count > 0 { total_luma / sample_count as f32 } else { 0.42 };
    let avg_r = if sample_count > 0 { total_r / sample_count as f32 } else { 0.42 };
    let avg_g = if sample_count > 0 { total_g / sample_count as f32 } else { 0.42 };
    let avg_b = if sample_count > 0 { total_b / sample_count as f32 } else { 0.42 };

    PhotometricSample {
        avg_luma,
        avg_r,
        avg_g,
        avg_b,
        face_sample_count,
        face_luma_sum,
    }
}

/// Calcula los parámetros de auto-exposición y balance de blancos objetivo (Gray-World y Portrait Zone VI).
pub fn compute_target_lighting(
    sample: &PhotometricSample,
    current_luma: f32,
    face_lighting: Option<&FaceLighting>,
) -> (f32, f32, f32, f32, f32) {
    if let Some(fl) = face_lighting.filter(|l| l.has_face) {
        let face_luma = fl.face_luma.clamp(0.05, 0.95);
        let target_skin_luma = 0.53_f32;
        let measured = (face_luma * 0.85 + current_luma * 0.15).max(0.08);
        let exp = (target_skin_luma / measured).log2().clamp(-0.40, 1.10);

        let avg_scene = (fl.scene_r + fl.scene_g + fl.scene_b) / 3.0;
        let wb_r = (avg_scene / fl.scene_r.max(0.05)).clamp(0.92, 1.08);
        let wb_g = (avg_scene / fl.scene_g.max(0.05)).clamp(0.95, 1.05);
        let wb_b = (avg_scene / fl.scene_b.max(0.05)).clamp(0.92, 1.08);

        (exp, wb_r, wb_g, wb_b, face_luma)
    } else if sample.face_sample_count > 4 {
        let face_luma = sample.face_luma_sum / sample.face_sample_count as f32;
        let target_skin_luma = 0.53_f32;
        let measured = (face_luma * 0.85 + current_luma * 0.15).max(0.08);
        let exp = (target_skin_luma / measured).log2().clamp(-0.40, 1.10);

        let avg_scene = (sample.avg_r + sample.avg_g + sample.avg_b) / 3.0;
        let wb_r = (avg_scene / sample.avg_r.max(0.05)).clamp(0.92, 1.08);
        let wb_g = (avg_scene / sample.avg_g.max(0.05)).clamp(0.95, 1.05);
        let wb_b = (avg_scene / sample.avg_b.max(0.05)).clamp(0.92, 1.08);

        (exp, wb_r, wb_g, wb_b, face_luma)
    } else {
        let target_luma = 0.48_f32;
        let measured = current_luma.max(0.08);
        let exp = (target_luma / measured).log2().clamp(-0.35, 0.80);
        let avg_scene = (sample.avg_r + sample.avg_g + sample.avg_b) / 3.0;
        let wb_r = (avg_scene / sample.avg_r.max(0.05)).clamp(0.92, 1.08);
        let wb_g = (avg_scene / sample.avg_g.max(0.05)).clamp(0.95, 1.05);
        let wb_b = (avg_scene / sample.avg_b.max(0.05)).clamp(0.92, 1.08);
        (exp, wb_r, wb_g, wb_b, current_luma)
    }
}
