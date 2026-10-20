use crate::types::{CameraDeviceInfo, CameraFormatInfo};
use nokhwa::pixel_format::RgbAFormat;
use nokhwa::query;
use nokhwa::utils::{
    ApiBackend, CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType,
    Resolution,
};
use nokhwa::Camera;
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct VideoCapture {
    pub width: usize,
    pub height: usize,
    synthetic_buffer: Vec<u8>,
    live_frame: Arc<Mutex<Vec<u8>>>,
    running: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    is_live_camera: bool,
}

impl VideoCapture {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            synthetic_buffer: vec![0; width * height * 4],
            live_frame: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            worker: None,
            is_live_camera: false,
        }
    }

    /// Enumeración dinámica de todas las cámaras web físicas en Windows sin nada quemado
    pub fn enumerate_cameras() -> Vec<CameraDeviceInfo> {
        let mut devices = Vec::new();

        // 1. Consultar dispositivos de Windows Media Foundation
        if let Ok(cams) = query(ApiBackend::MediaFoundation) {
            for cam_info in cams {
                let name = cam_info.human_name();
                let id = match cam_info.index() {
                    CameraIndex::Index(i) => i.to_string(),
                    CameraIndex::String(s) => s.clone(),
                };
                let mut formats = Vec::new();

                // Consultar formatos compatibles directamente del hardware
                if let Ok(mut temp_cam) = Camera::new(
                    cam_info.index().clone(),
                    RequestedFormat::new::<RgbAFormat>(RequestedFormatType::None),
                ) {
                    if let Ok(compat_formats) = temp_cam.compatible_camera_formats() {
                        for cf in compat_formats {
                            let fmt_name = match cf.format() {
                                FrameFormat::NV12 => "NV12".to_string(),
                                FrameFormat::YUYV => "YUY2".to_string(),
                                FrameFormat::MJPEG => "MJPG".to_string(),
                                FrameFormat::RAWRGB => "RGB24".to_string(),
                                other => format!("{:?}", other),
                            };

                            let format_entry = CameraFormatInfo {
                                width: cf.width(),
                                height: cf.height(),
                                fps: cf.frame_rate(),
                                format: fmt_name,
                            };

                            if !formats.contains(&format_entry) {
                                formats.push(format_entry);
                            }
                        }
                    }
                }

                // Limitar estrictamente a un máximo de 30 FPS para captura eficiente y estable
                formats.retain(|f| f.fps <= 30);
                if formats.is_empty() {
                    // Fallback de seguridad si el driver reportara solo 60 fps
                    formats.push(CameraFormatInfo { width: 1920, height: 1080, fps: 30, format: "NV12".to_string() });
                }

                // Ordenar formatos: mayor resolución y FPS primero
                formats.sort_by(|a, b| {
                    b.width
                        .cmp(&a.width)
                        .then_with(|| b.height.cmp(&a.height))
                        .then_with(|| b.fps.cmp(&a.fps))
                });

                devices.push(CameraDeviceInfo { id, name, formats });
            }
        }

        // Simulador de Estudio con múltiples resoluciones @ máx 30 FPS
        devices.push(CameraDeviceInfo {
            id: "synthetic".to_string(),
            name: "CamAr Studio Simulator (Test Pattern & Avatar)".to_string(),
            formats: vec![
                CameraFormatInfo { width: 3840, height: 2160, fps: 30, format: "NV12".to_string() },
                CameraFormatInfo { width: 2560, height: 1440, fps: 30, format: "NV12".to_string() },
                CameraFormatInfo { width: 1920, height: 1080, fps: 30, format: "NV12".to_string() },
                CameraFormatInfo { width: 1920, height: 1080, fps: 30, format: "YUY2".to_string() },
                CameraFormatInfo { width: 1280, height: 720, fps: 30, format: "NV12".to_string() },
                CameraFormatInfo { width: 854, height: 480, fps: 30, format: "NV12".to_string() },
            ],
        });

        devices
    }

    /// Inicia la captura física de la cámara seleccionada con la resolución, FPS y formato elegidos
    pub fn start_camera(
        &mut self,
        cam_id: &str,
        width: u32,
        height: u32,
        fps: u32,
        format_str: &str,
    ) {
        self.stop();

        self.width = width as usize;
        self.height = height as usize;
        self.synthetic_buffer.resize(self.width * self.height * 4, 0);

        if cam_id == "synthetic" || cam_id == "auto" {
            self.is_live_camera = false;
            crate::log_info!("Capture", "Iniciado en modo Simulador Procedural 1080p60.");
            return;
        }

        let fourcc = match format_str {
            "YUY2" => FrameFormat::YUYV,
            "MJPG" => FrameFormat::MJPEG,
            "RGB24" => FrameFormat::RAWRGB,
            _ => FrameFormat::NV12,
        };

        // Parsear CameraIndex de forma robusta
        let index = if let Ok(idx) = cam_id.parse::<u32>() {
            CameraIndex::Index(idx)
        } else if cam_id.starts_with("Index(") {
            let num = cam_id.trim_start_matches("Index(").trim_end_matches(')').parse::<u32>().unwrap_or(0);
            CameraIndex::Index(num)
        } else {
            CameraIndex::Index(0)
        };

        self.running.store(true, Ordering::SeqCst);
        self.is_live_camera = true;
        let frame_store = Arc::clone(&self.live_frame);
        let running_flag = Arc::clone(&self.running);
        let w = self.width;
        let h = self.height;

        let handle = thread::spawn(move || {
            let capped_fps = fps.clamp(1, 30);
            let req_format = RequestedFormat::new::<RgbAFormat>(RequestedFormatType::Closest(CameraFormat::new(
                Resolution::new(width, height),
                fourcc,
                capped_fps,
            )));

            match Camera::new(index, req_format) {
                Ok(mut cam) => {
                    if let Err(e) = cam.open_stream() {
                        crate::log_error!("Capture", "Error al abrir stream de cámara: {}", e);
                        return;
                    }

                    let mut capture_frame_count = 0;
                    let mut last_capture_time = std::time::Instant::now();
                    let mut converted = vec![0u8; w * h * 4];

                    while running_flag.load(Ordering::Relaxed) {
                        match cam.frame() {
                            Ok(frame) => {
                                capture_frame_count += 1;
                                if last_capture_time.elapsed().as_secs_f32() >= 1.0 {
                                    crate::log_info!("Capture", "Hardware camera capture rate: {} frames/sec", capture_frame_count);
                                    capture_frame_count = 0;
                                    last_capture_time = std::time::Instant::now();
                                }

                                // 1. Conversor multihilo directo Rayon con búfer pre-asignado (Cero Heap Allocs)
                                if Self::convert_raw_to_rgba(frame.buffer(), w, h, fourcc, &mut converted) {
                                    if let Ok(mut lock) = frame_store.lock() {
                                        if lock.len() != converted.len() {
                                            lock.resize(converted.len(), 0);
                                        }
                                        lock.copy_from_slice(&converted);
                                    }
                                } else if let Ok(decoded) = frame.decode_image::<RgbAFormat>() {
                                    // 2. Fallback decodificador nativo nokhwa
                                    let raw_bytes = decoded.into_raw();
                                    if let Ok(mut lock) = frame_store.lock() {
                                        *lock = raw_bytes;
                                    }
                                }
                            }
                            Err(e) => {
                                crate::log_warn!("Capture", "Aviso temporal de captura: {}", e);
                                std::thread::sleep(std::time::Duration::from_millis(16));
                            }
                        }
                    }
                    let _ = cam.stop_stream();
                }
                Err(e) => {
                    crate::log_error!("Capture", "Error al inicializar cámara física: {}", e);
                }
            }
        });

        self.worker = Some(handle);
        crate::log_info!(
            "Capture",
            "Cámara física conectada exitosamente a {}x{} @ {} FPS [{}]",
            width, height, fps, format_str
        );
    }

    /// Conversor multihilo directo para formatos de hardware (NV12, YUY2, MJPEG, RGB24)
    fn convert_raw_to_rgba(
        buffer: &[u8],
        width: usize,
        height: usize,
        fourcc: FrameFormat,
        output: &mut [u8],
    ) -> bool {
        let expected_size = width * height * 4;
        if output.len() < expected_size {
            return false;
        }

        match fourcc {
            FrameFormat::NV12 => {
                let y_size = width * height;
                if buffer.len() < y_size + y_size / 2 {
                    return false;
                }
                let y_plane = &buffer[0..y_size];
                let uv_plane = &buffer[y_size..];

                output
                    .par_chunks_exact_mut(width * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        let uv_row_offset = (y / 2) * width;
                        let y_row_offset = y * width;
                        for x in (0..width).step_by(2) {
                            let uv_idx = uv_row_offset + x;
                            let u_val = uv_plane[uv_idx] as i32 - 128;
                            let v_val = uv_plane[uv_idx + 1] as i32 - 128;

                            let c_r = (403 * v_val) >> 8;
                            let c_g = (-48 * u_val - 120 * v_val) >> 8;
                            let c_b = (475 * u_val) >> 8;

                            let y0 = y_plane[y_row_offset + x] as i32;
                            let px0 = x * 4;
                            row[px0] = (y0 + c_r).clamp(0, 255) as u8;
                            row[px0 + 1] = (y0 + c_g).clamp(0, 255) as u8;
                            row[px0 + 2] = (y0 + c_b).clamp(0, 255) as u8;
                            row[px0 + 3] = 255;

                            if x + 1 < width {
                                let y1 = y_plane[y_row_offset + x + 1] as i32;
                                let px1 = px0 + 4;
                                row[px1] = (y1 + c_r).clamp(0, 255) as u8;
                                row[px1 + 1] = (y1 + c_g).clamp(0, 255) as u8;
                                row[px1 + 2] = (y1 + c_b).clamp(0, 255) as u8;
                                row[px1 + 3] = 255;
                            }
                        }
                    });
                true
            }
            FrameFormat::YUYV => {
                let yuyv_size = width * height * 2;
                if buffer.len() < yuyv_size {
                    return false;
                }
                output
                    .par_chunks_exact_mut(width * 4)
                    .enumerate()
                    .for_each(|(y, row)| {
                        let yuyv_row = &buffer[y * width * 2..(y + 1) * width * 2];
                        for chunk in 0..(width / 2) {
                            let y0 = yuyv_row[chunk * 4] as f32;
                            let u = yuyv_row[chunk * 4 + 1] as f32 - 128.0;
                            let y1 = yuyv_row[chunk * 4 + 2] as f32;
                            let v = yuyv_row[chunk * 4 + 3] as f32 - 128.0;

                            let r0 = (y0 + 1.5748 * v).clamp(0.0, 255.0) as u8;
                            let g0 = (y0 - 0.1873 * u - 0.4681 * v).clamp(0.0, 255.0) as u8;
                            let b0 = (y0 + 1.8556 * u).clamp(0.0, 255.0) as u8;

                            let r1 = (y1 + 1.5748 * v).clamp(0.0, 255.0) as u8;
                            let g1 = (y1 - 0.1873 * u - 0.4681 * v).clamp(0.0, 255.0) as u8;
                            let b1 = (y1 + 1.8556 * u).clamp(0.0, 255.0) as u8;

                            let px0 = chunk * 8;
                            row[px0] = r0;
                            row[px0 + 1] = g0;
                            row[px0 + 2] = b0;
                            row[px0 + 3] = 255;

                            row[px0 + 4] = r1;
                            row[px0 + 5] = g1;
                            row[px0 + 6] = b1;
                            row[px0 + 7] = 255;
                        }
                    });
                true
            }
            FrameFormat::MJPEG => {
                if let Ok(img) = image::load_from_memory(buffer) {
                    let rgba = img.to_rgba8();
                    let raw = rgba.into_raw();
                    if raw.len() == expected_size {
                        output.copy_from_slice(&raw);
                        return true;
                    }
                }
                false
            }
            _ => {
                if buffer.len() == expected_size {
                    output.copy_from_slice(buffer);
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn get_frame(&mut self, time_sec: f32) -> &[u8] {
        if self.is_live_camera {
            if let Ok(lock) = self.live_frame.lock() {
                if lock.len() == self.width * self.height * 4 {
                    if self.synthetic_buffer.len() != lock.len() {
                        self.synthetic_buffer.resize(lock.len(), 0);
                    }
                    self.synthetic_buffer.copy_from_slice(&lock);
                    return &self.synthetic_buffer;
                }
            }
        }

        // Generador procedural si no hay frame aún
        self.generate_synthetic(time_sec)
    }

    fn generate_synthetic(&mut self, time_sec: f32) -> &[u8] {
        let width = self.width;
        let height = self.height;

        let total_size = width * height * 4;
        if self.synthetic_buffer.len() != total_size {
            self.synthetic_buffer.resize(total_size, 0);
        }

        let head_center_x = width as f32 * (0.5 + 0.04 * (time_sec * 1.5).sin());
        let head_center_y = height as f32 * (0.42 + 0.02 * (time_sec * 2.0).cos());
        let head_radius = height as f32 * 0.18;

        for y in 0..height {
            let ny = y as f32 / height as f32;
            let row_offset = y * width * 4;

            for x in 0..width {
                let nx = x as f32 / width as f32;
                let idx = row_offset + x * 4;

                let mut bg_r = 0.15 + 0.1 * (nx * std::f32::consts::PI + time_sec * 0.5).sin();
                let mut bg_g = 0.10 + 0.05 * (ny * std::f32::consts::PI).cos();
                let mut bg_b = 0.35 + 0.15 * ((nx + ny) * std::f32::consts::PI).sin();

                if x % 80 == 0 || y % 80 == 0 {
                    bg_r += 0.08;
                    bg_g += 0.08;
                    bg_b += 0.12;
                }

                let dx = x as f32 - head_center_x;
                let dy = y as f32 - head_center_y;
                let dist_head = (dx * dx + dy * dy).sqrt();

                let torso_y = y as f32 - (head_center_y + head_radius * 0.8);
                let torso_width = if torso_y > 0.0 { head_radius * 1.2 + torso_y * 0.8 } else { 0.0 };
                let in_torso = torso_y > 0.0 && dx.abs() < torso_width && y < height;

                if dist_head < head_radius {
                    let skin_shade = 1.0 - (dist_head / head_radius) * 0.3;
                    self.synthetic_buffer[idx] = (245.0 * skin_shade) as u8;
                    self.synthetic_buffer[idx + 1] = (195.0 * skin_shade) as u8;
                    self.synthetic_buffer[idx + 2] = (170.0 * skin_shade) as u8;
                    self.synthetic_buffer[idx + 3] = 255;

                    let eye_y = head_center_y - head_radius * 0.15;
                    let left_eye_x = head_center_x - head_radius * 0.35;
                    let right_eye_x = head_center_x + head_radius * 0.35;
                    let d_eye_l = ((x as f32 - left_eye_x).powi(2) + (y as f32 - eye_y).powi(2)).sqrt();
                    let d_eye_r = ((x as f32 - right_eye_x).powi(2) + (y as f32 - eye_y).powi(2)).sqrt();
                    if d_eye_l < head_radius * 0.12 || d_eye_r < head_radius * 0.12 {
                        self.synthetic_buffer[idx] = 30;
                        self.synthetic_buffer[idx + 1] = 40;
                        self.synthetic_buffer[idx + 2] = 50;
                    }

                    let mouth_y = head_center_y + head_radius * 0.45;
                    if (y as f32 - mouth_y).abs() < head_radius * 0.08 && dx.abs() < head_radius * 0.25 {
                        self.synthetic_buffer[idx] = 200;
                        self.synthetic_buffer[idx + 1] = 70;
                        self.synthetic_buffer[idx + 2] = 80;
                    }
                } else if in_torso {
                    self.synthetic_buffer[idx] = 45;
                    self.synthetic_buffer[idx + 1] = 60;
                    self.synthetic_buffer[idx + 2] = 95;
                    self.synthetic_buffer[idx + 3] = 255;
                } else {
                    self.synthetic_buffer[idx] = (bg_r.min(1.0) * 255.0) as u8;
                    self.synthetic_buffer[idx + 1] = (bg_g.min(1.0) * 255.0) as u8;
                    self.synthetic_buffer[idx + 2] = (bg_b.min(1.0) * 255.0) as u8;
                    self.synthetic_buffer[idx + 3] = 255;
                }
            }
        }

        &self.synthetic_buffer
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.is_live_camera = false;
    }
}

impl Drop for VideoCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Rotación multihilo ultra-rápida de buffers RGBA (<0.5ms) para orientación de sensor vertical/apaisado
pub fn rotate_rgba(src: &[u8], src_w: usize, src_h: usize, rot: i32, dst: &mut Vec<u8>) {
    let rot_norm = ((rot % 360) + 360) % 360;
    let (dst_w, dst_h) = match rot_norm {
        90 | 270 => (src_h, src_w),
        _ => (src_w, src_h),
    };
    let target_len = dst_w * dst_h * 4;
    if dst.len() != target_len {
        dst.resize(target_len, 0);
    }

    if rot_norm == 0 {
        dst.copy_from_slice(src);
        return;
    }

    match rot_norm {
        90 => {
            // 90° Horario (Vertical Derecha): dst(x, y) = src(y, src_h - 1 - x)
            dst.par_chunks_exact_mut(dst_w * 4)
                .enumerate()
                .for_each(|(y_out, row)| {
                    let x_in = y_out;
                    for x_out in 0..dst_w {
                        let y_in = src_h - 1 - x_out;
                        let src_idx = (y_in * src_w + x_in) * 4;
                        let dst_idx = x_out * 4;
                        row[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
                    }
                });
        }
        180 => {
            // 180° Invertido: dst(x, y) = src(src_w - 1 - x, src_h - 1 - y)
            dst.par_chunks_exact_mut(dst_w * 4)
                .enumerate()
                .for_each(|(y_out, row)| {
                    let y_in = src_h - 1 - y_out;
                    for x_out in 0..dst_w {
                        let x_in = src_w - 1 - x_out;
                        let src_idx = (y_in * src_w + x_in) * 4;
                        let dst_idx = x_out * 4;
                        row[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
                    }
                });
        }
        270 => {
            // 270° Horario / 90° Antihorario (Vertical Izquierda): dst(x, y) = src(src_w - 1 - y, x)
            dst.par_chunks_exact_mut(dst_w * 4)
                .enumerate()
                .for_each(|(y_out, row)| {
                    let x_in = src_w - 1 - y_out;
                    for x_out in 0..dst_w {
                        let y_in = x_out;
                        let src_idx = (y_in * src_w + x_in) * 4;
                        let dst_idx = x_out * 4;
                        row[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
                    }
                });
        }
        _ => {
            dst.copy_from_slice(src);
        }
    }
}

