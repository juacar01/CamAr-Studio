//! Módulo de Máscara Facial Canónica 3D (468 vértices, 898 triángulos).
//!
//! Responsabilidad única (SRP): Rasterización ultra-rápida de la malla UV completa
//! del rostro, vinculación anatómica de texturas de maquillaje y corrección de ojeras.

use super::face_mesh_data::{CANONICAL_UV, FACEMESH_TRIANGLES, FACEMESH_TRI_TYPE};
use crate::types::Point2f;

#[inline(always)]
pub fn pack_face_uv(u: f32, v: f32, region: u8) -> u32 {
    let u_int = (u.clamp(0.0, 1.0) * 4095.0) as u32;
    let v_int = (v.clamp(0.0, 1.0) * 4095.0) as u32;
    let reg_int = (region & 0x0F) as u32;
    (1 << 31) | (reg_int << 24) | (v_int << 12) | u_int
}

#[inline(always)]
pub fn unpack_face_uv(packed: u32) -> Option<(f32, f32, u8)> {
    if packed & (1 << 31) == 0 {
        return None;
    }
    let u = (packed & 0xFFF) as f32 * (1.0 / 4095.0);
    let v = ((packed >> 12) & 0xFFF) as f32 * (1.0 / 4095.0);
    let region = ((packed >> 24) & 0x0F) as u8;
    Some((u, v, region))
}

pub struct FaceMeshRenderer {
    pub width: usize,
    pub height: usize,
    pub face_uv_map: Vec<u32>,
    pub prev_box: (usize, usize, usize, usize),
}

impl FaceMeshRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            face_uv_map: vec![0; width * height],
            prev_box: (0, 0, 0, 0),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.face_uv_map.resize(width * height, 0);
        self.face_uv_map.fill(0);
        self.prev_box = (0, 0, 0, 0);
    }

    /// Limpia solo el área del rostro del frame anterior y rasteriza los 898 triángulos canónicos.
    pub fn rasterize_face(&mut self, landmarks: &[Point2f]) -> Option<(usize, usize, usize, usize)> {
        if landmarks.len() < 468 {
            return None;
        }

        // 1. Limpieza rápida del bounding box del frame anterior
        let (p_min_x, p_max_x, p_min_y, p_max_y) = self.prev_box;
        if p_max_x >= p_min_x && p_max_y >= p_min_y && p_max_x < self.width && p_max_y < self.height {
            for y in p_min_y..=p_max_y {
                let row_start = y * self.width;
                let end_idx = row_start + p_max_x;
                if end_idx < self.face_uv_map.len() {
                    let slice = &mut self.face_uv_map[row_start + p_min_x..=end_idx];
                    slice.fill(0);
                }
            }
        }

        // 2. Calcular nuevo bounding box del rostro
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for p in landmarks.iter().take(468) {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }

        let f_min_x = (min_x.floor() as usize).saturating_sub(4).max(0);
        let f_max_x = (max_x.ceil() as usize + 4).min(self.width - 1);
        let f_min_y = (min_y.floor() as usize).saturating_sub(4).max(0);
        let f_max_y = (max_y.ceil() as usize + 4).min(self.height - 1);

        self.prev_box = (f_min_x, f_max_x, f_min_y, f_max_y);

        // 3. Rasterizar los 898 triángulos canónicos
        for i in 0..898 {
            let region = FACEMESH_TRI_TYPE[i];
            // Región 2 = Interior de la boca (dientes/garganta): excluir
            // Región 3 = Córnea y esclerótica del ojo: excluir (se maneja por el motor de iris)
            if region == 2 || region == 3 {
                continue;
            }

            let [v0, v1, v2] = FACEMESH_TRIANGLES[i];
            let p0 = (landmarks[v0 as usize].x, landmarks[v0 as usize].y);
            let p1 = (landmarks[v1 as usize].x, landmarks[v1 as usize].y);
            let p2 = (landmarks[v2 as usize].x, landmarks[v2 as usize].y);

            let denom = (p1.1 - p2.1) * (p0.0 - p2.0) + (p2.0 - p1.0) * (p0.1 - p2.1);
            if denom.abs() < 1e-4 {
                continue;
            }
            let inv_denom = 1.0 / denom;

            let t_min_x = (p0.0.min(p1.0).min(p2.0).floor() as usize).max(f_min_x);
            let t_max_x = (p0.0.max(p1.0).max(p2.0).ceil() as usize).min(f_max_x);
            let t_min_y = (p0.1.min(p1.1).min(p2.1).floor() as usize).max(f_min_y);
            let t_max_y = (p0.1.max(p1.1).max(p2.1).ceil() as usize).min(f_max_y);

            let uv0 = CANONICAL_UV[v0 as usize];
            let uv1 = CANONICAL_UV[v1 as usize];
            let uv2 = CANONICAL_UV[v2 as usize];

            for y in t_min_y..=t_max_y {
                let py = y as f32 + 0.5;
                let row_offset = y * self.width;

                for x in t_min_x..=t_max_x {
                    let px = x as f32 + 0.5;

                    let w0 = ((p1.1 - p2.1) * (px - p2.0) + (p2.0 - p1.0) * (py - p2.1)) * inv_denom;
                    if w0 < 0.0 { continue; }
                    let w1 = ((p2.1 - p0.1) * (px - p2.0) + (p0.0 - p2.0) * (py - p2.1)) * inv_denom;
                    if w1 < 0.0 { continue; }
                    let w2 = 1.0 - w0 - w1;
                    if w2 < 0.0 { continue; }

                    let u = w0 * uv0[0] + w1 * uv1[0] + w2 * uv2[0];
                    let v = w0 * uv0[1] + w1 * uv1[1] + w2 * uv2[1];

                    let idx = row_offset + x;
                    if idx < self.face_uv_map.len() {
                        self.face_uv_map[idx] = pack_face_uv(u, v, region);
                    }
                }
            }
        }

        Some((f_min_x, f_max_x, f_min_y, f_max_y))
    }
}
