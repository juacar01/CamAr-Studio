//! Generador y Rasterizador de Sub-Mallas 3D Anatómicas Adaptadas por Tono.
//!
//! Sub-mallas desacopladas e independientes:
//! 1. Ojos (Izquierdo y Derecho): párpados, esclera, iris 3D con recorte palpebral exacto.
//! 2. Cejas (Izquierda y Derecha): región protegida con factor de atenuación de maquillaje.
//! 3. Boca y Labios: contorno exterior, interior, apertura continua y hueco dental.
//! 4. Piel Facial + Extensión Adaptativa de Frente: Proyección de vértices 3D
//!    guiada por el gradiente de contraste tonal (contraste con el cabello).

use crate::types::Point2f;
use super::color::rgb_to_ycbcr;
use super::geometry::{sample_lid_y, sample_lip_y};

/// Bounding Box 2D con extensión anatómica
#[derive(Debug, Clone, Copy, Default)]
pub struct SubMeshBounds {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

impl SubMeshBounds {
    #[inline(always)]
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.min_x && px <= self.max_x && py >= self.min_y && py <= self.max_y
    }
}

/// Sub-Malla 3D Anatómica de Ojo y Párpados (Izquierdo / Derecho)
#[derive(Debug, Clone)]
pub struct EyeSubMesh {
    pub is_left: bool,
    pub pupil_center: (f32, f32),
    pub iris_radius: f32,
    pub eye_width: f32,
    pub eye_height: f32,
    pub bounds: SubMeshBounds,
    pub upper_points: [(f32, f32); 9],
    pub lower_points: [(f32, f32); 9],
}

impl EyeSubMesh {
    /// Evalúa la apertura palpebral y antialiasing en (px, py).
    /// Retorna Some(alpha_factor) si el punto está dentro de la fisura del ojo.
    #[inline(always)]
    pub fn evaluate_lid_clipping(&self, px: f32, py: f32) -> Option<f32> {
        let left_x = self.upper_points[0].0.min(self.lower_points[0].0);
        let right_x = self.upper_points[8].0.max(self.lower_points[8].0);
        if px < left_x || px > right_x {
            return None;
        }

        let y_top = sample_lid_y(&self.upper_points, px);
        let y_bot = sample_lid_y(&self.lower_points, px);

        if py >= y_top && py <= y_bot {
            // Antialiasing sub-píxel en los bordes de los párpados
            let ap_top = (py - y_top).clamp(0.0, 1.0);
            let ap_bot = (y_bot - py).clamp(0.0, 1.0);
            Some(ap_top * ap_bot)
        } else {
            None
        }
    }
}

/// Sub-Malla 3D Anatómica de Labios y Boca
#[derive(Debug, Clone)]
pub struct LipsSubMesh {
    pub bounds: SubMeshBounds,
    pub left_x: f32,
    pub right_x: f32,
    pub inv_width: f32,
    pub outer_top: [(f32, f32); 11],
    pub inner_top: [(f32, f32); 11],
    pub inner_bot: [(f32, f32); 11],
    pub outer_bot: [(f32, f32); 11],
}

impl LipsSubMesh {
    /// Determina si un píxel (px, py) cae en el labio superior, labio inferior o hueco dental
    #[inline(always)]
    pub fn evaluate_lip_region(&self, px: f32, py: f32) -> Option<(f32, bool, f32)> {
        if px < self.left_x || px > self.right_x {
            return None;
        }

        let y_ot = sample_lip_y(&self.outer_top, px);
        let y_it = sample_lip_y(&self.inner_top, px).max(y_ot + 0.5);
        let y_ib = sample_lip_y(&self.inner_bot, px).max(y_it);
        let y_ob = sample_lip_y(&self.outer_bot, px).max(y_ib + 0.5);

        let mouth_gap = (y_ib - y_it).max(0.0);
        let mouth_open_t = ((mouth_gap - 1.6) / 3.2).clamp(0.0, 1.0);
        let y_mid = 0.5 * (y_it + y_ib);
        let y_it_eff = y_mid * (1.0 - mouth_open_t) + y_it * mouth_open_t;
        let y_ib_eff = y_mid * (1.0 - mouth_open_t) + y_ib * mouth_open_t;

        if py >= y_ot && py <= y_it_eff {
            let v = ((py - y_ot) / (y_it_eff - y_ot).max(0.5)).clamp(0.0, 1.0);
            Some((v, true, mouth_open_t)) // true = labio superior
        } else if py >= y_ib_eff && py <= y_ob {
            let v = ((py - y_ib_eff) / (y_ob - y_ib_eff).max(0.5)).clamp(0.0, 1.0);
            Some((v, false, mouth_open_t)) // false = labio inferior
        } else {
            None
        }
    }
}

/// Sub-Malla 3D de Protección Anatómica de Cejas
#[derive(Debug, Clone)]
pub struct EyebrowSubMesh {
    pub bounds: SubMeshBounds,
    pub is_left: bool,
    pub head_x: f32,
    pub tail_x: f32,
}

impl EyebrowSubMesh {
    #[inline(always)]
    pub fn is_in_eyebrow_zone(&self, px: f32, py: f32) -> bool {
        self.bounds.contains(px, py)
    }
}

/// Extensión de Frente Adaptada por Contraste Tonal (Hairline Edge Detection)
pub struct AdaptiveForeheadMesh {
    /// Límite vertical Y superior para cada columna X interpolada entre las sienes
    pub hairline_y: Vec<f32>,
    pub min_x: usize,
    pub max_x: usize,
}

impl AdaptiveForeheadMesh {
    pub fn new() -> Self {
        Self {
            hairline_y: Vec::new(),
            min_x: 0,
            max_x: 0,
        }
    }

    /// Calcula la línea adaptativa del nacimiento del cabello escaneando gradientes tonales
    /// de color y luminancia (contraste piel vs. pelo) verticalmente desde las cejas/punto 10 hacia arriba.
    pub fn compute(
        &mut self,
        landmarks: &[Point2f],
        input_rgba: &[u8],
        width: usize,
        height: usize,
    ) {
        if landmarks.len() < 468 || input_rgba.len() < width * height * 4 {
            self.hairline_y.clear();
            return;
        }

        let p10 = landmarks[10];   // Centro superior de la frente canónica
        let p103 = landmarks[103]; // Sien izquierda
        let p332 = landmarks[332]; // Sien derecha
        let p9 = landmarks[9];     // Entrecejo / entre las cejas

        let min_x = ((p103.x.min(p332.x) - 15.0).floor() as usize).max(0);
        let max_x = ((p103.x.max(p332.x) + 15.0).ceil() as usize).min(width.saturating_sub(1));

        if max_x <= min_x {
            self.hairline_y.clear();
            return;
        }

        let span = max_x - min_x + 1;
        self.hairline_y.resize(span, 0.0);
        self.min_x = min_x;
        self.max_x = max_x;

        // Distancia anatómica de referencia (cejas a punto 10)
        let brow_to_mid = (p9.y - p10.y).abs().max(10.0);
        // La frente real puede extenderse hacia arriba hasta 1.35x de esa distancia
        let max_forehead_height = brow_to_mid * 1.35;

        // Tono de piel de referencia en el centro de la frente (punto 10)
        let ref_x = (p10.x.round() as usize).clamp(0, width - 1);
        let ref_y = (p10.y.round() as usize).clamp(0, height - 1);
        let ref_idx = (ref_y * width + ref_x) * 4;
        let ref_r = input_rgba[ref_idx] as f32 / 255.0;
        let ref_g = input_rgba[ref_idx + 1] as f32 / 255.0;
        let ref_b = input_rgba[ref_idx + 2] as f32 / 255.0;
        let (ref_y_val, ref_cb, ref_cr) = rgb_to_ycbcr(ref_r, ref_g, ref_b);

        for x in min_x..=max_x {
            let col_idx = x - min_x;
            // Interpolación de la base de la frente a lo largo del arco de las cejas/punto 10
            let t = (x - min_x) as f32 / (span as f32).max(1.0);
            let base_y = p10.y + (1.0 - (t * std::f32::consts::PI).sin()) * (brow_to_mid * 0.35);

            let scan_top = (base_y - max_forehead_height).max(0.0) as usize;
            let scan_bot = base_y.min((height - 1) as f32) as usize;

            let mut found_hair_y = scan_top as f32;

            // Escanear desde el centro de la frente (base_y) hacia arriba buscando el salto tonal (cabello/fondo)
            for y in (scan_top..scan_bot).rev() {
                let p_idx = (y * width + x) * 4;
                let r = input_rgba[p_idx] as f32 / 255.0;
                let g = input_rgba[p_idx + 1] as f32 / 255.0;
                let b = input_rgba[p_idx + 2] as f32 / 255.0;
                let (y_val, cb, cr) = rgb_to_ycbcr(r, g, b);

                // Diferencia cromática con la piel de la frente
                let d_cr = (cr - ref_cr).abs();
                let d_cb = (cb - ref_cb).abs();
                let is_skin_tone = cr > 0.025 && (cr - cb) > 0.020 && y_val > 0.12 && d_cr < 0.14 && d_cb < 0.14;

                // Si encontramos cabello (drástica caída de luminancia o cambio cromático a oscuro)
                let is_hair_contrast = (ref_y_val - y_val) > 0.28 || (!is_skin_tone && y_val < ref_y_val * 0.65);

                if is_hair_contrast {
                    found_hair_y = y as f32;
                    break;
                }
            }

            self.hairline_y[col_idx] = found_hair_y;
        }
    }

    #[inline(always)]
    pub fn is_in_forehead(&self, px: f32, py: f32, top_canonical_y: f32) -> bool {
        let ix = px.round() as usize;
        if ix >= self.min_x && ix <= self.max_x {
            let col = ix - self.min_x;
            if let Some(&hair_y) = self.hairline_y.get(col) {
                // Cubre desde el límite canónico de la malla (top_canonical_y) hacia arriba hasta el cabello
                return py >= hair_y && py <= top_canonical_y + 4.0;
            }
        }
        false
    }
}
