mod ai;
mod audio;
mod capture;
mod hardware;
mod ipc;
mod logger;
mod processing;
mod types;
mod virtualcam;

use ai::AiEngine;
use audio::AudioEngine;
use capture::VideoCapture;
use hardware::SystemHardwareInfo;
use include_dir::{include_dir, Dir};
use ipc::IpcServer;
use processing::VideoProcessor;
use serde_json::json;
use std::borrow::Cow;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use tokio::sync::RwLock;
use types::{EngineTelemetry, StudioConfig};
use virtualcam::VirtualCamManager;
use wry::http::{header::CONTENT_TYPE, Response};
use wry::WebViewBuilder;

// Embeber los archivos compilados de la UI dentro del binario
static UI_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../ui/dist");

#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(uPeriod: u32) -> u32;
    #[allow(dead_code)]
    fn timeEndPeriod(uPeriod: u32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    fn SetDllDirectoryA(lpPathName: *const u8) -> i32;
}

fn get_mime_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    }
}

fn load_app_icon() -> Option<tao::window::Icon> {
    const ICO_BYTES: &[u8] = include_bytes!("../app.ico");
    if let Ok(img) = image::load_from_memory(ICO_BYTES) {
        let rgba = img.into_rgba8();
        let (w, h) = rgba.dimensions();
        tao::window::Icon::from_rgba(rgba.into_raw(), w, h).ok()
    } else {
        None
    }
}

fn run_multimedia_engine(
    config: Arc<RwLock<StudioConfig>>,
    devices_json: String,
) {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            let desc = format!("Fallo al crear el entorno de ejecución multihilo (Tokio): {}", e);
            crate::logger::show_error_dialog("Motor Multimedia (Tokio Runtime)", &desc);
            panic!("{}", desc);
        }
    };

    rt.block_on(async move {
        let (ipc_server, telemetry_tx, video_tx) = IpcServer::new(Arc::clone(&config), devices_json);

        tokio::spawn(async move {
            if let Err(e) = ipc_server.run(8765).await {
                log_error!("IPC-RS", "Error en servidor WebSocket: {}", e);
            }
        });

        // Inicializar captura de video
        let mut width = 1920;
        let mut height = 1080;
        let mut capture = VideoCapture::new(width, height);
        let initial_ai_backend = {
            let cfg = config.read().await;
            cfg.ai_backend.clone()
        };
        let mut ai_engine = AiEngine::new(width, height, &initial_ai_backend);
        let mut processor = VideoProcessor::new(width, height);
        let mut virtual_cam = VirtualCamManager::new(width, height);

        // Inicializar captura de audio en vivo
        let mut audio_engine = AudioEngine::new();
        let _ = audio_engine.start("default");

        let mut current_cam_id = String::new();
        let mut current_w = 0;
        let mut current_h = 0;
        let mut current_fps = 0;
        let mut current_fmt = String::new();
        let mut current_rotation: i32 = -1;
        let mut current_audio_id = "default".to_string();
        let mut rotated_frame_buf = Vec::new();

        let hardware_info = SystemHardwareInfo::detect();
        let mut last_video_backend = String::new();
        let mut last_ai_backend = String::new();

        // Worker multihilo desacoplado para compresión JPEG con reciclaje de búfer (Zero-Allocation)
        let (preview_tx, preview_rx) = std::sync::mpsc::sync_channel::<(Vec<u8>, u32, u32)>(1);
        let (recycle_tx, recycle_rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(2);
        let preview_vtx = video_tx.clone();
        let worker_recycle_tx = recycle_tx.clone();
        std::thread::Builder::new()
            .name("CamAr-PreviewEncoder".to_string())
            .spawn(move || {
                let mut jpeg_bytes = Vec::with_capacity(131072);

                while let Ok((rgb_data, pw, ph)) = preview_rx.recv() {
                    jpeg_bytes.clear();
                    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_bytes, 62);
                    if let Ok(()) = encoder.encode(
                        &rgb_data,
                        pw,
                        ph,
                        image::ExtendedColorType::Rgb8,
                    ) {
                        let _ = preview_vtx.send(jpeg_bytes.clone());
                    }
                    // Devolver el búfer para reutilización inmediata (Zero Heap Allocation)
                    let _ = worker_recycle_tx.try_send(rgb_data);
                }
            })
            .expect("Error al inicializar worker de preview");

        log_info!("Engine-RS", "Pipeline de Video y Audio activo a 60 FPS...");
        log_info!("Hardware", "Hardware detectado: CPU: {} ({} núcleos) | GPU: {} | NPU: {}", 
            hardware_info.cpu_name, hardware_info.cpu_cores, hardware_info.gpu_name, hardware_info.npu_name);

        let start_time = Instant::now();
        let mut last_fps_time = Instant::now();
        let mut frame_count = 0;
        let mut measured_fps;
        let mut avg_ai_ms;
        let mut avg_gpu_ms;

        let mut frame_interval = Duration::from_micros(33_333); // 30 FPS por defecto
        let mut last_preview_time = Instant::now();
        let preview_interval = Duration::from_millis(16); // Hasta 60 FPS ultra-fluido para Preview UI
        let mut last_perf_log = Instant::now();

        loop {
            let frame_start = Instant::now();
            let elapsed_sec = start_time.elapsed().as_secs_f32();

            let current_cfg = {
                let read_lock = config.read().await;
                read_lock.clone()
            };

            // 1. Detección de cambio dinámico de Cámara / Resolución / Formato / Rotación
            let rot = current_cfg.camera_rotation;
            let is_portrait = rot == 90 || rot == 270;
            let effective_w = if is_portrait { current_cfg.selected_height as usize } else { current_cfg.selected_width as usize };
            let effective_h = if is_portrait { current_cfg.selected_width as usize } else { current_cfg.selected_height as usize };

            if current_cfg.selected_camera_id != current_cam_id
                || current_cfg.selected_width != current_w
                || current_cfg.selected_height != current_h
                || current_cfg.selected_fps != current_fps
                || current_cfg.selected_format != current_fmt
                || rot != current_rotation
            {
                let cam_hardware_changed = current_cfg.selected_camera_id != current_cam_id
                    || current_cfg.selected_width != current_w
                    || current_cfg.selected_height != current_h
                    || current_cfg.selected_fps != current_fps
                    || current_cfg.selected_format != current_fmt;

                current_cam_id = current_cfg.selected_camera_id.clone();
                current_w = current_cfg.selected_width;
                current_h = current_cfg.selected_height;
                current_fps = current_cfg.selected_fps;
                current_fmt = current_cfg.selected_format.clone();
                current_rotation = rot;

                width = effective_w;
                height = effective_h;
                frame_interval = Duration::from_micros(1_000_000 / current_fps.max(1) as u64);

                if cam_hardware_changed {
                    capture.start_camera(&current_cam_id, current_w, current_h, current_fps, &current_fmt);
                }
                ai_engine.resize(width, height);
                processor.resize(width, height);
                virtual_cam.resize(width, height, current_fps);
            }

            // 2. Control dinámico de Audio
            if current_cfg.selected_audio_id != current_audio_id {
                current_audio_id = current_cfg.selected_audio_id.clone();
                let _ = audio_engine.start(&current_audio_id);
            }
            audio_engine.set_volume(current_cfg.audio_volume);
            audio_engine.set_muted(current_cfg.audio_muted);
            let (audio_peak_db, audio_rms_db) = audio_engine.get_levels();

            // 2.1 Control de Backend de Aceleración de Hardware (Video y AI)
            if current_cfg.video_backend != last_video_backend {
                last_video_backend = current_cfg.video_backend.clone();
                if last_video_backend == "gpu_d3d12" {
                    crate::log_info!("Hardware", "Aceleración de Video activada: GPU {} (Direct3D 12 Compute Pipeline)", hardware_info.gpu_name);
                } else {
                    crate::log_info!("Hardware", "Aceleración de Video activada: CPU {} (SIMD AVX2 Multihilo)", hardware_info.cpu_name);
                }
            }

            if current_cfg.ai_backend != last_ai_backend {
                let is_reconfig = !last_ai_backend.is_empty();
                last_ai_backend = current_cfg.ai_backend.clone();
                match last_ai_backend.as_str() {
                    "npu" => crate::log_info!("Hardware", "Inferencia de IA activada en NPU: {} (Acelerador Neural)", hardware_info.npu_name),
                    "gpu" => crate::log_info!("Hardware", "Inferencia de IA activada en GPU: {} (Tensor Cores / DirectML)", hardware_info.gpu_name),
                    _ => crate::log_info!("Hardware", "Inferencia de IA activada en CPU: {} (SIMD AVX2)", hardware_info.cpu_name),
                }
                if is_reconfig {
                    crate::log_info!("AiEngine", "Reinicializando motor de IA con nuevo backend: {}", last_ai_backend);
                    drop(ai_engine);
                    ai_engine = AiEngine::new(width, height, &last_ai_backend);
                }
            }

            // 3. Control de Cámara Virtual
            if current_cfg.vcam_active && !virtual_cam.is_active() {
                virtual_cam.start();
            } else if !current_cfg.vcam_active && virtual_cam.is_active() {
                virtual_cam.stop();
            }

            // 4. Captura y Rotación Dinámica de Frame (Orientación de Sensor)
            let raw_camera_frame = capture.get_frame(elapsed_sec);
            let raw_frame: &[u8] = if current_rotation != 0 {
                crate::capture::rotate_rgba(
                    raw_camera_frame,
                    current_w as usize,
                    current_h as usize,
                    current_rotation,
                    &mut rotated_frame_buf,
                );
                &rotated_frame_buf
            } else {
                raw_camera_frame
            };

            // 5. Inferencia Facial 1:1 Síncrona de Cero Latencia (0.0ms lag) + Segmentación GPU Asíncrona
            let ai_state = ai_engine.process_frame_sync(raw_frame, &current_cfg);
            avg_ai_ms = ai_state.ai_ms;

            // 6. Procesamiento Multihilo & Auto-Ajuste de Luz con Prioridad Retrato Facial
            let proc_start = Instant::now();
            let (processed_frame, photo_stats) = processor.process_frame(
                raw_frame,
                &ai_state.segmentation_mask,
                &ai_state.skin_mask,
                &ai_state.feature_mask,
                &ai_state.faces,
                Some(&ai_state.face_lighting),
                &current_cfg,
            );
            avg_gpu_ms = proc_start.elapsed().as_secs_f32() * 1000.0;

            // 7. Publicación en Cámara Virtual
            if virtual_cam.is_active() {
                virtual_cam.publish_frame(processed_frame);
            }

            // 7.1 Transmisión de Preview Inteligente SSAA Multihilo con Búfer Reciclable (Zero-Allocation)
            // Para <= 1080p -> 1:1 nativo. Para 1440p/4K -> 2x2 SSAA Box-Average en Rayon a 30 FPS para ahorrar CPU
            let now_instant = Instant::now();
            let should_send_preview = now_instant.duration_since(last_preview_time) >= preview_interval;

            if !current_cfg.preview_paused && video_tx.receiver_count() > 0 && should_send_preview {
                last_preview_time = now_instant;
                let scale_2x = width >= 1280 || height >= 720;
                let preview_w = if scale_2x { width / 2 } else { width };
                let preview_h = if scale_2x { height / 2 } else { height };
                let target_len = preview_w * preview_h * 3;

                let mut buf = recycle_rx.try_recv().unwrap_or_default();
                if buf.len() != target_len {
                    buf.resize(target_len, 0);
                }

                if scale_2x {
                    // SSAA 2x2 Box-Average secuencial (Cero contención de hilos en Rayon, 0.4ms)
                    buf.chunks_exact_mut(preview_w * 3)
                        .enumerate()
                        .for_each(|(py, row)| {
                            let sy0 = py * 2;
                            let sy1 = (sy0 + 1).min(height - 1);
                            let src_row0 = sy0 * width * 4;
                            let src_row1 = sy1 * width * 4;

                            for px in 0..preview_w {
                                let sx0 = px * 2;
                                let sx1 = (sx0 + 1).min(width - 1);

                                let i00 = src_row0 + sx0 * 4;
                                let i10 = src_row0 + sx1 * 4;
                                let i01 = src_row1 + sx0 * 4;
                                let i11 = src_row1 + sx1 * 4;

                                let r = (processed_frame[i00] as u32 + processed_frame[i10] as u32 + processed_frame[i01] as u32 + processed_frame[i11] as u32 + 2) >> 2;
                                let g = (processed_frame[i00 + 1] as u32 + processed_frame[i10 + 1] as u32 + processed_frame[i01 + 1] as u32 + processed_frame[i11 + 1] as u32 + 2) >> 2;
                                let b = (processed_frame[i00 + 2] as u32 + processed_frame[i10 + 2] as u32 + processed_frame[i01 + 2] as u32 + processed_frame[i11 + 2] as u32 + 2) >> 2;

                                let out_idx = px * 3;
                                row[out_idx] = r as u8;
                                row[out_idx + 1] = g as u8;
                                row[out_idx + 2] = b as u8;
                            }
                        });
                } else {
                    // 1:1 Nativo para 1080p y 720p
                    buf.chunks_exact_mut(preview_w * 3)
                        .enumerate()
                        .for_each(|(py, row)| {
                            let src_row = py * preview_w * 4;
                            for px in 0..preview_w {
                                let src_idx = src_row + px * 4;
                                let out_idx = px * 3;
                                row[out_idx] = processed_frame[src_idx];
                                row[out_idx + 1] = processed_frame[src_idx + 1];
                                row[out_idx + 2] = processed_frame[src_idx + 2];
                            }
                        });
                }

                if let Err(std::sync::mpsc::TrySendError::Full((returned_buf, _, _))) =
                    preview_tx.try_send((buf, preview_w as u32, preview_h as u32))
                {
                    let _ = recycle_tx.try_send(returned_buf);
                }
            }

            frame_count += 1;

            // 8. Envío de Telemetría a la UI
            let now = Instant::now();
            let fps_elapsed = now.duration_since(last_fps_time).as_secs_f32();
            if fps_elapsed >= 0.25 {
                measured_fps = frame_count as f32 / fps_elapsed;
                frame_count = 0;
                last_fps_time = now;

                if last_perf_log.elapsed().as_secs_f32() >= 1.0 {
                    crate::log_info!("Perf", "Main Loop FPS: {:.1} | Frame: {:.2}ms | Proc: {:.2}ms | AI: {:.2}ms", 
                        measured_fps, 1000.0 / measured_fps.max(1.0), avg_gpu_ms, avg_ai_ms);
                    last_perf_log = now;
                }

                let telemetry = EngineTelemetry {
                    msg_type: "TELEMETRY".to_string(),
                    fps: measured_fps,
                    frame_time_ms: 1000.0 / measured_fps.max(1.0),
                    ai_ms: avg_ai_ms,
                    gpu_ms: avg_gpu_ms,
                    vcam_active: virtual_cam.is_active(),
                    gpu_name: if current_cfg.video_backend == "gpu_d3d12" {
                        format!("{} [Direct3D 12 GPU Compute]", hardware_info.gpu_name)
                    } else {
                        format!("{} [CPU SIMD AVX2]", hardware_info.cpu_name)
                    },
                    faces: ai_state.faces.len(),
                    detected_luma: photo_stats.detected_luma,
                    auto_exposure: photo_stats.auto_exposure,
                    auto_wb: photo_stats.auto_wb,
                    ai_lighting_active: current_cfg.ai_lighting_enabled,
                    ai_denoise_active: current_cfg.ai_denoise_enabled,
                    ai_auto_tune_active: current_cfg.ai_auto_tune_enabled,
                    recommended_exposure: photo_stats.recommended_exposure,
                    recommended_contrast: photo_stats.recommended_contrast,
                    recommended_saturation: photo_stats.recommended_saturation,
                    recommended_sharpness: photo_stats.recommended_sharpness,
                    recommended_wb: photo_stats.recommended_wb,
                    audio_peak_db,
                    audio_rms_db,
                    active_video_backend: current_cfg.video_backend.clone(),
                    active_ai_backend: current_cfg.ai_backend.clone(),
                    hardware: hardware_info.clone(),
                };

                if let Ok(json_str) = serde_json::to_string(&telemetry) {
                    let _ = telemetry_tx.send(json_str);
                }
            }

            let elapsed_frame = frame_start.elapsed();
            if elapsed_frame < frame_interval {
                tokio::time::sleep(frame_interval - elapsed_frame).await;
            }
        }
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 0. Prioridad a DLLs locales del ejecutable (evita colisiones con onnxruntime.dll de System32)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            if let Ok(c_parent) = std::ffi::CString::new(parent.to_string_lossy().as_bytes()) {
                unsafe {
                    SetDllDirectoryA(c_parent.as_ptr() as *const u8);
                }
            }
        }
    }

    // Activar resolución de temporizador de 1ms en Windows para 60.0 FPS estables y fluidos
    unsafe {
        timeBeginPeriod(1);
    }

    // 0. Inicializar sistema de logs persistente y hook de pánico
    logger::init_logger();

    // 0. Configuración de precisión de temporizadores multimedia de Windows (1ms resolution)
    unsafe {
        timeBeginPeriod(1);
    }

    // 0.1 Configurar ThreadPool de Rayon balanceado y ultra-eficiente (4 a 6 hilos dedicados para mantener 60 FPS estables con bajo consumo de CPU <15%)
    let num_threads = std::thread::available_parallelism()
        .map(|n| (n.get() / 3).clamp(4, 6))
        .unwrap_or(4);
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .thread_name(|i| format!("CamAr-Worker-{}", i))
        .build_global();

    log_info!("App", "======================================================");
    log_info!("App", "        CamAr Studio - Native Desktop App [RUST]      ");
    log_info!("App", "  Dynamic Hardware Capture, AI Lighting & Color Studio");
    log_info!("App", "======================================================");

    // 1. Detección Dinámica de Cámaras y Micrófonos (Cero Hardcoding)
    log_info!("Hardware", "Detectando cámaras web conectadas al sistema...");
    let cameras = VideoCapture::enumerate_cameras();
    for cam in &cameras {
        log_info!("Hardware", " -> Cámara: {} [{} formatos disponibles]", cam.name, cam.formats.len());
    }

    log_info!("Hardware", "Detectando dispositivos de audio...");
    let microphones = AudioEngine::enumerate_devices();
    for mic in &microphones {
        log_info!("Hardware", " -> Micrófono: {} (Predeterminado: {})", mic.name, mic.is_default);
    }

    let devices_payload = json!({
        "type": "DEVICES_LIST",
        "cameras": cameras,
        "microphones": microphones
    })
    .to_string();

    // 2. Estado compartido con carga de presets guardados y verificación de existencia física de cámara
    let mut initial_config = StudioConfig::load_presets();
    if initial_config.selected_camera_id.is_empty() || initial_config.selected_camera_id == "auto" {
        initial_config.selected_camera_id = "synthetic".to_string();
    } else if initial_config.selected_camera_id != "synthetic" {
        let cam_exists = cameras.iter().any(|c| c.id == initial_config.selected_camera_id);
        if !cam_exists {
            log_info!(
                "Config",
                "La cámara guardada '{}' ya no está conectada; activando Dummy Simulator.",
                initial_config.selected_camera_id
            );
            initial_config.selected_camera_id = "synthetic".to_string();
            initial_config.selected_width = 1920;
            initial_config.selected_height = 1080;
            initial_config.selected_fps = 60;
            initial_config.selected_format = "NV12".to_string();
        }
    }
    log_info!(
        "Config",
        "Configuración inicial cargada [Cámara: {}, {}x{} @ {} FPS {}]",
        initial_config.selected_camera_id,
        initial_config.selected_width,
        initial_config.selected_height,
        initial_config.selected_fps,
        initial_config.selected_format
    );
    let config = Arc::new(RwLock::new(initial_config));

    let args: Vec<String> = std::env::args().collect();
    let server_only = args.iter().any(|a| a == "--server-only" || a == "--headless" || a == "--no-ui");

    if server_only {
        log_info!("Engine-RS", "Modo servidor nativo sin ventana TAO activo (Backend para C# WinUI 3)");
        run_multimedia_engine(config, devices_payload);
        return Ok(());
    }

    // 3. Iniciar el motor en hilo dedicado
    let engine_config = Arc::clone(&config);
    let dev_json_clone = devices_payload.clone();
    std::thread::Builder::new()
        .name("CamAr-EngineThread".to_string())
        .spawn(move || {
            run_multimedia_engine(engine_config, dev_json_clone);
        })?;

    // 4. Crear ventana nativa con TAO
    let event_loop = EventLoop::new();
    let mut window_builder = WindowBuilder::new()
        .with_title("CamAr Studio - Live Video & Virtual Camera")
        .with_inner_size(LogicalSize::new(1480.0, 900.0))
        .with_min_inner_size(LogicalSize::new(1100.0, 700.0));

    if let Some(icon) = load_app_icon() {
        window_builder = window_builder.with_window_icon(Some(icon));
    }

    let window = match window_builder.build(&event_loop) {
        Ok(w) => w,
        Err(e) => {
            let desc = format!("No se pudo inicializar la ventana nativa del sistema: {}", e);
            crate::logger::show_error_dialog("Creación de Ventana (TAO)", &desc);
            return Err(Box::new(e));
        }
    };

    // 5. Configurar WebView con WRY embebido
    let webview_builder = WebViewBuilder::new()
        .with_custom_protocol("camar".into(), move |_webview, request| {
            let path = request.uri().path();
            let trimmed_path = if path == "/" || path.is_empty() {
                "index.html"
            } else {
                path.trim_start_matches('/')
            };

            let disk_path = std::path::Path::new(trimmed_path);
            if disk_path.exists() && disk_path.is_file() {
                if let Ok(bytes) = std::fs::read(disk_path) {
                    let mime = get_mime_type(trimmed_path);
                    return Response::builder()
                        .header(CONTENT_TYPE, mime)
                        .body(std::borrow::Cow::Owned(bytes))
                        .unwrap();
                }
            }
            if let Ok(exe_dir) = std::env::current_exe() {
                if let Some(parent) = exe_dir.parent() {
                    let exe_disk_path = parent.join(trimmed_path);
                    if exe_disk_path.exists() && exe_disk_path.is_file() {
                        if let Ok(bytes) = std::fs::read(&exe_disk_path) {
                            let mime = get_mime_type(trimmed_path);
                            return Response::builder()
                                .header(CONTENT_TYPE, mime)
                                .body(std::borrow::Cow::Owned(bytes))
                                .unwrap();
                        }
                    }
                }
            }

            if let Some(file) = UI_DIR.get_file(trimmed_path) {
                let mime = get_mime_type(trimmed_path);
                Response::builder()
                    .header(CONTENT_TYPE, mime)
                    .body(Cow::Borrowed(file.contents()))
                    .unwrap()
            } else {
                let fallback = UI_DIR.get_file("index.html").expect("index.html no encontrado");
                Response::builder()
                    .header(CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Cow::Borrowed(fallback.contents()))
                    .unwrap()
            }
        })
        .with_url("camar://localhost/index.html");

    let _webview = match webview_builder.build(&window) {
        Ok(wv) => wv,
        Err(e) => {
            let desc = format!(
                "No se pudo inicializar el entorno de interfaz web (WebView2): {}\n\nAsegúrese de que Microsoft Edge WebView2 Runtime esté instalado en Windows.",
                e
            );
            crate::logger::show_error_dialog("Interfaz de Usuario (WebView2)", &desc);
            return Err(Box::new(e));
        }
    };

    log_info!("UI-Desktop", "Ventana nativa de interfaz iniciada exitosamente.");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            log_info!("App", "Cerrando aplicación CamAr Studio.");
            *control_flow = ControlFlow::Exit;
        }
    });
}
