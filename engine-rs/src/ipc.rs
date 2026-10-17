use crate::types::{BackgroundMode, CameraHardwareProfile, ColorRange, ColorSpace, StudioConfig};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;

pub struct IpcServer {
    config: Arc<RwLock<StudioConfig>>,
    telemetry_tx: broadcast::Sender<String>,
    video_tx: broadcast::Sender<Vec<u8>>,
    devices_json: Arc<String>,
}

impl IpcServer {
    pub fn new(
        config: Arc<RwLock<StudioConfig>>,
        devices_json: String,
    ) -> (Self, broadcast::Sender<String>, broadcast::Sender<Vec<u8>>) {
        let (tx, _rx) = broadcast::channel(16);
        let (vtx, _vrx) = broadcast::channel(4);
        let server = Self {
            config,
            telemetry_tx: tx.clone(),
            video_tx: vtx.clone(),
            devices_json: Arc::new(devices_json),
        };
        (server, tx, vtx)
    }

    pub async fn run(self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(&addr).await?;
        crate::log_info!("IPC-RS", "Servidor WebSocket escuchando en ws://127.0.0.1:{}", port);

        while let Ok((stream, _)) = listener.accept().await {
            let config_clone = Arc::clone(&self.config);
            let tx_clone = self.telemetry_tx.clone();
            let vtx_clone = self.video_tx.clone();
            let devices_json_clone = Arc::clone(&self.devices_json);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, config_clone, tx_clone, vtx_clone, devices_json_clone).await {
                    crate::log_error!("IPC-RS", "Error en conexión cliente: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        config: Arc<RwLock<StudioConfig>>,
        telemetry_tx: broadcast::Sender<String>,
        video_tx: broadcast::Sender<Vec<u8>>,
        devices_json: Arc<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        crate::log_info!("IPC-RS", "Cliente UI conectado exitosamente.");

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let mut telemetry_rx = telemetry_tx.subscribe();
        let mut video_rx = video_tx.subscribe();

        // 1. Enviar PRIMERO el preset guardado para restaurar la cámara y configuración antes de procesar dispositivos
        {
            let current_cfg = config.read().await;
            if let Ok(preset_json) = serde_json::to_string(&serde_json::json!({
                "type": "LOAD_PRESET",
                "config": *current_cfg
            })) {
                let _ = ws_sender.send(Message::Text(preset_json)).await;
            }
        }

        // 2. Enviar la lista de dispositivos detectados (cámaras reales y micrófonos)
        let _ = ws_sender.send(Message::Text((*devices_json).clone())).await;

        // Tarea multiplexada para enviar telemetría de texto y frames de video binarios JPEG
        let send_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    res = telemetry_rx.recv() => {
                        match res {
                            Ok(msg) => {
                                if ws_sender.send(Message::Text(msg)).await.is_err() {
                                    break;
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(broadcast::error::RecvError::Closed) => break,
                        }
                    }
                    res = video_rx.recv() => {
                        match res {
                            Ok(frame) => {
                                if ws_sender.send(Message::Binary(frame)).await.is_err() {
                                    break;
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(broadcast::error::RecvError::Closed) => break,
                        }
                    }
                }
            }
        });

        // Loop de recepción de comandos desde la UI
        while let Some(msg) = ws_receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    let cmd = v["command"].as_str().unwrap_or("");
                    let mut cfg = config.write().await;

                    match cmd {
                        "PAUSE_PREVIEW" => {
                            cfg.preview_paused = true;
                            crate::log_info!("Preview", "Preview en pausa por selección de fuente");
                        }
                        "RESUME_PREVIEW" => {
                            cfg.preview_paused = false;
                            crate::log_info!("Preview", "Preview reanudado");
                        }
                        "UPDATE_CONFIG" => {
                            if let Some(s) = v["saturation"].as_f64() { cfg.saturation = s as f32; }
                            if let Some(c) = v["contrast"].as_f64() { cfg.contrast = c as f32; }
                            if let Some(sh) = v["sharpness"].as_f64() { cfg.sharpness = sh as f32; }
                            if let Some(h) = v["hue"].as_f64() { cfg.hue = h as f32; }
                            if let Some(wb) = v["whiteBalance"].as_f64() { cfg.white_balance = wb as f32; }
                            if let Some(e) = v["exposure"].as_f64() { cfg.exposure = e as f32; }
                            if let Some(b) = v["brightness"].as_f64() { cfg.brightness = b as f32; }

                            if let Some(cr) = v["colorRange"].as_i64() {
                                cfg.color_range = if cr == 1 { ColorRange::Limited } else { ColorRange::Full };
                            }
                            if let Some(cs) = v["colorSpace"].as_i64() {
                                cfg.color_space = match cs {
                                    1 => ColorSpace::SRgb,
                                    2 => ColorSpace::Rec601,
                                    3 => ColorSpace::Rec2020,
                                    _ => ColorSpace::Rec709,
                                };
                            }

                            if let Some(l) = v["aiLightingEnabled"].as_bool() { cfg.ai_lighting_enabled = l; }
                            if let Some(d) = v["aiDenoiseEnabled"].as_bool() { cfg.ai_denoise_enabled = d; }
                            if let Some(at) = v["aiAutoTuneEnabled"].as_bool() { cfg.ai_auto_tune_enabled = at; }

                            if let Some(be) = v["beautyEnabled"].as_bool() { cfg.beauty_enabled = be; }
                            if let Some(ss) = v["skinSmoothing"].as_f64() { cfg.skin_smoothing = ss as f32; }
                            if let Some(sw) = v["skinWhitening"].as_f64() { cfg.skin_whitening = sw as f32; }
                            if let Some(fs) = v["faceSlim"].as_f64() { cfg.face_slim = fs as f32; }
                            if let Some(ee) = v["eyeEnlarge"].as_f64() { cfg.eye_enlarge = ee as f32; }
                            if let Some(er) = v["eyeRound"].as_f64() { cfg.eye_round = er as f32; }
                            if let Some(eb) = v["eyeBrighten"].as_f64() { cfg.eye_brighten = eb as f32; }
                            if let Some(ec) = v["eyeCatchlight"].as_f64() { cfg.eye_catchlight = ec as f32; }
                            if let Some(el) = v["eyelashes"].as_f64() { cfg.eyelashes = el as f32; }
                            if let Some(il) = v["irisLighting"].as_f64() { cfg.iris_lighting = il as f32; }
                            if let Some(ic) = v["irisColor"].as_str() { cfg.iris_color = ic.to_string(); }
                            if let Some(bl) = v["browLift"].as_f64() { cfg.brow_lift = bl as f32; }
                            if let Some(ns) = v["noseSlim"].as_f64() { cfg.nose_slim = ns as f32; }
                            if let Some(lp) = v["lipPlump"].as_f64() { cfg.lip_plump = lp as f32; }
                            if let Some(ft) = v["foundationTone"].as_str() { cfg.foundation_tone = ft.to_string(); }
                            if let Some(fi) = v["foundationIntensity"].as_f64() { cfg.foundation_intensity = fi as f32; }
                            if let Some(tw) = v["teethWhitening"].as_f64() { cfg.teeth_whitening = tw as f32; }
                            if let Some(ls) = v["lipstick"].as_f64() { cfg.lipstick = ls as f32; }
                            if let Some(lc) = v["lipstickColor"].as_str() { cfg.lipstick_color = lc.to_string(); }
                            if let Some(es) = v["eyeshadow"].as_f64() { cfg.eyeshadow = es as f32; }
                            if let Some(ec) = v["eyeshadowColor"].as_str() { cfg.eyeshadow_color = ec.to_string(); }
                            if let Some(est) = v["eyeshadowStyle"].as_str() { cfg.eyeshadow_style = est.to_string(); }
                            if let Some(ct) = v["contour"].as_f64() { cfg.contour = ct as f32; }

                            if let Some(bg) = v["bgMode"].as_i64() { cfg.bg_mode = BackgroundMode::from(bg as i32); }
                            if let Some(br) = v["blurRadius"].as_f64() { cfg.blur_radius = br as f32; }
                            if let Some(ef) = v["edgeFeather"].as_f64() { cfg.edge_feather = ef as f32; }
                            if let Some(st) = v["segThreshold"].as_f64() { cfg.seg_threshold = st as f32; }

                            if let Some(fd) = v["faceDetection"].as_bool() { cfg.face_detection = fd; }
                            if let Some(ps) = v["personSegmentation"].as_bool() { cfg.person_segmentation = ps; }
                            if let Some(dfb) = v["drawFaceBox"].as_bool() { cfg.draw_face_box = dfb; }
                            if let Some(dfl) = v["drawFaceLandmarks"].as_bool() { cfg.draw_face_landmarks = dfl; }

                            if let Some(vb) = v["videoBackend"].as_str() { cfg.video_backend = vb.to_string(); }
                            if let Some(ab) = v["aiBackend"].as_str() { cfg.ai_backend = ab.to_string(); }

                            if let Some(fl) = v["filterLut"].as_str() { cfg.filter_lut = fl.to_string(); }
                            if let Some(fi) = v["filterIntensity"].as_f64() { cfg.filter_intensity = fi as f32; }

                            // Solo actualizar dimensiones/FPS en UPDATE_CONFIG si corresponden a la cámara actualmente activa
                            let incoming_cam = v["selectedCameraId"].as_str().unwrap_or("");
                            if incoming_cam.is_empty() || incoming_cam == cfg.selected_camera_id {
                                if let Some(w) = v["selectedWidth"].as_u64() { cfg.selected_width = w as u32; }
                                if let Some(h) = v["selectedHeight"].as_u64() { cfg.selected_height = h as u32; }
                                if let Some(fps) = v["selectedFps"].as_u64() { cfg.selected_fps = fps as u32; }
                                if let Some(fmt) = v["selectedFormat"].as_str() { cfg.selected_format = fmt.to_string(); }
                            }

                            if let Some(favs) = v["favoritePresets"].as_array() {
                                cfg.favorite_presets = favs.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                            }
                            if let Some(pid) = v["activePresetId"].as_str() {
                                cfg.active_preset_id = pid.to_string();
                            }
                            if let Some(mi) = v["makeupIntensity"].as_f64() {
                                cfg.makeup_intensity = mi as f32;
                            }
                            if let Some(ba) = v.get("baseAppearance") {
                                if let Ok(parsed_ba) = serde_json::from_value::<crate::types::BaseAppearanceConfig>(ba.clone()) {
                                    cfg.base_appearance = parsed_ba;
                                } else {
                                    cfg.sync_base_appearance_from_top_level();
                                }
                            } else {
                                cfg.sync_base_appearance_from_top_level();
                            }

                            // Actualizar el perfil guardado de la cámara activa
                            if !cfg.selected_camera_id.is_empty() {
                                let cam_key = cfg.selected_camera_id.clone();
                                let active_profile = CameraHardwareProfile {
                                    width: cfg.selected_width,
                                    height: cfg.selected_height,
                                    fps: cfg.selected_fps,
                                    format: cfg.selected_format.clone(),
                                    saturation: cfg.saturation,
                                    contrast: cfg.contrast,
                                    sharpness: cfg.sharpness,
                                    hue: cfg.hue,
                                    white_balance: cfg.white_balance,
                                    exposure: cfg.exposure,
                                    brightness: cfg.brightness,
                                    color_range: cfg.color_range,
                                    color_space: cfg.color_space,
                                };
                                cfg.camera_profiles.insert(cam_key, active_profile);
                            }

                            cfg.save_hardware();
                            cfg.save_beauty();
                        }
                        "SELECT_CAMERA" => {
                            let new_cam_id = v["cameraId"].as_str().unwrap_or("synthetic").to_string();
                            let is_same_camera = new_cam_id == cfg.selected_camera_id;

                            if !is_same_camera {
                                // 1. Guardar calibración de la cámara previa
                                if !cfg.selected_camera_id.is_empty() {
                                    let prev_cam_key = cfg.selected_camera_id.clone();
                                    let prev_profile = CameraHardwareProfile {
                                        width: cfg.selected_width,
                                        height: cfg.selected_height,
                                        fps: cfg.selected_fps,
                                        format: cfg.selected_format.clone(),
                                        saturation: cfg.saturation,
                                        contrast: cfg.contrast,
                                        sharpness: cfg.sharpness,
                                        hue: cfg.hue,
                                        white_balance: cfg.white_balance,
                                        exposure: cfg.exposure,
                                        brightness: cfg.brightness,
                                        color_range: cfg.color_range,
                                        color_space: cfg.color_space,
                                    };
                                    cfg.camera_profiles.insert(prev_cam_key, prev_profile);
                                }

                                // 2. Cambiar identificador de cámara
                                cfg.selected_camera_id = new_cam_id.clone();

                                // 3. Restaurar perfil si ya existía para esta cámara específica, o inicializarlo
                                if let Some(prof) = cfg.camera_profiles.get(&new_cam_id).cloned() {
                                    cfg.selected_width = v["width"].as_u64().map(|w| w as u32).unwrap_or(prof.width);
                                    cfg.selected_height = v["height"].as_u64().map(|h| h as u32).unwrap_or(prof.height);
                                    cfg.selected_fps = v["fps"].as_u64().map(|f| f as u32).unwrap_or(prof.fps);
                                    cfg.selected_format = v["format"].as_str().map(|f| f.to_string()).unwrap_or(prof.format);
                                    cfg.saturation = prof.saturation;
                                    cfg.contrast = prof.contrast;
                                    cfg.sharpness = prof.sharpness;
                                    cfg.hue = prof.hue;
                                    cfg.white_balance = prof.white_balance;
                                    cfg.exposure = prof.exposure;
                                    cfg.brightness = prof.brightness;
                                    cfg.color_range = prof.color_range;
                                    cfg.color_space = prof.color_space;
                                } else {
                                    if let Some(w) = v["width"].as_u64() { cfg.selected_width = w as u32; }
                                    if let Some(h) = v["height"].as_u64() { cfg.selected_height = h as u32; }
                                    if let Some(fps) = v["fps"].as_u64() { cfg.selected_fps = fps as u32; }
                                    if let Some(fmt) = v["format"].as_str() { cfg.selected_format = fmt.to_string(); }
                                    cfg.saturation = 1.0;
                                    cfg.contrast = 1.0;
                                    cfg.sharpness = 0.2;
                                    cfg.hue = 0.0;
                                    cfg.white_balance = 0.0;
                                    cfg.exposure = 0.0;
                                    cfg.brightness = 0.0;
                                    cfg.color_range = ColorRange::Full;
                                    cfg.color_space = ColorSpace::Rec709;
                                }
                            } else {
                                // Misma cámara: el usuario seleccionó expresamente nueva resolución, FPS o formato!
                                if let Some(w) = v["width"].as_u64() { cfg.selected_width = w as u32; }
                                if let Some(h) = v["height"].as_u64() { cfg.selected_height = h as u32; }
                                if let Some(fps) = v["fps"].as_u64() { cfg.selected_fps = fps as u32; }
                                if let Some(fmt) = v["format"].as_str() { cfg.selected_format = fmt.to_string(); }
                            }

                            // Actualizar el perfil activo para esta cámara con la nueva resolución/FPS
                            let active_profile = CameraHardwareProfile {
                                width: cfg.selected_width,
                                height: cfg.selected_height,
                                fps: cfg.selected_fps,
                                format: cfg.selected_format.clone(),
                                saturation: cfg.saturation,
                                contrast: cfg.contrast,
                                sharpness: cfg.sharpness,
                                hue: cfg.hue,
                                white_balance: cfg.white_balance,
                                exposure: cfg.exposure,
                                brightness: cfg.brightness,
                                color_range: cfg.color_range,
                                color_space: cfg.color_space,
                            };
                            cfg.camera_profiles.insert(new_cam_id, active_profile);

                            // Guardar ÚNICAMENTE camar_hardware.json (la belleza en camar_beauty.json queda 100% intacta)
                            cfg.save_hardware();

                            crate::log_info!(
                                "IPC-RS",
                                "Cámara configurada a {}x{} @ {} FPS [{}]",
                                cfg.selected_width, cfg.selected_height, cfg.selected_fps, cfg.selected_format
                            );

                            // 4. Sincronizar inmediatamente la UI con el perfil cargado/actualizado
                            if let Ok(preset_json) = serde_json::to_string(&serde_json::json!({
                                "type": "LOAD_PRESET",
                                "config": *cfg
                            })) {
                                let _ = telemetry_tx.send(preset_json);
                            }
                        }
                        "SELECT_AUDIO" => {
                            if let Some(audio_id) = v["audioId"].as_str() { cfg.selected_audio_id = audio_id.to_string(); }
                            cfg.save_hardware();
                        }
                        "SET_AUDIO_VOLUME" => {
                            if let Some(vol) = v["volume"].as_f64() { cfg.audio_volume = vol as f32; }
                            cfg.save_hardware();
                        }
                        "TOGGLE_MUTE" => {
                            if let Some(m) = v["muted"].as_bool() { cfg.audio_muted = m; } else { cfg.audio_muted = !cfg.audio_muted; }
                            cfg.save_hardware();
                        }
                        "TOGGLE_VCAM" => {
                            if let Some(active) = v["active"].as_bool() {
                                cfg.vcam_active = active;
                            } else {
                                cfg.vcam_active = !cfg.vcam_active;
                            }
                        }
                        "SAVE_PRESET_FILE" => {
                            if let Some(preset_val) = v.get("preset") {
                                let preset_id = preset_val["id"].as_str().unwrap_or("custom_preset").to_string();
                                cfg.custom_presets.insert(preset_id.clone(), preset_val.clone());
                                cfg.active_preset_id = preset_id.clone();
                                if !cfg.favorite_presets.contains(&preset_id) {
                                    cfg.favorite_presets.push(preset_id.clone());
                                }
                                cfg.save_beauty();
                                crate::log_info!("Presets", "Preset personalizado '{}' integrado en camar_beauty.json", preset_id);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        send_task.abort();
        crate::log_info!("IPC-RS", "Cliente UI desconectado.");
        Ok(())
    }
}
