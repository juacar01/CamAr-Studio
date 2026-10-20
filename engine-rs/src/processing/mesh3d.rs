//! Módulo de Renderizado y Física de Mallas 3D Low-Poly (.OBJ / AR Props).
//!
//! Responsabilidad única (SRP): Estimación de pose tridimensional de cabeza (Head Pose 6DoF),
//! transformación matricial rígida (Rigid Body), oclusión con cráneo virtual y rasterización
//! ultra-rápida de triángulos 3D con Z-Buffer e iluminación Blinn-Phong/Fresnel.

use crate::types::Point2f;

#[derive(Clone, Debug)]
pub struct Mesh3D {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub triangles: Vec<[usize; 3]>,
}

impl Mesh3D {
    pub fn from_obj_str(content: &str) -> Self {
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut triangles = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("v ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let (Ok(x), Ok(y), Ok(z)) = (parts[1].parse(), parts[2].parse(), parts[3].parse()) {
                        vertices.push([x, y, z]);
                    }
                }
            } else if line.starts_with("vn ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    if let (Ok(x), Ok(y), Ok(z)) = (parts[1].parse(), parts[2].parse(), parts[3].parse()) {
                        normals.push([x, y, z]);
                    }
                }
            } else if line.starts_with("f ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let parse_idx = |s: &str| -> Option<usize> {
                        let token = s.split('/').next()?;
                        let idx: usize = token.parse().ok()?;
                        if idx > 0 { Some(idx - 1) } else { None }
                    };
                    if let (Some(p0), Some(p1), Some(p2)) = (parse_idx(parts[1]), parse_idx(parts[2]), parse_idx(parts[3])) {
                        triangles.push([p0, p1, p2]);
                    }
                }
            }
        }

        Self {
            vertices,
            normals,
            triangles,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HeadPose3D {
    pub center_x: f32,
    pub center_y: f32,
    pub scale: f32,
    pub rot: [[f32; 3]; 3],
    pub yaw_ratio: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SmoothedHeadPose3D {
    pub center_x: f32,
    pub center_y: f32,
    pub scale: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// Estima la orientación 3D completa de la cabeza (Yaw, Pitch, Roll) a partir de MediaPipe FaceMesh
pub fn estimate_head_pose_3d(
    landmarks: &[Point2f],
    smoothed: &mut Option<SmoothedHeadPose3D>,
) -> Option<HeadPose3D> {
    if landmarks.len() < 468 {
        *smoothed = None;
        return None;
    }

    let p_127 = landmarks[127]; // Sien izquierda del cráneo
    let p_356 = landmarks[356]; // Sien derecha del cráneo
    let p_1 = landmarks[1];     // Punta de nariz
    let p_168 = landmarks[168]; // Puente nasal / entrecejo
    let _p_9 = landmarks[9];     // Glabella (entre las cejas)
    let _p_10 = landmarks[10];   // Centro superior de la frente (nacimiento de pelo)
    let p_152 = landmarks[152]; // Barbilla / mentón

    let raw_dx = p_356.x - p_127.x;
    let raw_dy = p_356.y - p_127.y;
    let temple_dist = (raw_dx * raw_dx + raw_dy * raw_dy).sqrt().max(10.0);
    let raw_scale = (temple_dist / 110.0).clamp(0.35, 4.0);
    let raw_roll = raw_dy.atan2(raw_dx);

    let cos_roll = raw_roll.cos();
    let sin_roll = raw_roll.sin();

    // Yaw (rotación lateral): Proyección del vector nariz-centro respecto a las sienes
    let mid_x = (p_127.x + p_356.x) * 0.5;
    let mid_y = (p_127.y + p_356.y) * 0.5;
    let nose_proj = (p_1.x - mid_x) * cos_roll + (p_1.y - mid_y) * sin_roll;
    let raw_yaw_ratio = (nose_proj / (temple_dist * 0.40)).clamp(-1.25, 1.25);
    // Signo negativo: al girar la cabeza a la derecha en modo espejo, los cuernos giran a la derecha
    let raw_yaw_rad = -raw_yaw_ratio * 1.30; // hasta ~75°

    // Pitch (cabeceo arriba/abajo): Relación vertical entrecejo-mentón vs distancia de sienes
    let face_h = ((p_152.x - p_168.x).powi(2) + (p_152.y - p_168.y).powi(2)).sqrt().max(10.0);
    let vert_ratio = face_h / temple_dist;
    let raw_pitch_rad = ((0.85 - vert_ratio) * 1.2).clamp(-0.7, 0.7);

    // Centro 3D del cráneo (pivote anatómico de rotación de la cabeza):
    // Eje X: punto medio entre sienes p_127 y p_356
    // Eje Y: altura del puente nasal / ojos p_168
    let raw_cx = (p_127.x + p_356.x) * 0.5;
    let raw_cy = (p_127.y + p_356.y) * 0.5 * 0.35 + p_168.y * 0.65;

    // Filtro temporal adaptativo para eliminar micro-temblores a 60 FPS
    let (cx, cy, scale, yaw, pitch, roll) = match *smoothed {
        Some(prev) => {
            let dist_sq = (raw_cx - prev.center_x).powi(2) + (raw_cy - prev.center_y).powi(2);
            let alpha = if dist_sq < 4.0 { 0.25 } else if dist_sq < 49.0 { 0.55 } else { 0.85 };

            let n_cx = prev.center_x * (1.0 - alpha) + raw_cx * alpha;
            let n_cy = prev.center_y * (1.0 - alpha) + raw_cy * alpha;
            let n_scale = prev.scale * (1.0 - alpha) + raw_scale * alpha;
            let n_yaw = prev.yaw * (1.0 - alpha) + raw_yaw_rad * alpha;
            let n_pitch = prev.pitch * (1.0 - alpha) + raw_pitch_rad * alpha;
            let n_roll = prev.roll * (1.0 - alpha) + raw_roll * alpha;

            let updated = SmoothedHeadPose3D {
                center_x: n_cx,
                center_y: n_cy,
                scale: n_scale,
                yaw: n_yaw,
                pitch: n_pitch,
                roll: n_roll,
            };
            *smoothed = Some(updated);
            (n_cx, n_cy, n_scale, n_yaw, n_pitch, n_roll)
        }
        None => {
            let init = SmoothedHeadPose3D {
                center_x: raw_cx,
                center_y: raw_cy,
                scale: raw_scale,
                yaw: raw_yaw_rad,
                pitch: raw_pitch_rad,
                roll: raw_roll,
            };
            *smoothed = Some(init);
            (raw_cx, raw_cy, raw_scale, raw_yaw_rad, raw_pitch_rad, raw_roll)
        }
    };

    // Matriz de rotación 3D compuesta: R = R_roll * R_pitch * R_yaw
    let (cy_c, sy_c) = (yaw.cos(), yaw.sin());
    let (cp_c, sp_c) = (pitch.cos(), pitch.sin());
    let (cr_c, sr_c) = (roll.cos(), roll.sin());

    let rot = [
        [
            cr_c * cy_c + sr_c * sp_c * sy_c,
            -sr_c * cp_c,
            -cr_c * sy_c + sr_c * sp_c * cy_c,
        ],
        [
            sr_c * cy_c - cr_c * sp_c * sy_c,
            cr_c * cp_c,
            -sr_c * sy_c - cr_c * sp_c * cy_c,
        ],
        [
            cp_c * sy_c,
            sp_c,
            cp_c * cy_c,
        ],
    ];

    Some(HeadPose3D {
        center_x: cx,
        center_y: cy,
        scale,
        rot,
        yaw_ratio: raw_yaw_ratio,
    })
}

pub struct Mesh3DRenderer {
    pub width: usize,
    pub height: usize,
    pub z_buffer: Vec<f32>,
    pub prev_box: (usize, usize, usize, usize),
}

impl Mesh3DRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            z_buffer: vec![1e9f32; width * height],
            prev_box: (0, 0, 0, 0),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.z_buffer.resize(width * height, 1e9f32);
        self.z_buffer.fill(1e9f32);
        self.prev_box = (0, 0, 0, 0);
    }

    /// Limpia el Z-buffer en la región usada en el frame anterior
    pub fn clear_dirty_region(&mut self) {
        let (p_min_x, p_max_x, p_min_y, p_max_y) = self.prev_box;
        if p_max_x >= p_min_x && p_max_y >= p_min_y && p_max_x < self.width && p_max_y < self.height {
            for y in p_min_y..=p_max_y {
                let row_start = y * self.width;
                let end_idx = row_start + p_max_x;
                if end_idx < self.z_buffer.len() {
                    self.z_buffer[row_start + p_min_x..=end_idx].fill(1e9f32);
                }
            }
        }
    }

    /// Rasteriza una malla 3D completa sobre el búfer de imagen con oclusión de cráneo e iluminación 3D
    pub fn render_mesh(
        &mut self,
        mesh: &Mesh3D,
        pose: &HeadPose3D,
        intensity: f32,
        output_buffer: &mut [u8],
    ) -> usize {
        if intensity <= 0.001 || mesh.triangles.is_empty() {
            return 0;
        }

        self.clear_dirty_region();

        let width = self.width;
        let height = self.height;
        let scale = pose.scale;
        let cx = pose.center_x;
        let cy = pose.center_y;
        let r = pose.rot;

        // Vector de luz principal en el espacio de mundo (luz cenital-derecha)
        let light = [0.42f32, -0.65f32, 0.63f32]; // ya normalizado

        // 1. Proyectar vértices y normales en espacio de pantalla 3D
        let mut proj_verts = Vec::with_capacity(mesh.vertices.len());
        let mut world_norms = Vec::with_capacity(mesh.normals.len());

        for v in &mesh.vertices {
            let rx = r[0][0] * v[0] + r[0][1] * v[1] + r[0][2] * v[2];
            let ry = r[1][0] * v[0] + r[1][1] * v[1] + r[1][2] * v[2];
            let rz = r[2][0] * v[0] + r[2][1] * v[1] + r[2][2] * v[2];

            let sx = cx + rx * scale;
            let sy = cy + ry * scale;
            let sz = rz * scale; // Z hacia la cámara (positivo)

            proj_verts.push((sx, sy, sz));
        }

        for n in &mesh.normals {
            let nx = r[0][0] * n[0] + r[0][1] * n[1] + r[0][2] * n[2];
            let ny = r[1][0] * n[0] + r[1][1] * n[1] + r[1][2] * n[2];
            let nz = r[2][0] * n[0] + r[2][1] * n[1] + r[2][2] * n[2];
            world_norms.push([nx, ny, nz]);
        }

        let mut drawn_pixels = 0usize;
        let mut min_bound_x = width - 1;
        let mut max_bound_x = 0;
        let mut min_bound_y = height - 1;
        let mut max_bound_y = 0;

        // Oclusión con cráneo virtual (elipsoide de la cabeza centrado en el cráneo)
        let r_skull_x = 68.0 * scale;
        let r_skull_y = 85.0 * scale;
        let skull_cy = cy - 5.0 * scale;

        // 2. Rasterización de triángulos con coordenadas baricéntricas y Z-Buffer
        for tri in &mesh.triangles {
            let p0 = proj_verts[tri[0]];
            let p1 = proj_verts[tri[1]];
            let p2 = proj_verts[tri[2]];

            // Backface culling en 2D
            let cross_2d = (p1.0 - p0.0) * (p2.1 - p0.1) - (p1.1 - p0.1) * (p2.0 - p0.0);
            if cross_2d <= 0.0 {
                continue;
            }

            let n0 = world_norms[tri[0]];
            let n1 = world_norms[tri[1]];
            let n2 = world_norms[tri[2]];

            let avg_nx = (n0[0] + n1[0] + n2[0]) / 3.0;
            let avg_ny = (n0[1] + n1[1] + n2[1]) / 3.0;
            let avg_nz = (n0[2] + n1[2] + n2[2]) / 3.0;

            let n_len = (avg_nx * avg_nx + avg_ny * avg_ny + avg_nz * avg_nz).sqrt();
            let (nx, ny, nz) = if n_len > 1e-4 {
                (avg_nx / n_len, avg_ny / n_len, avg_nz / n_len)
            } else {
                (0.0, 0.0, 1.0)
            };

            // Iluminación 3D Difusa Lambertiana
            let l_dot_n = (nx * light[0] + ny * light[1] + nz * light[2]).max(0.0);

            // Reflejo Especular Blinn-Phong
            let _rx = 2.0 * l_dot_n * nx - light[0];
            let _ry = 2.0 * l_dot_n * ny - light[1];
            let rz = 2.0 * l_dot_n * nz - light[2];
            let spec = rz.max(0.0).powi(16);

            // Luz de borde Fresnel (Rim light rojo sangre demoníaco)
            let rim = (1.0 - nz.abs()).max(0.0).powf(2.4);

            // Gradiente cromático (obsidiana profundo en la base, escarlata ardiente en punta)
            let local_y0 = mesh.vertices[tri[0]][1];
            // Y va de +12.0 (base en cráneo) a -108.0 (punta)
            let t_height = ((12.0 - local_y0) / 120.0).clamp(0.0, 1.0);

            let base_lum = 0.20 + 0.80 * t_height.powf(0.75);
            let base_fade = (t_height / 0.15).clamp(0.0, 1.0);

            let horn_r = ((190.0 * base_lum + 85.0 * l_dot_n + 190.0 * spec + 140.0 * rim) * (0.35 + 0.65 * base_fade)).clamp(0.0, 255.0);
            let horn_g = ((15.0 * base_lum + 18.0 * l_dot_n + 130.0 * spec + 40.0 * rim) * (0.20 + 0.80 * base_fade)).clamp(0.0, 255.0);
            let horn_b = ((15.0 * base_lum + 15.0 * l_dot_n + 140.0 * spec + 40.0 * rim) * (0.20 + 0.80 * base_fade)).clamp(0.0, 255.0);

            // Bounding box del triángulo
            let t_min_x = (p0.0.min(p1.0).min(p2.0).floor() as isize).max(0) as usize;
            let t_max_x = (p0.0.max(p1.0).max(p2.0).ceil() as isize).min(width as isize - 1) as usize;
            let t_min_y = (p0.0.min(p1.1).min(p2.1).floor() as isize).max(0) as usize;
            let t_max_y = (p0.1.max(p1.1).max(p2.1).ceil() as isize).min(height as isize - 1) as usize;

            if t_min_x > t_max_x || t_min_y > t_max_y {
                continue;
            }

            min_bound_x = min_bound_x.min(t_min_x);
            max_bound_x = max_bound_x.max(t_max_x);
            min_bound_y = min_bound_y.min(t_min_y);
            max_bound_y = max_bound_y.max(t_max_y);

            let inv_area = 1.0 / cross_2d;

            for y in t_min_y..=t_max_y {
                let y_f = y as f32;
                let row_start = y * width;
                let out_row_start = row_start * 4;

                let dy_skull = (y_f - skull_cy) / r_skull_y;
                let dy_sq = dy_skull * dy_skull;

                for x in t_min_x..=t_max_x {
                    let x_f = x as f32;

                    let w0 = ((p1.1 - p2.1) * (x_f - p2.0) + (p2.0 - p1.0) * (y_f - p2.1)) * inv_area;
                    let w1 = ((p2.1 - p0.1) * (x_f - p2.0) + (p0.0 - p2.0) * (y_f - p2.1)) * inv_area;
                    let w2 = 1.0 - w0 - w1;

                    if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                        // Profundidad Z: invertida para que menor valor sea más cercano a cámara
                        let depth = -(w0 * p0.2 + w1 * p1.2 + w2 * p2.2);

                        // Oclusión anatómica con el cráneo:
                        // Si el píxel cae dentro de la silueta del cráneo y está en la parte trasera (Z > 0), no se dibuja
                        let dx_skull = (x_f - cx) / r_skull_x;
                        if dx_skull * dx_skull + dy_sq < 1.0 && depth > 5.0 * scale {
                            continue;
                        }

                        let z_idx = row_start + x;
                        let out_idx = out_row_start + x * 4;
                        if z_idx < self.z_buffer.len() && out_idx + 3 < output_buffer.len() && depth < self.z_buffer[z_idx] {
                            self.z_buffer[z_idx] = depth;
                            drawn_pixels += 1;

                            let cur_r = output_buffer[out_idx] as f32;
                            let cur_g = output_buffer[out_idx + 1] as f32;
                            let cur_b = output_buffer[out_idx + 2] as f32;

                            output_buffer[out_idx] = (cur_r * (1.0 - intensity) + horn_r * intensity).clamp(0.0, 255.0) as u8;
                            output_buffer[out_idx + 1] = (cur_g * (1.0 - intensity) + horn_g * intensity).clamp(0.0, 255.0) as u8;
                            output_buffer[out_idx + 2] = (cur_b * (1.0 - intensity) + horn_b * intensity).clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
        }

        if min_bound_x <= max_bound_x && min_bound_y <= max_bound_y {
            self.prev_box = (min_bound_x, max_bound_x, min_bound_y, max_bound_y);
        } else {
            self.prev_box = (0, 0, 0, 0);
        }

        drawn_pixels
    }
}
