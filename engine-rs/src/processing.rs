use crate::types::{BackgroundMode, ColorRange, ColorSpace, FaceLandmarks, FaceLighting, Point2f, StudioConfig};
use rayon::prelude::*;

pub mod color;
pub mod geometry;
pub mod sprites;
pub mod eyes;
pub mod eyebrows;
pub mod lips;
pub mod photometry;
pub mod face_mesh_data;
pub mod face_mesh;
pub mod effects;
pub mod mesh3d;
pub mod beard;

pub use face_mesh::*;
#[allow(unused_imports)]
pub use effects::*;
pub use mesh3d::*;
pub use beard::*;

pub use color::*;
pub use geometry::*;
pub use sprites::*;
pub use eyes::*;
pub use eyebrows::*;
pub use lips::*;
pub use photometry::*;

pub struct VideoProcessor {
    width: usize,
    height: usize,
    output_buffer: Vec<u8>,
    luma_accumulator: f32,
    gaussian_lut: [f32; 256],
    srgb_lut: [f32; 1024],
    /// Sprites RGBA 256x256 (.png) de Pupilentes con fibras de iris, criptas alfa, collarette y agujero de pupila:
    /// [0]=honey, [1]=hazel, [2]=emerald, [3]=ice_blue, [4]=sapphire, [5]=gray, [6]=violet
    lens_sprites_rgba: Vec<Vec<[f32; 4]>>,
    /// Sprites RGBA 256x256 (.png) de Sombras de Ojos deformables con canal Alpha:
    /// [0]=natural_nude, [1]=smoky_glam, [2]=cat_wing, [3]=sunset_glow
    eyeshadow_sprites_rgba: Vec<Vec<[f32; 4]>>,
    /// Sprites RGBA 256x128 (.png) de Cejas Microblading / Ombré con canal Alpha:
    /// [0]=natural_fill, [1]=defined_arch, [2]=korean_straight, [3]=fluffy_feathered
    #[allow(dead_code)]
    eyebrow_sprites_rgba: Vec<Vec<[f32; 4]>>,
    /// Motor de Renderizado 3D de Malla Facial Canónica UV (468 vértices, 898 triángulos)
    face_mesh_renderer: FaceMeshRenderer,
    /// Textura Maestra de Maquillaje RGBA 1024x1024 (Cejas, Ojeras, Sombras, Rubor, Labios)
    master_makeup_rgba: Vec<[f32; 4]>,
    /// Sprite 2D pre-generado 128x128 de soporte para modo Luz de Iris sin color
    contact_lens_sprite: Vec<[f32; 4]>,
    /// Estabilizador temporal adaptativo 1€ de los discos de iris [(cx, cy, rad, iris_lum, sclera_lum); 2]
    smoothed_iris_discs: Option<[(f32, f32, f32, f32, f32); 2]>,
    /// Sprite 2D pre-generado 128x64 de Labios Deformable por 4 Curvas (Upper 0..32, Lower 32..64):
    /// [0] = outer_alpha (máscara alpha difuminada en borde exterior y comisuras laterales)
    /// [1] = cushion_3d (textura vertical de pliegues labiales + volumen 3D central)
    /// [2] = gloss_spec (brillo satinado 3D en cojín labial inferior y arco de Cupido)
    /// [3] = liner_shade (perfilado sutil de contorno labial exterior)
    lip_sprite: Vec<[f32; 4]>,
    /// Estabilizador temporal adaptativo 1€ de las 4 curvas de labios (elimina micro-temblor al 100% de opacidad)
    smoothed_lip_curves: Option<[[(f32, f32); 11]; 4]>,
    /// Malla 3D Low-Poly de Cuernos de Diablo (.obj Wavefront)
    devil_horns_mesh: Mesh3D,
    /// Motor de Renderizado 3D de Mallas con Z-Buffer y Shading Blinn-Phong/Fresnel
    mesh3d_renderer: Mesh3DRenderer,
    /// Orientación 3D suavizada (Yaw, Pitch, Roll, Centro 3D)
    smoothed_head_pose: Option<SmoothedHeadPose3D>,
    // Amortiguación cinemática EMA (SnapJM / iPhone ISP) para cero parpadeo
    current_exp_gain: f32,
    current_wb_r: f32,
    current_wb_g: f32,
    current_wb_b: f32,
    current_contrast: f32,
    // Histeresis y estabilización temporal de iluminación (Zero-Flicker por Movimiento)
    locked_auto_exp: f32,
    locked_wb_r: f32,
    locked_wb_g: f32,
    locked_wb_b: f32,
    locked_contrast: f32,
    lighting_hold_frames: u32,
    lighting_initialized: bool,
}

impl VideoProcessor {
    pub fn new(width: usize, height: usize) -> Self {
        let mut gaussian_lut = [0.0f32; 256];
        let two_sigma_sq = 2.0 * 0.075 * 0.075; // 0.01125
        for i in 0..256 {
            let diff = i as f32 / 255.0;
            gaussian_lut[i] = (-diff * diff / two_sigma_sq).exp();
        }

        let mut srgb_lut = [0.0f32; 1024];
        for i in 0..1024 {
            let x = i as f32 / 1023.0;
            srgb_lut[i] = if x <= 0.0031308 {
                12.92 * x
            } else {
                1.055 * x.powf(1.0 / 2.4) - 0.055
            };
        }

        // Carga de los 7 recursos gráficos reales (.png RGBA 256x256) de Pupilentes con canal Alpha
        let lens_sprites_rgba = vec![
            load_rgba_sprite("lenses", "honey", include_bytes!("../assets/sprites/lenses/honey.png"), 256, 256),
            load_rgba_sprite("lenses", "hazel", include_bytes!("../assets/sprites/lenses/hazel.png"), 256, 256),
            load_rgba_sprite("lenses", "emerald", include_bytes!("../assets/sprites/lenses/emerald.png"), 256, 256),
            load_rgba_sprite("lenses", "ice_blue", include_bytes!("../assets/sprites/lenses/ice_blue.png"), 256, 256),
            load_rgba_sprite("lenses", "sapphire", include_bytes!("../assets/sprites/lenses/sapphire.png"), 256, 256),
            load_rgba_sprite("lenses", "gray", include_bytes!("../assets/sprites/lenses/gray.png"), 256, 256),
            load_rgba_sprite("lenses", "violet", include_bytes!("../assets/sprites/lenses/violet.png"), 256, 256),
        ];

        // Carga de los 7 recursos gráficos reales (.png RGBA 256x256) de Sombras de Ojos con canal Alpha
        let eyeshadow_sprites_rgba = vec![
            load_rgba_sprite("eyeshadow", "natural_nude", include_bytes!("../assets/sprites/eyeshadow/natural_nude.png"), 256, 256),
            load_rgba_sprite("eyeshadow", "smoky_glam", include_bytes!("../assets/sprites/eyeshadow/smoky_glam.png"), 256, 256),
            load_rgba_sprite("eyeshadow", "cat_wing", include_bytes!("../assets/sprites/eyeshadow/cat_wing.png"), 256, 256),
            load_rgba_sprite("eyeshadow", "sunset_glow", include_bytes!("../assets/sprites/eyeshadow/sunset_glow.png"), 256, 256),
            load_rgba_sprite("eyeshadow", "grunge_kohl", include_bytes!("../assets/sprites/eyeshadow/grunge_kohl.png"), 256, 256),
            load_rgba_sprite("eyeshadow", "foxy_glam", include_bytes!("../assets/sprites/eyeshadow/foxy_glam.png"), 256, 256),
            load_rgba_sprite("eyeshadow", "glitter_pop", include_bytes!("../assets/sprites/eyeshadow/glitter_pop.png"), 256, 256),
        ];

        // Carga de los 4 recursos gráficos reales (.png RGBA 256x128) de Cejas Microblading / Ombré con canal Alpha
        let eyebrow_sprites_rgba = vec![
            load_rgba_sprite("eyebrows", "natural_fill", include_bytes!("../assets/sprites/eyebrows/natural_fill.png"), 256, 128),
            load_rgba_sprite("eyebrows", "defined_arch", include_bytes!("../assets/sprites/eyebrows/defined_arch.png"), 256, 128),
            load_rgba_sprite("eyebrows", "korean_straight", include_bytes!("../assets/sprites/eyebrows/korean_straight.png"), 256, 128),
            load_rgba_sprite("eyebrows", "fluffy_feathered", include_bytes!("../assets/sprites/eyebrows/fluffy_feathered.png"), 256, 128),
        ];

        // Carga de la Textura Maestra de Maquillaje RGBA 1024x1024 (Classic Oil / Canónica 3D)
        let master_makeup_rgba = load_rgba_sprite(
            "makeup",
            "classic_oil",
            include_bytes!("../assets/sprites/makeup/classic_oil.png"),
            1024,
            1024,
        );
        let face_mesh_renderer = FaceMeshRenderer::new(width, height);

        // Carga de Mallas 3D Low-Poly y Renderizador con Z-Buffer (Efectos / AR Props)
        let devil_horns_mesh = Mesh3D::from_obj_str(include_str!("../assets/models/devil_horns.obj"));
        let mesh3d_renderer = Mesh3DRenderer::new(width, height);

        // Generación procedimental de Sprites de soporte
        let contact_lens_sprite = generate_default_contact_lens_sprite();
        let lip_sprite = generate_default_lip_sprite();

        Self {
            width,
            height,
            output_buffer: vec![0; width * height * 4],
            luma_accumulator: 0.42,
            gaussian_lut,
            srgb_lut,
            lens_sprites_rgba,
            eyeshadow_sprites_rgba,
            eyebrow_sprites_rgba,
            face_mesh_renderer,
            master_makeup_rgba,
            devil_horns_mesh,
            mesh3d_renderer,
            smoothed_head_pose: None,
            contact_lens_sprite,
            smoothed_iris_discs: None,
            lip_sprite,
            smoothed_lip_curves: None,
            current_exp_gain: 1.0,
            current_wb_r: 1.0,
            current_wb_g: 1.0,
            current_wb_b: 1.0,
            current_contrast: 1.0,
            locked_auto_exp: 0.0,
            locked_wb_r: 1.0,
            locked_wb_g: 1.0,
            locked_wb_b: 1.0,
            locked_contrast: 1.0,
            lighting_hold_frames: 0,
            lighting_initialized: false,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.output_buffer.resize(width * height * 4, 0);
    }

    pub fn process_frame(
        &mut self,
        input_rgba: &[u8],
        mask: &[u8],
        skin_mask: &[u8],
        feature_mask: &[u8],
        faces: &[FaceLandmarks],
        face_lighting: Option<&FaceLighting>,
        config: &StudioConfig,
    ) -> (&[u8], PhotometricStats) {
        let width = self.width;
        let height = self.height;

        // 1. Análisis fotométrico de luminancia y crominancia ambiental & Prioridad Retrato Facial
        let active_face = faces.iter().find(|f| f.detected && f.bounding_box.width > 20.0);
        let photo_sample = analyze_frame_luma(input_rgba, width, height, faces);
        self.luma_accumulator = self.luma_accumulator * 0.90 + photo_sample.avg_luma * 0.10;
        let current_luma = self.luma_accumulator;

        // 2. Cálculo de Auto-Ajustes e IA de Calibración Base (SnapJM Zone VI Portrait Metering)
        let (calculated_exp, target_wb_r, target_wb_g, target_wb_b, effective_measured_luma) =
            compute_target_lighting(&photo_sample, current_luma, face_lighting);

        // Calibración visual neutra y cinematográfica estilo smartphone de gama alta
        let calculated_contrast: f32 = 1.00_f32;
        let calculated_sat: f32 = 1.06_f32;

        let calculated_sharpness = 0.35_f32;
        let calculated_wb_val = (target_wb_r - target_wb_b) * 4.0;

        let rec_exp = (calculated_exp * 20.0).round() / 20.0;
        let rec_contrast = (calculated_contrast * 20.0).round() / 20.0;
        let rec_sat = (calculated_sat * 20.0).round() / 20.0;
        let rec_wb = (calculated_wb_val * 20.0).round() / 20.0;
        let rec_sharpness = calculated_sharpness;

        // Histeresis y Ventana de Tolerancia Óptima:
        // Evita recalcular y oscilar la iluminación constantemente con micro-movimientos de cabeza o gestos
        if !self.lighting_initialized {
            self.locked_auto_exp = calculated_exp;
            self.locked_wb_r = target_wb_r;
            self.locked_wb_g = target_wb_g;
            self.locked_wb_b = target_wb_b;
            self.locked_contrast = calculated_contrast;
            self.lighting_initialized = true;
        } else {
            let exp_diff = (calculated_exp - self.locked_auto_exp).abs();
            let wb_diff = (target_wb_r - self.locked_wb_r).abs() + (target_wb_b - self.locked_wb_b).abs();
            let contrast_diff = (calculated_contrast - self.locked_contrast).abs();

            // Margen de tolerancia óptimo (Deadband):
            // Si la variación es menor al 5.5% de exposición o 4.5% de color, se mantiene 100% bloqueado
            let tolerance_exceeded = exp_diff > 0.055 || wb_diff > 0.045 || contrast_diff > 0.06;

            if tolerance_exceeded {
                self.lighting_hold_frames += 1;
                // Ventana de tiempo de confirmación: requiere que el cambio persista al menos 20 frames (~330ms)
                // para confirmar que es un cambio real de iluminación de la habitación y no un movimiento corporal
                if self.lighting_hold_frames >= 20 {
                    self.locked_auto_exp = calculated_exp;
                    self.locked_wb_r = target_wb_r;
                    self.locked_wb_g = target_wb_g;
                    self.locked_wb_b = target_wb_b;
                    self.locked_contrast = calculated_contrast;
                    self.lighting_hold_frames = 0;
                }
            } else {
                self.lighting_hold_frames = 0;
            }
        }

        // Modo Bypass Puro 1:1:
        // Si todo el procesamiento de IA está apagado y los controles manuales están en estado neutro,
        // se entrega el fotograma 100% idéntico al sensor crudo (Camera Hub) sin tocar ningún píxel.
        let is_bypass = !config.ai_auto_tune_enabled
            && !config.ai_lighting_enabled
            && !config.ai_denoise_enabled
            && !config.beauty_enabled
            && config.exposure.abs() < 0.001
            && config.brightness.abs() < 0.001
            && (config.contrast - 1.0).abs() < 0.001
            && (config.saturation - 1.0).abs() < 0.001
            && config.white_balance.abs() < 0.001
            && config.hue.abs() < 0.001
            && config.sharpness < 0.01
            && config.bg_mode == BackgroundMode::None
            && config.color_space == ColorSpace::Rec709
            && config.color_range == ColorRange::Full;

        if is_bypass {
            if self.output_buffer.len() != input_rgba.len() {
                self.output_buffer.resize(input_rgba.len(), 0);
            }
            self.output_buffer.copy_from_slice(input_rgba);
            return (
                &self.output_buffer,
                PhotometricStats {
                    detected_luma: effective_measured_luma,
                    auto_exposure: 0.0,
                    auto_wb: 0.0,
                    auto_contrast: 1.0,
                    recommended_exposure: rec_exp,
                    recommended_contrast: rec_contrast,
                    recommended_saturation: rec_sat,
                    recommended_sharpness: rec_sharpness,
                    recommended_wb: rec_wb,
                },
            );
        }

        // Amortiguación Temporal Exponencial Ultra-Estable (Zero-Flicker Cinematic AE/AWB)
        // Con autoridad directa e inmediata sobre controles manuales
        let is_ai_calibrate_active = config.ai_auto_tune_enabled;
        let target_auto_exp = if is_ai_calibrate_active { self.locked_auto_exp } else { 0.0 };
        let auto_gain = 2.0_f32.powf(target_auto_exp);
        self.current_exp_gain += 0.025 * (auto_gain - self.current_exp_gain);
        let exp_factor = self.current_exp_gain * 2.0_f32.powf(config.exposure.clamp(-2.0, 2.0));

        let target_auto_wb_r = if is_ai_calibrate_active { self.locked_wb_r } else { 1.0 };
        let target_auto_wb_g = if is_ai_calibrate_active { self.locked_wb_g } else { 1.0 };
        let target_auto_wb_b = if is_ai_calibrate_active { self.locked_wb_b } else { 1.0 };
        self.current_wb_r += 0.025 * (target_auto_wb_r - self.current_wb_r);
        self.current_wb_g += 0.025 * (target_auto_wb_g - self.current_wb_g);
        self.current_wb_b += 0.025 * (target_auto_wb_b - self.current_wb_b);
        let wb_r_gain = self.current_wb_r * (1.0 + config.white_balance.clamp(-1.0, 1.0) * 0.25);
        let wb_g_gain = self.current_wb_g;
        let wb_b_gain = self.current_wb_b * (1.0 - config.white_balance.clamp(-1.0, 1.0) * 0.25);

        let target_contrast = if is_ai_calibrate_active { self.locked_contrast } else { 1.0 };
        self.current_contrast += 0.03 * (target_contrast - self.current_contrast);
        let effective_contrast = (self.current_contrast * config.contrast.clamp(0.2, 2.5)).max(0.0);

        let saturation = if config.ai_auto_tune_enabled {
            (config.saturation.clamp(0.0, 2.5) * calculated_sat).max(0.0)
        } else {
            config.saturation.clamp(0.0, 2.5)
        };

        let hue_shift = config.hue.clamp(-1.0, 1.0) * 0.5;
        let sharpness = if config.ai_auto_tune_enabled && config.sharpness < 0.15 {
            calculated_sharpness
        } else {
            config.sharpness.clamp(0.0, 1.0)
        };
        let brightness = config.brightness.clamp(-0.8, 0.8);
        let bg_mode = config.bg_mode;
        let seg_threshold = config.seg_threshold;
        let edge_feather = config.edge_feather * 0.05;
        let color_range = config.color_range;
        let color_space = config.color_space;
        let ai_lighting = config.ai_lighting_enabled;
        let ai_denoise = config.ai_denoise_enabled;
        let beauty_active = config.beauty_enabled && config.skin_smoothing > 0.01;
        let skin_intensity = config.skin_smoothing.clamp(0.0, 1.0);
        let skin_whitening = if config.beauty_enabled { config.skin_whitening.clamp(0.0, 1.0) } else { 0.0 };
        let beauty_enabled = config.beauty_enabled;

        let teeth_whitening = if beauty_enabled { config.teeth_whitening.clamp(0.0, 1.0) } else { 0.0 };
        let lipstick = if beauty_enabled { config.lipstick.clamp(0.0, 1.0) } else { 0.0 };
        let foundation_tone = if beauty_enabled { config.foundation_tone.as_str() } else { "none" };
        let foundation_intensity = if beauty_enabled { config.foundation_intensity.clamp(0.0, 1.0) } else { 0.0 };
        let contour = if beauty_enabled { config.contour.clamp(0.0, 1.0) } else { 0.0 };
        let eyeshadow = if beauty_enabled { config.eyeshadow.clamp(0.0, 1.0) } else { 0.0 };
        let eyeshadow_color = if beauty_enabled { config.eyeshadow_color.as_str() } else { "none" };
        let eyeshadow_style = if beauty_enabled { config.eyeshadow_style.as_str() } else { "natural_nude" };
        let eyeshadow_target_rgb = get_eyeshadow_color_rgb(eyeshadow_color);
        let selected_eyeshadow_style_idx = get_eyeshadow_style_index(eyeshadow_style);
        let selected_eyeshadow_rgba = if eyeshadow > 0.001 && eyeshadow_target_rgb.is_some() {
            self.eyeshadow_sprites_rgba.get(selected_eyeshadow_style_idx)
        } else {
            None
        };

        let eyebrows = if beauty_enabled { config.eyebrows.clamp(0.0, 1.0) } else { 0.0 };
        let eyebrow_color = if beauty_enabled { config.eyebrow_color.as_str() } else { "dark_brown" };
        let _eyebrow_style = if beauty_enabled { config.eyebrow_style.as_str() } else { "natural_fill" };
        let eyebrow_target_rgb = get_eyebrow_color_rgb(eyebrow_color);

        let beard_enhancement = if beauty_enabled { config.beard_enhancement.clamp(0.0, 1.0) } else { 0.0 };
        let beard_color = if beauty_enabled { config.beard_color.as_str() } else { "natural" };
        let beard_target_rgb = get_beard_color_rgb(beard_color);

        let face_slim = if beauty_enabled { config.face_slim.clamp(0.0, 1.0) } else { 0.0 };
        let eye_enlarge = if beauty_enabled { config.eye_enlarge.clamp(0.0, 1.0) } else { 0.0 };
        let eye_round = if beauty_enabled { config.eye_round.clamp(0.0, 1.0) } else { 0.0 };
        let eye_shape_str = if beauty_enabled { config.eye_shape.as_str() } else { "auto" };
        let detected_shape = if let Some(face) = active_face {
            if face.landmarks.len() >= 468 {
                detect_eye_shape(&face.landmarks)
            } else {
                "almond"
            }
        } else {
            "almond"
        };
        let effective_eye_shape = if eye_shape_str == "auto" {
            detected_shape
        } else {
            eye_shape_str
        };
        let eye_shape_id = eye_shape_to_id(effective_eye_shape);
        let has_forced_eye_shape = beauty_enabled && eye_shape_str != "auto" && eye_shape_str != "almond";

        let eye_brighten = if beauty_enabled { config.eye_brighten.clamp(0.0, 1.0) } else { 0.0 };
        let eye_catchlight = if beauty_enabled { config.eye_catchlight.clamp(0.0, 1.0) } else { 0.0 };
        let eyelashes = if beauty_enabled { config.eyelashes.clamp(0.0, 1.0) } else { 0.0 };
        let iris_lighting = if beauty_enabled { config.iris_lighting.clamp(0.0, 1.0) } else { 0.0 };
        let iris_color = if beauty_enabled { config.iris_color.as_str() } else { "none" };
        let brow_lift = if beauty_enabled { config.brow_lift.clamp(0.0, 1.0) } else { 0.0 };
        let nose_slim = if beauty_enabled { config.nose_slim.clamp(-1.0, 1.0) } else { 0.0 };
        let lip_plump = if beauty_enabled { config.lip_plump.clamp(0.0, 1.0) } else { 0.0 };

        let morph_active = face_slim > 0.001
            || eye_enlarge > 0.001
            || eye_round > 0.001
            || has_forced_eye_shape
            || brow_lift > 0.001
            || nose_slim.abs() > 0.001
            || lip_plump > 0.001;

        let morph_data = if morph_active {
            if let Some(face) = active_face {
                if face.landmarks.len() >= 460 {
                    let lm = &face.landmarks;
                    let el_cx = (lm[160].x + lm[159].x * 2.0 + lm[158].x + lm[144].x + lm[145].x * 2.0 + lm[153].x) * 0.125;
                    let el_cy = (lm[160].y + lm[159].y * 2.0 + lm[158].y + lm[144].y + lm[145].y * 2.0 + lm[153].y) * 0.125;
                    let er_cx = (lm[385].x + lm[386].x * 2.0 + lm[387].x + lm[380].x + lm[374].x * 2.0 + lm[373].x) * 0.125;
                    let er_cy = (lm[385].y + lm[386].y * 2.0 + lm[387].y + lm[380].y + lm[374].y * 2.0 + lm[373].y) * 0.125;

                    let dx = er_cx - el_cx;
                    let dy = er_cy - el_cy;
                    let eye_dist = (dx * dx + dy * dy).sqrt().max(25.0);

                    // Parámetros anatómicos milimétricos del ojo izquierdo (viewer's left)
                    let el_rx = (lm[33].x - lm[133].x).abs() * 0.55;
                    let el_htop = (el_cy - lm[159].y).max(4.0);
                    let el_hbot = (lm[145].y - el_cy).max(4.0);
                    let el_hbrow = (el_cy - lm[105].y).max(el_htop * 1.6);
                    let eye_l_params = EyeMorphParams {
                        cx: el_cx,
                        cy: el_cy,
                        rx: el_rx.max(eye_dist * 0.20),
                        h_top: el_htop,
                        h_bot: el_hbot,
                        limit_y_top: (el_hbrow * 0.82).min(el_htop * 2.5).max(el_htop * 1.3),
                        limit_y_bot: (el_hbot * 2.2).max(el_hbot * 1.4),
                        is_viewer_left: true,
                        shape_id: eye_shape_id,
                    };

                    // Parámetros anatómicos milimétricos del ojo derecho (viewer's right)
                    let er_rx = (lm[263].x - lm[362].x).abs() * 0.55;
                    let er_htop = (er_cy - lm[386].y).max(4.0);
                    let er_hbot = (lm[374].y - er_cy).max(4.0);
                    let er_hbrow = (er_cy - lm[334].y).max(er_htop * 1.6);
                    let eye_r_params = EyeMorphParams {
                        cx: er_cx,
                        cy: er_cy,
                        rx: er_rx.max(eye_dist * 0.20),
                        h_top: er_htop,
                        h_bot: er_hbot,
                        limit_y_top: (er_hbrow * 0.82).min(er_htop * 2.5).max(er_htop * 1.3),
                        limit_y_bot: (er_hbot * 2.2).max(er_hbot * 1.4),
                        is_viewer_left: false,
                        shape_id: eye_shape_id,
                    };

                    let nose_tip = lm[1];
                    let nose_bridge = lm[168];
                    let mouth_l = lm[61];
                    let mouth_r = lm[291];
                    let mouth_top = lm[13];
                    let mouth_bot = lm[14];
                    let mouth_cx = (mouth_l.x + mouth_r.x) * 0.5;
                    let mouth_cy = (mouth_top.y + mouth_bot.y) * 0.5;
                    let brow_l = lm[105];
                    let brow_r = lm[334];
                    // Mandibular gonion / jaw angle under ears (172/397) and chin (152) - NOT cheeks (234/454)
                    let jaw_l = lm[172];
                    let jaw_r = lm[397];
                    let chin = lm[152];

                    let bb = &face.bounding_box;
                    let pad = eye_dist * 0.6;
                    let min_x = (bb.x - pad).max(0.0);
                    let max_x = (bb.x + bb.width + pad).min(width as f32);
                    let min_y = (bb.y - pad).max(0.0);
                    let max_y = (bb.y + bb.height + pad).min(height as f32);

                    Some((
                        eye_l_params, eye_r_params, nose_tip, nose_bridge,
                        brow_l, brow_r, jaw_l, jaw_r, chin,
                        mouth_cx, mouth_cy, eye_dist,
                        min_x, max_x, min_y, max_y,
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };


        let makeup_data = if contour > 0.001 {
            if let Some(face) = active_face {
                if face.landmarks.len() >= 460 {
                    let lm = &face.landmarks;
                    let jaw_l = lm[172];
                    let jaw_r = lm[397];
                    let mouth_l = lm[61];
                    let mouth_r = lm[291];
                    let dx = lm[386].x - lm[159].x;
                    let dy = lm[386].y - lm[159].y;
                    let eye_dist = (dx * dx + dy * dy).sqrt().max(25.0);

                    let chk_l = (jaw_l.x * 0.60 + mouth_l.x * 0.40, jaw_l.y * 0.60 + mouth_l.y * 0.40);
                    let chk_r = (jaw_r.x * 0.60 + mouth_r.x * 0.40, jaw_r.y * 0.60 + mouth_r.y * 0.40);

                    Some((chk_l, chk_r, eye_dist))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let face_geom = if let Some(face) = active_face {
            if face.landmarks.len() >= 460 {
                let lm = &face.landmarks;
                let eye_l = lm[159];
                let eye_r = lm[386];
                let cheek_l = lm[234];
                let cheek_r = lm[454];
                let jaw_l = lm[172];
                let jaw_r = lm[397];
                let dx = eye_r.x - eye_l.x;
                let dy = eye_r.y - eye_l.y;
                let eye_dist = (dx * dx + dy * dy).sqrt().max(25.0);
                let face_left = jaw_l.x.min(cheek_l.x);
                let face_right = jaw_r.x.max(cheek_r.x);
                Some((eye_l, eye_r, eye_dist, face_left, face_right))
            } else {
                None
            }
        } else {
            None
        };

        let foundation_rgb: Option<(f32, f32, f32)> = match foundation_tone {
            "01" | "encendido" | "fair" | "porcelain" | "tone-7" => Some((0.96, 0.88, 0.82)),
            "02" | "piel" | "natural" | "tone-6" => Some((0.93, 0.82, 0.74)),
            "03" | "tez" | "warm_beige" | "tone-5" => Some((0.85, 0.65, 0.47)),
            "04" | "bronceado" | "tan" | "tone-4" => Some((0.72, 0.48, 0.30)),
            "tone-1" => Some((0.51, 0.27, 0.14)),
            "tone-2" => Some((0.61, 0.35, 0.29)),
            "tone-3" => Some((0.69, 0.43, 0.28)),
            "deep_warm" => Some((0.55, 0.33, 0.20)),
            _ => None,
        };

        let selected_lens_idx: Option<usize> = match iris_color {
            "honey" | "ambar" | "miel" => Some(0),
            "hazel" | "avellana" => Some(1),
            "emerald" | "esmeralda" | "verde" => Some(2),
            "ice_blue" | "azul_hielo" | "cielo" => Some(3),
            "sapphire" | "zafiro" | "azul" => Some(4),
            "gray" | "gris" | "perla" => Some(5),
            "violet" | "violeta" | "lila" => Some(6),
            _ => None,
        };

        let lipstick_color = if beauty_enabled { config.lipstick_color.as_str() } else { "none" };
        let lipstick_target_rgb: Option<(f32, f32, f32)> = match lipstick_color {
            "none" => None,
            "coral_peach" => Some((0.91, 0.36, 0.30)),
            "soft_pink" => Some((0.90, 0.42, 0.60)),
            "nude_velvet" => Some((0.77, 0.44, 0.38)),
            "classic_red" => Some((0.82, 0.12, 0.22)),
            "berry_plum" => Some((0.62, 0.16, 0.36)),
            "wine_burgundy" => Some((0.48, 0.11, 0.22)),
            _ => Some((0.86, 0.28, 0.40)), // 'rose_natural'
        };

        // Precomputación geométrica de Ojos Estilo Bigo Live (Apertura Palpebral 1080p, Iris Circular, Pestañas y Catchlights)
        let eye_accents = if beauty_enabled
            && (eyelashes > 0.001
                || eye_catchlight > 0.001
                || eye_brighten > 0.001
                || iris_lighting > 0.001
                || selected_lens_idx.is_some()
                || (eyeshadow > 0.001 && selected_eyeshadow_rgba.is_some()))
        {
            if let Some(face) = active_face {
                if face.landmarks.len() >= 468 {
                    let lm = &face.landmarks;

                    // 1. Ojo Izquierdo (Viewer's left / Subject's right)
                    // Ordenado de izquierda (33 = canto externo) a derecha (133 = lagrimal interno)
                    let upper_pts_l = [
                        (lm[33].x, lm[33].y),
                        (lm[246].x, lm[246].y),
                        (lm[161].x, lm[161].y),
                        (lm[160].x, lm[160].y),
                        (lm[159].x, lm[159].y),
                        (lm[158].x, lm[158].y),
                        (lm[157].x, lm[157].y),
                        (lm[173].x, lm[173].y),
                        (lm[133].x, lm[133].y),
                    ];
                    let lower_pts_l = [
                        (lm[33].x, lm[33].y),
                        (lm[7].x, lm[7].y),
                        (lm[163].x, lm[163].y),
                        (lm[144].x, lm[144].y),
                        (lm[145].x, lm[145].y),
                        (lm[153].x, lm[153].y),
                        (lm[154].x, lm[154].y),
                        (lm[155].x, lm[155].y),
                        (lm[133].x, lm[133].y),
                    ];

                    let el_inner = lm[133];
                    let el_outer = lm[33];
                    let el_top = lm[159];
                    let el_bot = lm[145];

                    let dx_l = el_outer.x - el_inner.x;
                    let dy_l = el_outer.y - el_inner.y;
                    let eye_w_l = (dx_l * dx_l + dy_l * dy_l).sqrt().max(10.0);
                    let eye_h_l = (el_bot.y - el_top.y).abs().max(4.0);

                    let (wing_ext, wing_lift) = match eye_shape_id {
                        1 => (0.24, 0.32),   // Foxy: Ala temporal alargada y elevada a ~40 grados
                        2 => (0.16, 0.12),   // Asiático: Elongación sutil a lo largo de la línea palpebral
                        3 => (0.08, 0.08),   // Doe / Muñeca: Redondeo suave sin ala excesiva
                        4 => (0.18, 0.22),   // Encapotado: Batwing liner elevado por encima del pliegue
                        _ => (0.14, 0.18),   // Almendrado / Clásico
                    };

                    let wing_lift_px_l = eye_w_l * (0.24 * wing_lift);
                    let wing_l = Point2f {
                        x: el_outer.x + (dx_l / eye_w_l) * (eye_w_l * wing_ext),
                        y: el_outer.y - wing_lift_px_l,
                    };

                    let segs_l = [
                        LineSegment2D::new(wing_l.x, wing_l.y, lm[33].x, lm[33].y),
                        LineSegment2D::new(lm[33].x, lm[33].y, lm[246].x, lm[246].y),
                        LineSegment2D::new(lm[246].x, lm[246].y, lm[161].x, lm[161].y),
                        LineSegment2D::new(lm[161].x, lm[161].y, lm[160].x, lm[160].y),
                        LineSegment2D::new(lm[160].x, lm[160].y, lm[159].x, lm[159].y),
                        LineSegment2D::new(lm[159].x, lm[159].y, lm[158].x, lm[158].y),
                        LineSegment2D::new(lm[158].x, lm[158].y, lm[157].x, lm[157].y),
                        LineSegment2D::new(lm[157].x, lm[157].y, lm[173].x, lm[173].y),
                        LineSegment2D::new(lm[173].x, lm[173].y, lm[133].x, lm[133].y),
                    ];
                    let lower_segs_l = [
                        LineSegment2D::new(lm[33].x, lm[33].y, lm[7].x, lm[7].y),
                        LineSegment2D::new(lm[7].x, lm[7].y, lm[163].x, lm[163].y),
                        LineSegment2D::new(lm[163].x, lm[163].y, lm[144].x, lm[144].y),
                        LineSegment2D::new(lm[144].x, lm[144].y, lm[145].x, lm[145].y),
                        LineSegment2D::new(lm[145].x, lm[145].y, lm[153].x, lm[153].y),
                        LineSegment2D::new(lm[153].x, lm[153].y, lm[154].x, lm[154].y),
                        LineSegment2D::new(lm[154].x, lm[154].y, lm[155].x, lm[155].y),
                        LineSegment2D::new(lm[155].x, lm[155].y, lm[133].x, lm[133].y),
                    ];

                    let raw_disc_l = locate_iris_disc(&upper_pts_l, &lower_pts_l, input_rgba, width, height);

                    // 2. Ojo Derecho (Viewer's right / Subject's left)
                    // Ordenado de izquierda (362 = lagrimal interno) a derecha (263 = canto externo)
                    let upper_pts_r = [
                        (lm[362].x, lm[362].y),
                        (lm[398].x, lm[398].y),
                        (lm[384].x, lm[384].y),
                        (lm[385].x, lm[385].y),
                        (lm[386].x, lm[386].y),
                        (lm[387].x, lm[387].y),
                        (lm[388].x, lm[388].y),
                        (lm[466].x, lm[466].y),
                        (lm[263].x, lm[263].y),
                    ];
                    let lower_pts_r = [
                        (lm[362].x, lm[362].y),
                        (lm[382].x, lm[382].y),
                        (lm[381].x, lm[381].y),
                        (lm[380].x, lm[380].y),
                        (lm[374].x, lm[374].y),
                        (lm[373].x, lm[373].y),
                        (lm[390].x, lm[390].y),
                        (lm[249].x, lm[249].y),
                        (lm[263].x, lm[263].y),
                    ];

                    let er_inner = lm[362];
                    let er_outer = lm[263];
                    let er_top = lm[386];
                    let er_bot = lm[374];

                    let dx_r = er_outer.x - er_inner.x;
                    let dy_r = er_outer.y - er_inner.y;
                    let eye_w_r = (dx_r * dx_r + dy_r * dy_r).sqrt().max(10.0);
                    let eye_h_r = (er_bot.y - er_top.y).abs().max(4.0);

                    // Relación biométrica inter-ocular (Charlotte Tilbury Close-Set vs Wide-Set)
                    let d_inter_canthal = (lm[362].x - lm[133].x).abs();
                    let avg_eye_w = ((eye_w_l + eye_w_r) * 0.5).max(1.0);
                    let inter_canthal_ratio = d_inter_canthal / avg_eye_w;

                    let wing_lift_px_r = eye_w_r * (0.24 * wing_lift);
                    let wing_r = Point2f {
                        x: er_outer.x + (dx_r / eye_w_r) * (eye_w_r * wing_ext),
                        y: er_outer.y - wing_lift_px_r,
                    };

                    let segs_r = [
                        LineSegment2D::new(lm[362].x, lm[362].y, lm[398].x, lm[398].y),
                        LineSegment2D::new(lm[398].x, lm[398].y, lm[384].x, lm[384].y),
                        LineSegment2D::new(lm[384].x, lm[384].y, lm[385].x, lm[385].y),
                        LineSegment2D::new(lm[385].x, lm[385].y, lm[386].x, lm[386].y),
                        LineSegment2D::new(lm[386].x, lm[386].y, lm[387].x, lm[387].y),
                        LineSegment2D::new(lm[387].x, lm[387].y, lm[388].x, lm[388].y),
                        LineSegment2D::new(lm[388].x, lm[388].y, lm[466].x, lm[466].y),
                        LineSegment2D::new(lm[466].x, lm[466].y, lm[263].x, lm[263].y),
                        LineSegment2D::new(lm[263].x, lm[263].y, wing_r.x, wing_r.y),
                    ];
                    let lower_segs_r = [
                        LineSegment2D::new(lm[362].x, lm[362].y, lm[382].x, lm[382].y),
                        LineSegment2D::new(lm[382].x, lm[382].y, lm[381].x, lm[381].y),
                        LineSegment2D::new(lm[381].x, lm[381].y, lm[380].x, lm[380].y),
                        LineSegment2D::new(lm[380].x, lm[380].y, lm[374].x, lm[374].y),
                        LineSegment2D::new(lm[374].x, lm[374].y, lm[373].x, lm[373].y),
                        LineSegment2D::new(lm[373].x, lm[373].y, lm[390].x, lm[390].y),
                        LineSegment2D::new(lm[390].x, lm[390].y, lm[249].x, lm[249].y),
                        LineSegment2D::new(lm[249].x, lm[249].y, lm[263].x, lm[263].y),
                    ];

                    let raw_disc_r = locate_iris_disc(&upper_pts_r, &lower_pts_r, input_rgba, width, height);

                    // Estabilización temporal 1€ por velocidad para los discos de iris (cero temblor en reposo, cero lag al mirar)
                    let [disc_l, disc_r] = if let Some(prev) = self.smoothed_iris_discs {
                        let raw_pair = [raw_disc_l, raw_disc_r];
                        let mut next = prev;
                        for e in 0..2 {
                            let (rx, ry, rrad, r_ilum, r_slum) = raw_pair[e];
                            let (px_prev, py_prev, prad_prev, pilum, pslum) = prev[e];
                            let dist = ((rx - px_prev).powi(2) + (ry - py_prev).powi(2)).sqrt();
                            if dist > 20.0 {
                                next[e] = raw_pair[e];
                            } else {
                                let vel_t = ((dist - 0.35) / 2.2).clamp(0.0, 1.0);
                                let a_pos = 0.28 + 0.68 * vel_t * vel_t;
                                let a_rad = 0.22 + 0.55 * vel_t;
                                next[e] = (
                                    px_prev + a_pos * (rx - px_prev),
                                    py_prev + a_pos * (ry - py_prev),
                                    prad_prev + a_rad * (rrad - prad_prev),
                                    pilum + 0.25 * (r_ilum - pilum),
                                    pslum + 0.25 * (r_slum - pslum),
                                );
                            }
                        }
                        self.smoothed_iris_discs = Some(next);
                        next
                    } else {
                        let init = [raw_disc_l, raw_disc_r];
                        self.smoothed_iris_discs = Some(init);
                        init
                    };

                    let (pupil_l_x, pupil_l_y, iris_rad_l, iris_lum_l, sclera_lum_l) = disc_l;
                    let pupil_rad_l = (iris_rad_l * 0.62).clamp(2.8, 14.0);
                    let r_spec1_l = (pupil_rad_l * 0.42).clamp(1.6, 4.2);
                    let r_spec2_l = r_spec1_l * 0.48;

                    let left_x_l = upper_pts_l[0].0.min(lower_pts_l[0].0);
                    let right_x_l = upper_pts_l[8].0.max(lower_pts_l[8].0);

                    let r0_l = [
                        (lm[33].x, lm[33].y),
                        (lm[246].x, lm[246].y),
                        (lm[161].x, lm[161].y),
                        (lm[160].x, lm[160].y),
                        (lm[159].x, lm[159].y),
                        (lm[158].x, lm[158].y),
                        (lm[157].x, lm[157].y),
                        (lm[173].x, lm[173].y),
                        (lm[133].x, lm[133].y),
                    ];
                    let r1_l = [
                        (lm[7].x, lm[7].y),
                        (lm[247].x, lm[247].y),
                        (lm[30].x, lm[30].y),
                        (lm[29].x, lm[29].y),
                        (lm[27].x, lm[27].y),
                        (lm[28].x, lm[28].y),
                        (lm[56].x, lm[56].y),
                        (lm[190].x, lm[190].y),
                        (lm[243].x, lm[243].y),
                    ];
                    let r2_l = [
                        (lm[130].x, lm[130].y),
                        (lm[25].x, lm[25].y),
                        (lm[110].x, lm[110].y),
                        (lm[24].x, lm[24].y),
                        (lm[23].x, lm[23].y),
                        (lm[22].x, lm[22].y),
                        (lm[26].x, lm[26].y),
                        (lm[112].x, lm[112].y),
                        (lm[244].x, lm[244].y),
                    ];
                    let (mesh_triangles_l, num_triangles_l) = build_eye_mesh_triangles(&r0_l, &r1_l, &r2_l, (wing_l.x, wing_l.y));

                    let min_y_l = if eyeshadow > 0.001 {
                        (lm[23].y.min(lm[105].y) - 6.0).max(0.0)
                    } else {
                        (lm[159].y.min(pupil_l_y - iris_rad_l) - eye_h_l * 0.50 - 7.0).max(0.0)
                    };

                    let eye_l_params = EyeAccentParams {
                        min_x: (wing_l.x.min(left_x_l) - 6.0).max(0.0),
                        max_x: (right_x_l + 6.0).min(width as f32),
                        min_y: min_y_l,
                        max_y: (lm[145].y.max(pupil_l_y + iris_rad_l) + eye_w_l * 0.40 + 12.0).min(height as f32),
                        left_x: left_x_l,
                        right_x: right_x_l,
                        upper_pts: upper_pts_l,
                        lower_pts: lower_pts_l,
                        es_upper: r0_l,
                        es_crease: r1_l,
                        mesh_triangles: mesh_triangles_l,
                        num_triangles: num_triangles_l,
                        pupil_x: pupil_l_x,
                        pupil_y: pupil_l_y,
                        iris_rad: iris_rad_l,
                        inv_iris_rad: 1.0 / iris_rad_l,
                        iris_lum: iris_lum_l,
                        sclera_lum: sclera_lum_l,
                        hl1_x: pupil_l_x - pupil_rad_l * 0.26,
                        hl1_y: pupil_l_y - pupil_rad_l * 0.28,
                        r_spec1_sq: r_spec1_l * r_spec1_l,
                        inv_r_spec1: 1.0 / r_spec1_l,
                        hl2_x: pupil_l_x + pupil_rad_l * 0.22,
                        hl2_y: pupil_l_y + pupil_rad_l * 0.22,
                        r_spec2_sq: r_spec2_l * r_spec2_l,
                        inv_r_spec2: 1.0 / r_spec2_l,
                        segments: segs_l,
                        lower_segments: lower_segs_l,
                        is_viewer_left: true,
                        inner_x: lm[133].x,
                        outer_x: wing_l.x.min(lm[33].x),
                        inv_eye_width: 1.0 / eye_w_l,
                        eye_w: eye_w_l,
                        eye_h: eye_h_l,
                        shape_id: eye_shape_id,
                        inter_canthal_ratio,
                    };

                    let (pupil_r_x, pupil_r_y, iris_rad_r, iris_lum_r, sclera_lum_r) = disc_r;
                    let pupil_rad_r = (iris_rad_r * 0.62).clamp(2.8, 14.0);
                    let r_spec1_r = (pupil_rad_r * 0.42).clamp(1.6, 4.2);
                    let r_spec2_r = r_spec1_r * 0.48;

                    let left_x_r = upper_pts_r[0].0.min(lower_pts_r[0].0);
                    let right_x_r = upper_pts_r[8].0.max(lower_pts_r[8].0);

                    let r0_r = [
                        (lm[263].x, lm[263].y),
                        (lm[466].x, lm[466].y),
                        (lm[388].x, lm[388].y),
                        (lm[387].x, lm[387].y),
                        (lm[386].x, lm[386].y),
                        (lm[385].x, lm[385].y),
                        (lm[384].x, lm[384].y),
                        (lm[398].x, lm[398].y),
                        (lm[362].x, lm[362].y),
                    ];
                    let r1_r = [
                        (lm[359].x, lm[359].y),
                        (lm[467].x, lm[467].y),
                        (lm[260].x, lm[260].y),
                        (lm[259].x, lm[259].y),
                        (lm[257].x, lm[257].y),
                        (lm[258].x, lm[258].y),
                        (lm[286].x, lm[286].y),
                        (lm[414].x, lm[414].y),
                        (lm[463].x, lm[463].y),
                    ];
                    let r2_r = [
                        (lm[255].x, lm[255].y),
                        (lm[339].x, lm[339].y),
                        (lm[254].x, lm[254].y),
                        (lm[253].x, lm[253].y),
                        (lm[252].x, lm[252].y),
                        (lm[256].x, lm[256].y),
                        (lm[341].x, lm[341].y),
                        (lm[464].x, lm[464].y),
                        (lm[465].x, lm[465].y),
                    ];
                    let (mesh_triangles_r, num_triangles_r) = build_eye_mesh_triangles(&r0_r, &r1_r, &r2_r, (wing_r.x, wing_r.y));

                    let min_y_r = if eyeshadow > 0.001 {
                        (lm[252].y.min(lm[334].y) - 6.0).max(0.0)
                    } else {
                        (lm[386].y.min(pupil_r_y - iris_rad_r) - eye_h_r * 0.50 - 7.0).max(0.0)
                    };

                    let eye_r_params = EyeAccentParams {
                        min_x: (left_x_r - 6.0).max(0.0),
                        max_x: (wing_r.x.max(right_x_r) + 6.0).min(width as f32),
                        min_y: min_y_r,
                        max_y: (lm[374].y.max(pupil_r_y + iris_rad_r) + eye_w_r * 0.40 + 12.0).min(height as f32),
                        left_x: left_x_r,
                        right_x: right_x_r,
                        upper_pts: upper_pts_r,
                        lower_pts: lower_pts_r,
                        es_upper: r0_r,
                        es_crease: r1_r,
                        mesh_triangles: mesh_triangles_r,
                        num_triangles: num_triangles_r,
                        pupil_x: pupil_r_x,
                        pupil_y: pupil_r_y,
                        iris_rad: iris_rad_r,
                        inv_iris_rad: 1.0 / iris_rad_r,
                        iris_lum: iris_lum_r,
                        sclera_lum: sclera_lum_r,
                        hl1_x: pupil_r_x - pupil_rad_r * 0.26,
                        hl1_y: pupil_r_y - pupil_rad_r * 0.28,
                        r_spec1_sq: r_spec1_r * r_spec1_r,
                        inv_r_spec1: 1.0 / r_spec1_r,
                        hl2_x: pupil_r_x + pupil_rad_r * 0.22,
                        hl2_y: pupil_r_y + pupil_rad_r * 0.22,
                        r_spec2_sq: r_spec2_r * r_spec2_r,
                        inv_r_spec2: 1.0 / r_spec2_r,
                        segments: segs_r,
                        lower_segments: lower_segs_r,
                        is_viewer_left: false,
                        inner_x: lm[362].x,
                        outer_x: wing_r.x.max(lm[263].x),
                        inv_eye_width: 1.0 / eye_w_r,
                        eye_w: eye_w_r,
                        eye_h: eye_h_r,
                        shape_id: eye_shape_id,
                        inter_canthal_ratio,
                    };

                    let sampled_face_luma = if photo_sample.face_sample_count > 0 {
                        photo_sample.face_luma_sum / photo_sample.face_sample_count as f32
                    } else {
                        0.45
                    };
                    let face_luma = face_lighting
                        .filter(|fl| fl.has_face)
                        .map(|fl| fl.face_luma)
                        .unwrap_or(sampled_face_luma);
                    let backlight_factor = (face_luma / 0.40).clamp(0.28, 1.0);

                    Some(([eye_l_params, eye_r_params], backlight_factor))
                } else {
                    self.smoothed_iris_discs = None;
                    None
                }
            } else {
                self.smoothed_iris_discs = None;
                None
            }
        } else {
            self.smoothed_iris_discs = None;
            None
        };



        // Pre-cálculo por frame de las 4 curvas exactas 1080p para el Sprite Deformable de Labios
        // con estabilización temporal adaptativa 1€ (elimina micro-vibración en reposo sin lag al hablar)
        let lip_accents: Option<LipSpriteParams> = if beauty_enabled && (lipstick > 0.001 || teeth_whitening > 0.001) {
            if let Some(face) = active_face {
                if face.landmarks.len() >= 468 {
                    let lm = &face.landmarks;
                    let raw_curves = [
                        [
                            (lm[61].x, lm[61].y),
                            (lm[185].x, lm[185].y),
                            (lm[40].x, lm[40].y),
                            (lm[39].x, lm[39].y),
                            (lm[37].x, lm[37].y),
                            (lm[0].x, lm[0].y),
                            (lm[267].x, lm[267].y),
                            (lm[269].x, lm[269].y),
                            (lm[270].x, lm[270].y),
                            (lm[409].x, lm[409].y),
                            (lm[291].x, lm[291].y),
                        ],
                        [
                            (lm[61].x, lm[61].y),
                            (lm[191].x, lm[191].y),
                            (lm[80].x, lm[80].y),
                            (lm[81].x, lm[81].y),
                            (lm[82].x, lm[82].y),
                            (lm[13].x, lm[13].y),
                            (lm[312].x, lm[312].y),
                            (lm[311].x, lm[311].y),
                            (lm[310].x, lm[310].y),
                            (lm[415].x, lm[415].y),
                            (lm[291].x, lm[291].y),
                        ],
                        [
                            (lm[61].x, lm[61].y),
                            (lm[95].x, lm[95].y),
                            (lm[88].x, lm[88].y),
                            (lm[178].x, lm[178].y),
                            (lm[87].x, lm[87].y),
                            (lm[14].x, lm[14].y),
                            (lm[317].x, lm[317].y),
                            (lm[402].x, lm[402].y),
                            (lm[318].x, lm[318].y),
                            (lm[324].x, lm[324].y),
                            (lm[291].x, lm[291].y),
                        ],
                        [
                            (lm[61].x, lm[61].y),
                            (lm[146].x, lm[146].y),
                            (lm[91].x, lm[91].y),
                            (lm[181].x, lm[181].y),
                            (lm[84].x, lm[84].y),
                            (lm[17].x, lm[17].y),
                            (lm[314].x, lm[314].y),
                            (lm[405].x, lm[405].y),
                            (lm[321].x, lm[321].y),
                            (lm[375].x, lm[375].y),
                            (lm[291].x, lm[291].y),
                        ],
                    ];

                    let smoothed = if let Some(mut prev) = self.smoothed_lip_curves {
                        for c in 0..4 {
                            for i in 0..11 {
                                let (nx, ny) = raw_curves[c][i];
                                let (px, py) = prev[c][i];
                                let dist = ((nx - px).powi(2) + (ny - py).powi(2)).sqrt();
                                // Filtro 1€ por velocidad: en reposo (dist < 1.2px) amortigua el 88% del ruido sub-píxel;
                                // al hablar o moverse (dist > 1.2px) acelera suavemente hasta seguimiento instantáneo.
                                let alpha_t = if dist > 24.0 {
                                    1.0
                                } else {
                                    let vel_t = ((dist - 0.35) / 4.2).clamp(0.0, 1.0);
                                    0.12 + 0.78 * (vel_t * vel_t)
                                };
                                prev[c][i] = (
                                    px + (nx - px) * alpha_t,
                                    py + (ny - py) * alpha_t,
                                );
                            }
                        }
                        prev
                    } else {
                        raw_curves
                    };
                    self.smoothed_lip_curves = Some(smoothed);

                    let [outer_top, inner_top, inner_bot, outer_bot] = smoothed;
                    let left_x = outer_top[0].0;
                    let right_x = outer_top[10].0.max(left_x + 8.0);
                    let lip_w = (right_x - left_x).max(8.0);
                    let min_y = outer_top[4].1.min(outer_top[6].1).min(outer_top[5].1) - 4.0;
                    let max_y = outer_bot[5].1.max(outer_bot[4].1).max(outer_bot[6].1) + 4.0;

                    Some(LipSpriteParams {
                        min_x: (left_x - 3.0).max(0.0),
                        max_x: (right_x + 3.0).min(width as f32),
                        min_y: min_y.max(0.0),
                        max_y: max_y.min(height as f32),
                        left_x,
                        right_x,
                        inv_lip_width: 1.0 / lip_w,
                        outer_top,
                        inner_top,
                        inner_bot,
                        outer_bot,
                    })
                } else {
                    self.smoothed_lip_curves = None;
                    None
                }
            } else {
                self.smoothed_lip_curves = None;
                None
            }
        } else {
            self.smoothed_lip_curves = None;
            None
        };

        // Estimación de Orientación 3D de Cabeza (Head Pose 6DoF) para Props y Efectos 3D
        let head_pose_3d = if config.active_effect == "devil_horns" {
            if let Some(face) = active_face {
                let pose = estimate_head_pose_3d(
                    &face.landmarks,
                    &mut self.smoothed_head_pose,
                );
                static mut LAST_POSE_LOG: u32 = 0;
                unsafe {
                    LAST_POSE_LOG += 1;
                    if LAST_POSE_LOG % 60 == 1 {
                        if let Some(ref p) = pose {
                            crate::log_info!("Effects", "HeadPose3D OK: yaw={:.2}, center=({:.1}, {:.1}), scale={:.2}",
                                p.yaw_ratio, p.center_x, p.center_y, p.scale);
                        } else {
                            crate::log_warn!("Effects", "estimate_head_pose_3d es NONE! landmarks={}", face.landmarks.len());
                        }
                    }
                }
                pose
            } else {
                static mut LAST_NO_FACE: u32 = 0;
                unsafe {
                    LAST_NO_FACE += 1;
                    if LAST_NO_FACE % 60 == 1 {
                        crate::log_warn!("Effects", "devil_horns activo pero no hay active_face! faces.len()={}", faces.len());
                    }
                }
                self.smoothed_head_pose = None;
                None
            }
        } else {
            self.smoothed_head_pose = None;
            None
        };

        // Renderizado 3D de la Malla Canónica Facial (898 triángulos UV estilo Bigo Live)
        let face_mesh_bounds = if beauty_enabled {
            if let Some(face) = active_face {
                self.face_mesh_renderer.rasterize_face(&face.landmarks)
            } else {
                None
            }
        } else {
            None
        };
        let face_uv_map = &self.face_mesh_renderer.face_uv_map;
        let master_makeup = &self.master_makeup_rgba;

        let lut = &self.gaussian_lut;
        let lut_srgb = &self.srgb_lut;
        let lens_sprite = &self.contact_lens_sprite;
        let selected_lens_rgba: Option<&[[f32; 4]]> = selected_lens_idx.map(|i| self.lens_sprites_rgba[i].as_slice());
        let lip_sprite_lut = &self.lip_sprite;

        // 3. Procesamiento en paralelo con Rayon a 60 FPS estables
        self.output_buffer
            .par_chunks_mut(width * 4)
            .enumerate()
            .for_each(|(y, out_row)| {
                let in_row_start = y * width * 4;
                let in_row = &input_rgba[in_row_start..in_row_start + width * 4];
                let row_up = if y > 0 {
                    &input_rgba[(y - 1) * width * 4..y * width * 4]
                } else {
                    in_row
                };
                let row_down = if y + 1 < height {
                    &input_rgba[(y + 1) * width * 4..(y + 2) * width * 4]
                } else {
                    in_row
                };

                let has_feature_mask = feature_mask.len() == 320 * 240 * 4;
                let (r0_offset, r1_offset, feat_dy) = if has_feature_mask {
                    let fy = (y as f32 * 239.0) / (height - 1).max(1) as f32;
                    let y0 = (fy as usize).min(239);
                    let y1 = (y0 + 1).min(239);
                    let dy = fy - y0 as f32;
                    (y0 * 320 * 4, y1 * 320 * 4, dy)
                } else {
                    (0, 0, 0.0)
                };

                let has_compact_seg = mask.len() == 256 * 256;
                let seg_y_offset = if has_compact_seg {
                    let sy = (y * 255) / (height - 1).max(1);
                    sy * 256
                } else {
                    0
                };

                let mask_row_start = y * width;
                let mask_row = if !has_compact_seg && mask_row_start + width <= mask.len() {
                    &mask[mask_row_start..mask_row_start + width]
                } else {
                    &[]
                };
                let skin_row = if mask_row_start + width <= skin_mask.len() {
                    &skin_mask[mask_row_start..mask_row_start + width]
                } else {
                    &[]
                };
                let feat_row_start = y * width * 4;
                let feat_row = if !has_feature_mask && feat_row_start + width * 4 <= feature_mask.len() {
                    &feature_mask[feat_row_start..feat_row_start + width * 4]
                } else {
                    &[]
                };

                for x in 0..width {
                    let idx = x * 4;
                    let f_idx = x * 4;
                    let px = x as f32;
                    let py = y as f32;

                    let (mut skin_val, teeth_val, eyes_val, lips_val) = if has_feature_mask {
                        let fx = (px * 319.0) / (width - 1).max(1) as f32;
                        let x0 = (fx as usize).min(319);
                        let x1 = (x0 + 1).min(319);
                        let dx = fx - x0 as f32;

                        let o00 = r0_offset + x0 * 4;
                        let o10 = r0_offset + x1 * 4;
                        let o01 = r1_offset + x0 * 4;
                        let o11 = r1_offset + x1 * 4;

                        // Canal 0: Piel (Red) con Hermite s-curve para feathering orgánico
                        let v00_r = feature_mask[o00] as f32;
                        let v10_r = feature_mask[o10] as f32;
                        let v01_r = feature_mask[o01] as f32;
                        let v11_r = feature_mask[o11] as f32;
                        let top_r = v00_r + (v10_r - v00_r) * dx;
                        let bot_r = v01_r + (v11_r - v01_r) * dx;
                        let val_r = top_r + (bot_r - top_r) * feat_dy;
                        let t = (val_r * (1.0 / 255.0)).clamp(0.0, 1.0);
                        let s_skin = t * t * (3.0 - 2.0 * t);

                        // Canal 1: Dientes / Boca interior (Green)
                        let v00_g = feature_mask[o00 + 1] as f32;
                        let v10_g = feature_mask[o10 + 1] as f32;
                        let v01_g = feature_mask[o01 + 1] as f32;
                        let v11_g = feature_mask[o11 + 1] as f32;
                        let top_g = v00_g + (v10_g - v00_g) * dx;
                        let bot_g = v01_g + (v11_g - v01_g) * dx;
                        let s_teeth = ((top_g + (bot_g - top_g) * feat_dy) * (1.0 / 255.0)).clamp(0.0, 1.0);

                        // Canal 2: Ojos / Esclera (Blue)
                        let v00_b = feature_mask[o00 + 2] as f32;
                        let v10_b = feature_mask[o10 + 2] as f32;
                        let v01_b = feature_mask[o01 + 2] as f32;
                        let v11_b = feature_mask[o11 + 2] as f32;
                        let top_b = v00_b + (v10_b - v00_b) * dx;
                        let bot_b = v01_b + (v11_b - v01_b) * dx;
                        let s_eyes = ((top_b + (bot_b - top_b) * feat_dy) * (1.0 / 255.0)).clamp(0.0, 1.0);

                        // Canal 3: Labios exteriores (Alpha)
                        let v00_a = feature_mask[o00 + 3] as f32;
                        let v10_a = feature_mask[o10 + 3] as f32;
                        let v01_a = feature_mask[o01 + 3] as f32;
                        let v11_a = feature_mask[o11 + 3] as f32;
                        let top_a = v00_a + (v10_a - v00_a) * dx;
                        let bot_a = v01_a + (v11_a - v01_a) * dx;
                        let s_lips = ((top_a + (bot_a - top_a) * feat_dy) * (1.0 / 255.0)).clamp(0.0, 1.0);

                        (s_skin, s_teeth, s_eyes, s_lips)
                    } else if f_idx + 3 < feat_row.len() {
                        (
                            feat_row[f_idx] as f32 / 255.0,
                            feat_row[f_idx + 1] as f32 / 255.0,
                            feat_row[f_idx + 2] as f32 / 255.0,
                            feat_row[f_idx + 3] as f32 / 255.0,
                        )
                    } else if !skin_row.is_empty() {
                        (skin_row[x] as f32 / 255.0, 0.0, 0.0, 0.0)
                    } else {
                        (0.0, 0.0, 0.0, 0.0)
                    };

                    // Extensión orgánica hacia el cuello (debajo de la barbilla, sin cortes rectos ni cajas)
                    if skin_val < 0.05 {
                        if let Some(face) = active_face {
                            if let (Some(chin_p), Some(jaw_l_p), Some(jaw_r_p)) = (face.landmarks.get(152), face.landmarks.get(172), face.landmarks.get(397)) {
                                if py > chin_p.y && px >= jaw_l_p.x - 15.0 && px <= jaw_r_p.x + 15.0 {
                                    let eye_d = face_geom.map(|g| g.2).unwrap_or(40.0);
                                    let neck_reach = eye_d * 0.90;
                                    let dy = py - chin_p.y;
                                    if dy < neck_reach {
                                        let p_val = if has_compact_seg {
                                            let sx = (x * 255) / (width - 1).max(1);
                                            mask[seg_y_offset + sx] as f32 / 255.0
                                        } else {
                                            1.0
                                        };

                                        if p_val > 0.35 {
                                            let raw_r = in_row[idx] as f32 / 255.0;
                                            let raw_g = in_row[idx + 1] as f32 / 255.0;
                                            let raw_b = in_row[idx + 2] as f32 / 255.0;
                                            let (y_px, cb_px, cr_px) = rgb_to_ycbcr(raw_r, raw_g, raw_b);
                                            if cr_px > 0.022 && cb_px > -0.16 && cb_px < 0.04 && (cr_px - cb_px) > 0.028 && y_px > 0.16 && y_px < 0.88 {
                                                let t_neck = (1.0 - dy / neck_reach).clamp(0.0, 1.0);
                                                let falloff = t_neck * t_neck * (3.0 - 2.0 * t_neck);
                                                skin_val = falloff * 0.70 * p_val;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Protección anatómica estricta de Barba, Bigote y Vello Facial
                    if skin_val > 0.01 {
                        if let Some(face) = active_face {
                            if let (Some(nose_p), Some(mouth_top_p), Some(mouth_bot_p), Some(chin_p), Some(mouth_l_p), Some(mouth_r_p), Some(jaw_l_p), Some(jaw_r_p)) = (
                                face.landmarks.get(2),   // Base de nariz
                                face.landmarks.get(13),  // Labio superior
                                face.landmarks.get(14),  // Labio inferior
                                face.landmarks.get(152), // Barbilla
                                face.landmarks.get(61),  // Comisura izquierda
                                face.landmarks.get(291), // Comisura derecha
                                face.landmarks.get(172), // Mandíbula izquierda
                                face.landmarks.get(397), // Mandíbula derecha
                            ) {
                                let is_mustache_zone = py >= nose_p.y - 2.0 && py <= mouth_top_p.y + 2.0 && px >= mouth_l_p.x - 8.0 && px <= mouth_r_p.x + 8.0;
                                let is_chin_beard_zone = py >= mouth_bot_p.y - 2.0 && py <= chin_p.y + 15.0 && px >= jaw_l_p.x && px <= jaw_r_p.x;

                                if is_mustache_zone || is_chin_beard_zone {
                                    let raw_r = in_row[idx] as f32 / 255.0;
                                    let raw_g = in_row[idx + 1] as f32 / 255.0;
                                    let raw_b = in_row[idx + 2] as f32 / 255.0;
                                    let cr = 0.5000 * raw_r - 0.4542 * raw_g - 0.0458 * raw_b;
                                    let cb = -0.1146 * raw_r - 0.3854 * raw_g + 0.5000 * raw_b;
                                    let skin_sat = (cr - cb).max(0.0);
                                    let lum = 0.2126 * raw_r + 0.7152 * raw_g + 0.0722 * raw_b;

                                    if lum < 0.38 || skin_sat < 0.040 {
                                        skin_val = 0.0;
                                    }
                                }
                            }
                        }
                    }

                    // Exclusión estricta de orejas: Si el píxel está lateralmente fuera de los pómulos/mandíbula,
                    // pertenece a la oreja, cabello o fondo. Forzar skin_val = 0.0 para no desenfocar jamás las orejas.
                    if let Some((_, _, eye_dist, face_left, face_right)) = face_geom {
                        let margin = eye_dist * 0.08;
                        if px < face_left {
                            let t = ((px - (face_left - margin)) / margin).clamp(0.0, 1.0);
                            skin_val *= t * t * (3.0 - 2.0 * t);
                        } else if px > face_right {
                            let t = (((face_right + margin) - px) / margin).clamp(0.0, 1.0);
                            skin_val *= t * t * (3.0 - 2.0 * t);
                        }
                    }

                    let mut r;
                    let mut g;
                    let mut b;
                    let mut eye_sx = px;
                    let mut eye_sy = py;
                    let mut lip_sx = px;
                    let mut lip_sy = py;

                    // Muestreo con desplazamiento de malla 2D si hay deformaciones faciales activas
                    if let Some((
                        eye_l_params, eye_r_params, nose_tip, nose_bridge,
                        brow_l, brow_r, jaw_l, jaw_r, chin,
                        mouth_cx, mouth_cy, eye_dist,
                        min_x, max_x, min_y, max_y,
                    )) = morph_data {
                        if px >= min_x && px <= max_x && py >= min_y && py <= max_y {
                            let mut sx = px;
                            let mut sy = py;

                            // 1. Ojos (Agrandamiento isotrópico y Forma de Ojos anatómica)
                            if eye_enlarge > 0.001 || eye_round > 0.001 || has_forced_eye_shape {
                                warp_eye(px, py, &eye_l_params, eye_enlarge, eye_round, &mut sx, &mut sy);
                                warp_eye(px, py, &eye_r_params, eye_enlarge, eye_round, &mut sx, &mut sy);
                                eye_sx = sx;
                                eye_sy = sy;
                            }

                            // 2. Forma (Face Slimming V-Line Mandibular anatómico y suave)
                            // Estrictamente restringido al contorno de la mandíbula:
                            // Las orejas están fuera de jaw_l.x / jaw_r.x y NUNCA se deforman ni se pellizcan
                            if face_slim > 0.001 {
                                let mid_x = (eye_l_params.cx + eye_r_params.cx) * 0.5;
                                let min_jaw_y = nose_bridge.y;
                                let max_jaw_y = chin.y + eye_dist * 0.15;
                                if py >= min_jaw_y && py <= max_jaw_y {
                                    let y_span = (max_jaw_y - min_jaw_y).max(10.0);
                                    let ty = ((py - min_jaw_y) / y_span).clamp(0.0, 1.0);
                                    let y_falloff = (ty * std::f32::consts::PI).sin();

                                    // Mandíbula izquierda (estrictamente entre jaw_l.x y mid_x)
                                    // Píxeles en px < jaw_l.x pertenecen a la oreja izquierda y quedan completamente intactos
                                    if px >= jaw_l.x && px < mid_x {
                                        let span_x = (mid_x - jaw_l.x).max(10.0);
                                        let tx = (px - jaw_l.x) / span_x;
                                        // Hermite / seno suave: 0 en la oreja (jaw_l.x) y 0 en la barbilla (mid_x)
                                        let x_falloff = (tx * std::f32::consts::PI).sin().powi(2);
                                        let w = x_falloff * y_falloff * face_slim * (eye_dist * 0.18);
                                        sx -= w;
                                        sy -= w * 0.08;
                                    }
                                    // Mandíbula derecha (estrictamente entre mid_x y jaw_r.x)
                                    // Píxeles en px > jaw_r.x pertenecen a la oreja derecha y quedan completamente intactos
                                    else if px > mid_x && px <= jaw_r.x {
                                        let span_x = (jaw_r.x - mid_x).max(10.0);
                                        let tx = (jaw_r.x - px) / span_x;
                                        let x_falloff = (tx * std::f32::consts::PI).sin().powi(2);
                                        let w = x_falloff * y_falloff * face_slim * (eye_dist * 0.18);
                                        sx += w;
                                        sy -= w * 0.08;
                                    }
                                }
                            }

                            // 3. Elevar cejas (Brow Lift)
                            if brow_lift > 0.001 {
                                let r_brow = eye_dist * 0.45;
                                let r_brow_sq = r_brow * r_brow;
                                let dbl_x = px - brow_l.x;
                                let dbl_y = py - brow_l.y;
                                let d_bl_sq = dbl_x * dbl_x + dbl_y * dbl_y;
                                if d_bl_sq < r_brow_sq {
                                    let d = d_bl_sq.sqrt();
                                    let w = (1.0 - d / r_brow).powi(2) * brow_lift * (eye_dist * 0.16);
                                    sy += w;
                                }
                                let dbr_x = px - brow_r.x;
                                let dbr_y = py - brow_r.y;
                                let d_br_sq = dbr_x * dbr_x + dbr_y * dbr_y;
                                if d_br_sq < r_brow_sq {
                                    let d = d_br_sq.sqrt();
                                    let w = (1.0 - d / r_brow).powi(2) * brow_lift * (eye_dist * 0.16);
                                    sy += w;
                                }
                            }

                            // 4. Nariz (Nose Slim / Widen anatómico y continuo)
                            if nose_slim.abs() > 0.001 {
                                let r_nose_x = eye_dist * 0.32;
                                let r_nose_y = eye_dist * 0.40;
                                let dnx = (px - nose_tip.x).abs();
                                let dny = (py - (nose_bridge.y + nose_tip.y) * 0.5).abs();
                                if dnx < r_nose_x && dny < r_nose_y {
                                    let tx = (dnx / r_nose_x).clamp(0.0, 1.0);
                                    // Seno suave: 0 en el centro del tabique/punta (CERO discontinuidad ni saltos),
                                    // 0 en el borde exterior, y máximo en las aletas nasales.
                                    let wx = (tx * std::f32::consts::PI).sin();
                                    let ty = (dny / r_nose_y).clamp(0.0, 1.0);
                                    let wy = (1.0 - ty).powi(2);
                                    let factor = wx * wy * nose_slim * (eye_dist * 0.14);
                                    if px < nose_tip.x {
                                        sx -= factor;
                                    } else {
                                        sx += factor;
                                    }
                                }
                            }

                            // 5. Labios (Lip Plump)
                            if lip_plump > 0.001 {
                                let r_lip = eye_dist * 0.36;
                                let r_lip_sq = r_lip * r_lip;
                                let d_lip_x = px - mouth_cx;
                                let d_lip_y = py - mouth_cy;
                                let d_lip_sq = d_lip_x * d_lip_x + d_lip_y * d_lip_y;
                                if d_lip_sq < r_lip_sq {
                                    let d = d_lip_sq.sqrt();
                                    let w = (1.0f32 - d / r_lip).powi(2) * lip_plump * 0.32;
                                    sy -= d_lip_y * w;
                                    sx -= d_lip_x * (w * 0.5);
                                }
                            }
                            lip_sx = sx;
                            lip_sy = sy;

                            if (sx - px).abs() > 0.01 || (sy - py).abs() > 0.01 {
                                let csx = sx.clamp(0.0, (width - 1) as f32);
                                let csy = sy.clamp(0.0, (height - 1) as f32);
                                let x0 = csx.floor() as usize;
                                let y0 = csy.floor() as usize;
                                let x1 = (x0 + 1).min(width - 1);
                                let y1 = (y0 + 1).min(height - 1);
                                let u = csx - x0 as f32;
                                let v = csy - y0 as f32;

                                let p00 = (y0 * width + x0) * 4;
                                let p10 = (y0 * width + x1) * 4;
                                let p01 = (y1 * width + x0) * 4;
                                let p11 = (y1 * width + x1) * 4;

                                let samp_r = (input_rgba[p00] as f32 * (1.0 - u) + input_rgba[p10] as f32 * u) * (1.0 - v)
                                           + (input_rgba[p01] as f32 * (1.0 - u) + input_rgba[p11] as f32 * u) * v;
                                let samp_g = (input_rgba[p00 + 1] as f32 * (1.0 - u) + input_rgba[p10 + 1] as f32 * u) * (1.0 - v)
                                           + (input_rgba[p01 + 1] as f32 * (1.0 - u) + input_rgba[p11 + 1] as f32 * u) * v;
                                let samp_b = (input_rgba[p00 + 2] as f32 * (1.0 - u) + input_rgba[p10 + 2] as f32 * u) * (1.0 - v)
                                           + (input_rgba[p01 + 2] as f32 * (1.0 - u) + input_rgba[p11 + 2] as f32 * u) * v;

                                r = samp_r / 255.0;
                                g = samp_g / 255.0;
                                b = samp_b / 255.0;
                            } else {
                                r = in_row[idx] as f32 / 255.0;
                                g = in_row[idx + 1] as f32 / 255.0;
                                b = in_row[idx + 2] as f32 / 255.0;
                            }
                        } else {
                            r = in_row[idx] as f32 / 255.0;
                            g = in_row[idx + 1] as f32 / 255.0;
                            b = in_row[idx + 2] as f32 / 255.0;
                        }
                    } else {
                        r = in_row[idx] as f32 / 255.0;
                        g = in_row[idx + 1] as f32 / 255.0;
                        b = in_row[idx + 2] as f32 / 255.0;
                    }

                    // =========================================================================
                    // PASO 1: ILUMINACIÓN POR IA (Studio Ambient Fill Curvature)
                    // Curva de relleno de medios tonos continua y suave SIN máscaras cortantes
                    // Elimina completamente los halos en las paredes y artefactos de recorte
                    // =========================================================================
                    if ai_lighting {
                        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                        if luma > 0.04 && luma < 0.85 {
                            let fill_curve = 4.0 * luma * (1.0 - luma);
                            let fill = fill_curve * 0.055;
                            r = (r + fill).min(1.0);
                            g = (g + fill).min(1.0);
                            b = (b + fill).min(1.0);
                        }
                    }

                    // =========================================================================
                    // PASO 2: REDUCCIÓN DE RUIDO POR IA (Bilateral Cruzado Rápido con LUT)
                    // =========================================================================
                    let (clean_r, clean_g, clean_b) = if ai_denoise {
                        let lum_c = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                        let lum_c_i = (lum_c * 255.0) as i32;

                        let mut col_sum_r = r;
                        let mut col_sum_g = g;
                        let mut col_sum_b = b;
                        let mut w_sum = 1.0_f32;

                        let x_m1 = (x.saturating_sub(1)) * 4;
                        let x_p1 = (x + 1).min(width - 1) * 4;
                        let x_cur = x * 4;

                        let samples = [
                            (in_row[x_m1],    in_row[x_m1 + 1],    in_row[x_m1 + 2]),    // W
                            (in_row[x_p1],    in_row[x_p1 + 1],    in_row[x_p1 + 2]),    // E
                            (row_up[x_cur],   row_up[x_cur + 1],   row_up[x_cur + 2]),   // N
                            (row_down[x_cur], row_down[x_cur + 1], row_down[x_cur + 2]), // S
                        ];

                        for s in &samples {
                            let s_lum_i = (54 * s.0 as i32 + 183 * s.1 as i32 + 19 * s.2 as i32) >> 8;
                            let diff = (lum_c_i - s_lum_i).abs() as usize;
                            let w = lut[diff.min(255)];
                            let w_norm = w * (1.0 / 255.0);
                            col_sum_r += s.0 as f32 * w_norm;
                            col_sum_g += s.1 as f32 * w_norm;
                            col_sum_b += s.2 as f32 * w_norm;
                            w_sum += w;
                        }

                        let inv_w = 1.0 / w_sum;
                        let bi_r = col_sum_r * inv_w;
                        let bi_g = col_sum_g * inv_w;
                        let bi_b = col_sum_b * inv_w;

                        (r * 0.30 + bi_r * 0.70, g * 0.30 + bi_g * 0.70, b * 0.30 + bi_b * 0.70)
                    } else {
                        (r, g, b)
                    };

                    r = clean_r;
                    g = clean_g;
                    b = clean_b;

                    // =========================================================================
                    // PASO 2.5: SUAVIZADO FACIAL NATURAL (Surface Blur Bilateral Multi-Escala)
                    // Filtro bilateral de preservación de bordes estricto: alisa poros, líneas
                    // y asperezas sin sobreimpulsos, sin dobles líneas, sin halos y sin alterar
                    // rasgos anatómicos (ojos, labios, fosas nasales y cejas intactos al 100%).
                    // =========================================================================
                    if beauty_active && skin_val > 0.02 {
                        // Protección anatómica: ojos y labios quedan completamente protegidos
                        let feat_factor = (1.0 - eyes_val * 1.5).clamp(0.0, 1.0) * (1.0 - lips_val * 1.5).clamp(0.0, 1.0);
                        let mut smooth_blend = skin_intensity * skin_val * feat_factor;

                        // Si el usuario activa el realce de barba, preservar la textura y nitidez de los vellos
                        if beard_enhancement > 0.001 {
                            let packed = face_uv_map[y * width + x];
                            if let Some((u, v, region)) = unpack_face_uv(packed) {
                                let beard_w = calc_beard_uv_weight(u, v, region);
                                if beard_w > 0.05 {
                                    smooth_blend *= (1.0 - beard_w * 0.75 * beard_enhancement).clamp(0.0, 1.0);
                                }
                            }
                        }

                        if smooth_blend > 0.005 {
                            let lum_center = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                            let lum_center_i = (lum_center * 255.0) as i32;

                            // Paso de muestreo adaptativo a la resolución de imagen
                            let s1 = (width / 450).clamp(1, 3) as i32;
                            let s2 = s1 * 2;
                            let s3 = s1 * 3;

                            let mut sum_r = r * 2.0;
                            let mut sum_g = g * 2.0;
                            let mut sum_b = b * 2.0;
                            let mut sum_w = 2.0_f32;

                            // Muestreo simétrico multi-anillo (cercano, diagonal, medio, exterior)
                            let samples = [
                                // Cruz cercana (distancia 1)
                                (x as i32 - s1, y as i32, 1.00f32),
                                (x as i32 + s1, y as i32, 1.00f32),
                                (x as i32, y as i32 - s1, 1.00f32),
                                (x as i32, y as i32 + s1, 1.00f32),
                                // Diagonales (distancia 1.4)
                                (x as i32 - s1, y as i32 - s1, 0.70f32),
                                (x as i32 + s1, y as i32 - s1, 0.70f32),
                                (x as i32 - s1, y as i32 + s1, 0.70f32),
                                (x as i32 + s1, y as i32 + s1, 0.70f32),
                                // Anillo medio (distancia 2)
                                (x as i32 - s2, y as i32, 0.50f32),
                                (x as i32 + s2, y as i32, 0.50f32),
                                (x as i32, y as i32 - s2, 0.50f32),
                                (x as i32, y as i32 + s2, 0.50f32),
                                // Anillo exterior para homogeneizar tez (distancia 3)
                                (x as i32 - s3, y as i32, 0.30f32),
                                (x as i32 + s3, y as i32, 0.30f32),
                                (x as i32, y as i32 - s3, 0.30f32),
                                (x as i32, y as i32 + s3, 0.30f32),
                            ];

                            for &(nx, ny, spatial_w) in &samples {
                                let cx = nx.clamp(0, width as i32 - 1) as usize;
                                let cy = ny.clamp(0, height as i32 - 1) as usize;
                                let n_idx = (cy * width + cx) * 4;

                                let nr = input_rgba[n_idx] as f32 / 255.0;
                                let ng = input_rgba[n_idx + 1] as f32 / 255.0;
                                let nb = input_rgba[n_idx + 2] as f32 / 255.0;
                                let n_lum = (0.2126 * nr + 0.7152 * ng + 0.0722 * nb) * 255.0;

                                let diff = (lum_center_i - n_lum as i32).abs() as usize;
                                // Ponderación de rango bilateral: bordes anatómicos reales (fosas nasales,
                                // ojos, líneas de expresión marcadas) tienen diff alta y peso cero.
                                let range_w = lut[diff.min(255)];
                                let w = spatial_w * range_w;

                                sum_r += nr * w;
                                sum_g += ng * w;
                                sum_b += nb * w;
                                sum_w += w;
                            }

                            let inv_w = 1.0 / sum_w;
                            let bi_r = sum_r * inv_w;
                            let bi_g = sum_g * inv_w;
                            let bi_b = sum_b * inv_w;

                            // Combinación convexa estricta:
                            // GARANTÍA: Al ser una combinación convexa pura (alpha entre 0.0 y 1.0),
                            // NUNCA puede invertir el contraste, ni generar artefactos de doble contorno,
                            // ni sobreimpulsos, ni distorsiones de color.
                            r = r * (1.0 - smooth_blend) + bi_r * smooth_blend;
                            g = g * (1.0 - smooth_blend) + bi_g * smooth_blend;
                            b = b * (1.0 - smooth_blend) + bi_b * smooth_blend;
                        }
                    }

                    // =========================================================================
                    // PASO 2.55: MOTOR OCULAR ANATÓMICO 1080P CON SPRITES DE PUPILENTE Y PESTAÑAS 360°
                    // Usa las curvas analíticas exactas de los párpados (upper_pts / lower_pts), el
                    // Sprite 128x128 de Pupilente (fibras radiales, collarette y anillo limbal) y
                    // pestañas superiores + inferiores con micro-fibras individuales.
                    // =========================================================================
                    if let Some(([ref eye_l, ref eye_r], backlight_factor)) = eye_accents {
                        for eye in [eye_l, eye_r] {
                            if eye_sx >= eye.min_x && eye_sx <= eye.max_x && eye_sy >= eye.min_y && eye_sy <= eye.max_y {
                                // 0. Sombra de Ojos 3D Mesh UV Mapping (Malla Facial Canónica 3D Estilo Bigo/TikTok)
                                // Rasterización baricéntrica directa sobre los triángulos reales del párpado móvil,
                                // cuenca orbital y ala exterior para cobertura continua sin huecos ni saltos.
                                if eyeshadow > 0.001 {
                                    let mut sampled_uv = None;
                                    for i in 0..eye.num_triangles {
                                        if let Some(uv) = eye.mesh_triangles[i].sample_uv(eye_sx, eye_sy) {
                                            sampled_uv = Some(uv);
                                            break;
                                        }
                                    }

                                    if let Some((u_norm, v_norm)) = sampled_uv {
                                        let fx_s = u_norm.clamp(0.0, 1.0) * 255.0;
                                        let fy_s = (1.0 - v_norm.clamp(0.0, 1.0)) * 255.0;
                                        if let Some(es_sprite) = selected_eyeshadow_rgba {
                                            let [sp_r, sp_g, sp_b, sp_a] = sample_sprite_bilinear_256(es_sprite, fx_s, fy_s);
                                            let eff_alpha = (sp_a * eyeshadow).clamp(0.0, 0.95);
                                            if eff_alpha > 0.001 {
                                                if let Some((tint_r, tint_g, tint_b)) = eyeshadow_target_rgb {
                                                    let (mut target_r, mut target_g, mut target_b) = if eyeshadow_color == "smoky_black" {
                                                        let kohl = 0.06 + 0.10 * (1.0 - sp_r);
                                                        (kohl, kohl, kohl * 1.05)
                                                    } else if eyeshadow_color == "black_tea" {
                                                        (tint_r * (0.55 + 0.45 * sp_r), tint_g * (0.55 + 0.45 * sp_g), tint_b * (0.55 + 0.45 * sp_b))
                                                    } else {
                                                        (tint_r * (0.45 + 0.55 * sp_r), tint_g * (0.45 + 0.55 * sp_g), tint_b * (0.45 + 0.55 * sp_b))
                                                    };

                                                    // Glitter Pop: Micro-destellos especulares en párpado
                                                    if selected_eyeshadow_style_idx == 6 {
                                                        let noise = ((eye_sx * 13.73 + eye_sy * 37.19).sin() * 43758.5453).fract();
                                                        if noise > 0.65 {
                                                            let glint = ((noise - 0.65) / 0.35).powi(2) * 0.85;
                                                            target_r = (target_r + glint).min(1.0);
                                                            target_g = (target_g + glint * 0.90).min(1.0);
                                                            target_b = (target_b + glint * 0.75).min(1.0);
                                                        }
                                                    }

                                                    r = r * (1.0 - eff_alpha) + target_r * eff_alpha;
                                                    g = g * (1.0 - eff_alpha) + target_g * eff_alpha;
                                                    b = b * (1.0 - eff_alpha) + target_b * eff_alpha;
                                                }
                                            }
                                        }
                                    }
                                }

                                // 0.5 Sombra de Ojos en Párpado Inferior (Under-Eye Smudge 360° / Grunge / Runway)
                                // Delimitado estrictamente al párpado inferior: NUNCA desborda al puente nasal ni lagrimal
                                if eyeshadow > 0.001 {
                                    let (in_x_bound, u_outer) = if eye.is_viewer_left {
                                        let safe_inner = eye.inner_x - eye.eye_w * 0.14;
                                        let in_bounds = eye_sx >= (eye.left_x - 4.0) && eye_sx <= safe_inner;
                                        let u_out = ((safe_inner - eye_sx) / (safe_inner - eye.left_x).max(1.0)).clamp(0.0, 1.0);
                                        (in_bounds, u_out)
                                    } else {
                                        let safe_inner = eye.inner_x + eye.eye_w * 0.14;
                                        let in_bounds = eye_sx >= safe_inner && eye_sx <= (eye.right_x + 4.0);
                                        let u_out = ((eye_sx - safe_inner) / (eye.right_x - safe_inner).max(1.0)).clamp(0.0, 1.0);
                                        (in_bounds, u_out)
                                    };

                                    if in_x_bound {
                                        let y_lower_lid = sample_lid_y(&eye.lower_pts, eye_sx.clamp(eye.left_x, eye.right_x));
                                        if eye_sy >= y_lower_lid - 0.5 {
                                            let (reach_factor, density, min_u_cutoff) = match selected_eyeshadow_style_idx {
                                                4 => (0.16, 1.10, 0.05), // grunge_kohl (Foto 1: 360° oscuro difuminado)
                                                5 => (0.09, 0.90, 0.40), // foxy_glam (Fotos 2 y 4: tercio exterior conectado al ala)
                                                2 => (0.07, 0.80, 0.50), // cat_wing
                                                _ => (0.05, 0.60, 0.60), // estilos estándar
                                            };

                                            if u_outer >= min_u_cutoff {
                                                let lower_reach = (eye.eye_w * reach_factor).clamp(3.0, 16.0);
                                                let dy_low = eye_sy - y_lower_lid;
                                                if dy_low <= lower_reach {
                                                    let v_low = (dy_low / lower_reach).clamp(0.0, 1.0);
                                                    let falloff = (-v_low / 0.42).exp();
                                                    let u_weight = u_outer.powf(1.1);
                                                    let eff_alpha_low = (falloff * u_weight * density * eyeshadow * 0.80).clamp(0.0, 0.88);

                                                    if eff_alpha_low > 0.001 {
                                                        if let Some((tint_r, tint_g, tint_b)) = eyeshadow_target_rgb {
                                                            let (target_r, target_g, target_b) = if eyeshadow_color == "smoky_black" {
                                                                (0.09, 0.09, 0.10)
                                                            } else {
                                                                (tint_r * 0.70, tint_g * 0.70, tint_b * 0.70)
                                                            };

                                                            r = r * (1.0 - eff_alpha_low) + target_r * eff_alpha_low;
                                                            g = g * (1.0 - eff_alpha_low) + target_g * eff_alpha_low;
                                                            b = b * (1.0 - eff_alpha_low) + target_b * eff_alpha_low;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // 0.6 Piquito felino en lagrimal interno (Inner Corner Fox Flick para foxy_glam y cat_wing)
                                if (selected_eyeshadow_style_idx == 5 || selected_eyeshadow_style_idx == 2) && eyelashes > 0.08 {
                                    let d_inner_x = if eye.is_viewer_left { eye_sx - eye.inner_x } else { eye.inner_x - eye_sx };
                                    let flick_len = eye.eye_w * 0.10;
                                    if d_inner_x >= 0.0 && d_inner_x <= flick_len {
                                        let y_inner = eye.upper_pts[if eye.is_viewer_left { 8 } else { 0 }].1;
                                        let dy_inner = (eye_sy - y_inner).abs();
                                        let flick_width = (1.0 - (d_inner_x / flick_len)) * 2.0;
                                        if dy_inner <= flick_width {
                                            let flick_alpha = ((1.0 - dy_inner / flick_width.max(0.4)) * eyelashes * 0.92).clamp(0.0, 0.95);
                                            r = r * (1.0 - flick_alpha) + 0.08 * flick_alpha;
                                            g = g * (1.0 - flick_alpha) + 0.08 * flick_alpha;
                                            b = b * (1.0 - flick_alpha) + 0.09 * flick_alpha;
                                        }
                                    }
                                }

                                // 1. Interior de la fisura palpebral analítica 1080p (Esclera, Sprite de Pupilente y Catchlight)
                                if eye_sx >= eye.left_x && eye_sx <= eye.right_x {
                                    let y_top_raw = sample_lid_y(&eye.upper_pts, eye_sx);
                                    let y_bot_raw = sample_lid_y(&eye.lower_pts, eye_sx);
                                    // Recorte palpebral anatómico exacto: justo debajo de la línea de pestañas superiores
                                    // y en el borde interno del párpado inferior para que el círculo se corte limpiamente
                                    let y_top = y_top_raw + eye.eye_h * 0.085;
                                    let y_bot = y_bot_raw - eye.eye_h * 0.020;
                                    if eye_sy > y_top && eye_sy < y_bot {
                                        let ap_top = ((eye_sy - y_top) * 1.15).clamp(0.0, 1.0);
                                        let ap_bot = ((y_bot - eye_sy) * 1.15).clamp(0.0, 1.0);
                                        let ap_alpha = ap_top * ap_bot;

                                        let dx_i = eye_sx - eye.pupil_x;
                                        let dy_i = eye_sy - eye.pupil_y;
                                        let u_lens = dx_i * eye.inv_iris_rad;
                                        let v_lens = dy_i * eye.inv_iris_rad;
                                        let r_iris_norm = (u_lens * u_lens + v_lens * v_lens).sqrt();
                                        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;

                                        // Compuerta óptica adaptativa anti-desborde hacia la esclerótica blanca:
                                        // Evita al 100% pintar sobre la esclerótica blanca manteniendo el anillo limbal circular
                                        let scl_thresh = (eye.iris_lum + 0.52 * (eye.sclera_lum - eye.iris_lum)).max(0.23);
                                        let sclera_guard = if r_iris_norm > 0.46 {
                                            let bright_excess = ((lum - scl_thresh) * 7.5).clamp(0.0, 1.0);
                                            let edge_w = ((r_iris_norm - 0.46) / 0.36).clamp(0.0, 1.0);
                                            1.0 - bright_excess * edge_w
                                        } else {
                                            1.0
                                        };

                                        // A. SPRITE RGBA 256x256 (.PNG) DE PUPILENTE CON FIBRAS ALPHA, CRIPTAS TRANSPARENTES Y AGUJERO DE PUPILA
                                        if r_iris_norm <= 1.00 && sclera_guard > 0.02 {
                                            if let Some(png_sprite) = selected_lens_rgba {
                                                // Solo se aplica el pupilente si hay un sprite elegido y la intensidad > 0
                                                if iris_lighting > 0.001 {
                                                    let fx_s = u_lens * 126.0 + 127.5;
                                                    let fy_s = v_lens * 126.0 + 127.5;
                                                    let [sp_r, sp_g, sp_b, sp_a] = sample_sprite_bilinear_256(png_sprite, fx_s, fy_s);

                                                    // Preservar cualquier reflejo de luz real (catchlight natural) dentro de la córnea
                                                    let glint_thresh = (eye.iris_lum + 0.16).max(0.26);
                                                    let glint_preserve = if r_iris_norm < 0.72 {
                                                        ((lum - glint_thresh) * 3.4).clamp(0.0, 0.86)
                                                    } else {
                                                        0.0
                                                    };

                                                    // Sombreado esférico 3D bajo el párpado superior manteniendo tonos claros y luminosos
                                                    // incluso sobre ojos oscuros (como en la Imagen 3 de referencia)
                                                    let lid_t = ((eye_sy - y_top) / (eye.eye_h * 0.45).max(1.5)).clamp(0.0, 1.0);
                                                    let lid_shadow = (0.76 + 0.26 * lid_t) * backlight_factor.max(0.84);

                                                    // Superposición directa por canal Alpha del .PNG:
                                                    // El agujero de pupila y las criptas transparentes dejan ver el ojo real oscuro debajo
                                                    let eff_alpha = (sp_a * ap_alpha * sclera_guard * iris_lighting * (1.0 - glint_preserve) * 0.92).clamp(0.0, 0.92);
                                                    if eff_alpha > 0.001 {
                                                        let lens_r = (sp_r * lid_shadow).clamp(0.0, 1.0);
                                                        let lens_g = (sp_g * lid_shadow).clamp(0.0, 1.0);
                                                        let lens_b = (sp_b * lid_shadow).clamp(0.0, 1.0);

                                                        r = r * (1.0 - eff_alpha) + lens_r * eff_alpha;
                                                        g = g * (1.0 - eff_alpha) + lens_g * eff_alpha;
                                                        b = b * (1.0 - eff_alpha) + lens_b * eff_alpha;
                                                    }
                                                }
                                            } else if iris_lighting > 0.001 {
                                                // Modo "Natural / Sin pupilente": solo realza sutilmente el brillo del iris propio si se sube Luz de Iris
                                                let ix_s = ((u_lens * 60.0 + 63.5).round() as i32).clamp(0, 127) as usize;
                                                let iy_s = ((v_lens * 60.0 + 63.5).round() as i32).clamp(0, 127) as usize;
                                                let [fiber_lum, _, _, lens_alpha] = lens_sprite[iy_s * 128 + ix_s];
                                                let eff_alpha = lens_alpha * ap_alpha * sclera_guard * iris_lighting;
                                                let lift = 0.20 * eff_alpha * (0.45 + 0.55 * fiber_lum) * (1.0 - lum * 0.60).max(0.15);
                                                r = (r + lift * 1.06).min(1.0);
                                                g = (g + lift * 1.00).min(1.0);
                                                b = (b + lift * 0.94).min(1.0);
                                            }
                                        }
                                        // B. ESCLERÓTICA (Blanco del ojo fuera del disco del iris)
                                        else if eye_brighten > 0.001 {
                                            let (y_sc, cb_sc, cr_sc) = rgb_to_ycbcr(r, g, b);
                                            let chroma_dist = (cb_sc * cb_sc + cr_sc * cr_sc).sqrt();
                                            if (chroma_dist < 0.065 && lum > 0.14) || lum >= 0.24 {
                                                let sclera_edge = ((r_iris_norm - 0.96) * 6.0).clamp(0.0, 1.0) * ap_alpha;
                                                let clean_cr = cr_sc * (1.0 - 0.85 * eye_brighten);
                                                let clean_cb = cb_sc * (1.0 - 0.70 * eye_brighten);
                                                let lift_y = (y_sc + 0.20 * eye_brighten * (1.0 - y_sc) * sclera_edge).clamp(0.0, 1.0);
                                                let (cl_r, mut cl_g, mut cl_b) = ycbcr_to_rgb(lift_y, clean_cb, clean_cr);
                                                let pearl_boost = 0.022 * eye_brighten * sclera_edge * (1.0 - y_sc);
                                                cl_b = (cl_b + pearl_boost).min(1.0);
                                                cl_g = (cl_g + pearl_boost * 0.45).min(1.0);

                                                let w_sclera = eye_brighten * sclera_edge;
                                                r = r * (1.0 - w_sclera) + cl_r * w_sclera;
                                                g = g * (1.0 - w_sclera) + cl_g * w_sclera;
                                                b = b * (1.0 - w_sclera) + cl_b * w_sclera;
                                            }
                                        }

                                        // C. DESTELLO ESPECULAR / RING LIGHT EN PUPILA (CATCHLIGHT)
                                        // Recortado analíticamente por el párpado superior/inferior (ap_alpha) y anclado a la pupila
                                        if eye_catchlight > 0.001 && ap_alpha > 0.05 {
                                            let mut spec_lum = 0.0_f32;
                                            let d1_sq = (eye_sx - eye.hl1_x).powi(2) + (eye_sy - eye.hl1_y).powi(2);
                                            if d1_sq < eye.r_spec1_sq {
                                                let d1 = d1_sq.sqrt();
                                                let t1 = (1.0 - d1 * eye.inv_r_spec1).max(0.0);
                                                spec_lum += t1 * t1;
                                            }
                                            let d2_sq = (eye_sx - eye.hl2_x).powi(2) + (eye_sy - eye.hl2_y).powi(2);
                                            if d2_sq < eye.r_spec2_sq {
                                                let d2 = d2_sq.sqrt();
                                                let t2 = (1.0 - d2 * eye.inv_r_spec2).max(0.0);
                                                spec_lum += t2 * t2 * 0.42;
                                            }
                                            if spec_lum > 0.001 {
                                                let spec = (spec_lum * eye_catchlight * backlight_factor * ap_alpha).min(1.0);
                                                r = (r + spec * 0.95).min(1.0);
                                                g = (g + spec * 0.98).min(1.0);
                                                b = (b + spec * 1.00).min(1.0);
                                            }
                                        }
                                    }
                                }

                                // 2. PESTAÑAS SUPERIORES E INFERIORES CON PATRÓN DE MICRO-FIBRAS (SUPERPUESTAS AL BORDE PALPEBRAL)
                                if eyelashes > 0.001 {
                                    let t_outer = if eye.is_viewer_left {
                                        ((eye.inner_x - eye_sx) * eye.inv_eye_width).clamp(0.0, 1.0)
                                    } else {
                                        ((eye_sx - eye.inner_x) * eye.inv_eye_width).clamp(0.0, 1.0)
                                    };
                                    let scale_h = (eye.eye_h / 11.0).clamp(0.78, 1.45);
                                    let dir_sign = if eye.is_viewer_left { -1.0_f32 } else { 1.0_f32 };

                                    // 2A. PESTAÑAS Y DELINEADO DEL PÁRPADO SUPERIOR
                                    let mut min_seg_dist = 999.0_f32;
                                    let mut min_seg_cross = 0.0_f32;
                                    for seg in &eye.segments {
                                        let (dist, cross) = seg.distance_and_side(eye_sx, eye_sy);
                                        if dist < min_seg_dist {
                                            min_seg_dist = dist;
                                            min_seg_cross = cross;
                                        }
                                    }

                                    if min_seg_cross < 0.0 {
                                        let max_reach = (2.1 + 4.6 * t_outer) * scale_h;
                                        if min_seg_dist < max_reach {
                                            let t_lash = (1.0 - min_seg_dist / max_reach).clamp(0.0, 1.0);
                                            // Curvatura de pelitos individuales hacia el canto exterior en las puntas superiores
                                            let u_up = (eye_sx - eye.left_x) * eye.inv_eye_width - (min_seg_dist * 0.028) * dir_sign;
                                            let comb_up = 0.72 + 0.28 * (u_up * 32.0 * std::f32::consts::TAU).sin();
                                            let tip_mod = t_lash + (1.0 - t_lash) * comb_up;
                                            let lash_density = t_lash * t_lash * tip_mod * (1.05 + 0.90 * t_outer);
                                            let alpha = (lash_density * eyelashes * backlight_factor).min(0.92);
                                            let dark_tone = 0.03 * backlight_factor;
                                            r = r * (1.0 - alpha) + dark_tone * alpha;
                                            g = g * (1.0 - alpha) + dark_tone * alpha;
                                            b = b * (1.0 - alpha) + (dark_tone * 1.25) * alpha;
                                        }
                                    } else {
                                        let liner_reach = (1.0 + 1.3 * t_outer) * scale_h.min(1.2);
                                        if min_seg_dist < liner_reach {
                                            let t_liner = (1.0 - min_seg_dist / liner_reach).clamp(0.0, 1.0);
                                            let alpha = (t_liner * eyelashes * 0.90 * backlight_factor).min(0.94);
                                            let dark_tone = 0.02 * backlight_factor;
                                            r = r * (1.0 - alpha) + dark_tone * alpha;
                                            g = g * (1.0 - alpha) + dark_tone * alpha;
                                            b = b * (1.0 - alpha) + (dark_tone * 1.1) * alpha;
                                        }
                                    }

                                    // 2B. PESTAÑAS Y TIGHTLINE DEL PÁRPADO INFERIOR (RESALTE DE MIRADA)
                                    // Nacen suavemente desde el tercio interno (t_outer > 0.16) y se abren en abanico hacia el canto externo
                                    if t_outer > 0.14 && eye_sx >= eye.left_x - 2.0 && eye_sx <= eye.right_x + 2.0 {
                                        let y_bot_lid = sample_lid_y(&eye.lower_pts, eye_sx);
                                        if eye_sy >= y_bot_lid - 0.5 {
                                            let mut min_low_dist = 999.0_f32;
                                            for seg in &eye.lower_segments {
                                                let (dist, _) = seg.distance_and_side(eye_sx, eye_sy);
                                                if dist < min_low_dist {
                                                    min_low_dist = dist;
                                                }
                                            }

                                            // Envolvente anatómica: más sutil hacia el lagrimal, más definida en el tercio exterior
                                            let outer_ramp = ((t_outer - 0.14) / 0.72).clamp(0.0, 1.0);
                                            let corner_taper = ((1.02 - t_outer) * 7.0).clamp(0.0, 1.0);
                                            let lower_env = outer_ramp.sqrt() * corner_taper;

                                            // Tightline / sombreado sub-palpebral fino en la raíz de las pestañas inferiores
                                            let tightline_reach = (1.15 * scale_h).max(0.9);
                                            if min_low_dist < tightline_reach {
                                                let t_tl = (1.0 - min_low_dist / tightline_reach).clamp(0.0, 1.0);
                                                let alpha_tl = (t_tl * t_tl * lower_env * eyelashes * 0.44 * backlight_factor).min(0.55);
                                                let (shade_r, shade_g, shade_b) = if let Some((er, eg, eb)) = eyeshadow_target_rgb {
                                                    (
                                                        (0.07 * (1.0 - eyeshadow * 0.4) + er * 0.22 * eyeshadow) * backlight_factor,
                                                        (0.05 * (1.0 - eyeshadow * 0.4) + eg * 0.18 * eyeshadow) * backlight_factor,
                                                        (0.05 * (1.0 - eyeshadow * 0.4) + eb * 0.18 * eyeshadow) * backlight_factor,
                                                    )
                                                } else {
                                                    (0.07 * backlight_factor, 0.05 * backlight_factor, 0.05 * backlight_factor)
                                                };
                                                r = r * (1.0 - alpha_tl) + shade_r * alpha_tl;
                                                g = g * (1.0 - alpha_tl) + shade_g * alpha_tl;
                                                b = b * (1.0 - alpha_tl) + shade_b * alpha_tl;
                                            }

                                            // Pelitos individuales de pestañas inferiores proyectados hacia abajo y afuera
                                            if eye_sy > y_bot_lid {
                                                let max_low_reach = (1.5 + 3.1 * lower_env) * scale_h;
                                                if min_low_dist < max_low_reach {
                                                    let t_low = (1.0 - min_low_dist / max_low_reach).clamp(0.0, 1.0);
                                                    // Inclinación radial de cada pestaña inferior hacia el canto externo conforme baja
                                                    let u_low = (eye_sx - eye.left_x) * eye.inv_eye_width
                                                        - (min_low_dist * 0.040) * dir_sign;
                                                    let phase1 = (u_low * 22.0 * std::f32::consts::TAU).sin();
                                                    let phase2 = (u_low * 11.0 * std::f32::consts::TAU + 0.75).cos();
                                                    // Peine de micro-fibras separadas (pelitos finos e individuales)
                                                    let hair_comb = (0.52 + 0.48 * phase1).max(0.0).powi(2) * (0.68 + 0.32 * phase2);
                                                    let lash_low_alpha = (t_low * t_low * hair_comb * lower_env * eyelashes * 0.78 * backlight_factor).min(0.75);
                                                    if lash_low_alpha > 0.005 {
                                                        let dark_low = 0.04 * backlight_factor;
                                                        r = r * (1.0 - lash_low_alpha) + dark_low * lash_low_alpha;
                                                        g = g * (1.0 - lash_low_alpha) + (dark_low * 0.90) * lash_low_alpha;
                                                        b = b * (1.0 - lash_low_alpha) + (dark_low * 0.95) * lash_low_alpha;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // =========================================================================
                    // PASO 2.57: MÁSCARA CANÓNICA FACIAL 3D (898 TRIÁNGULOS UV ESTILO BIGO LIVE)
                    // Renderizado anatómico integrado:
                    // 1. RELLENO DE OJERAS (Región 4): Ilumina sombras oscuras de bolsas perioculares.
                    // 2. CEJAS (V 0.67..0.76): Ancladas a los landmarks reales del hueso superciliar.
                    // 3. MAQUILLAJE DE PIEL: Rubor en pómulos, luz en puente de nariz y labial.
                    // =========================================================================
                    if let Some((f_min_x, f_max_x, f_min_y, f_max_y)) = face_mesh_bounds {
                        let px_i = eye_sx as usize;
                        let py_i = eye_sy as usize;
                        if px_i >= f_min_x && px_i <= f_max_x && py_i >= f_min_y && py_i <= f_max_y {
                            let packed = face_uv_map[py_i * width + px_i];
                            if let Some((u, v, region)) = unpack_face_uv(packed) {
                                // 0. RESALTADO, TINTE Y VISAGISMO DE BARBA (Bigote, Mentón, Mandíbula y Patillas)
                                if beard_enhancement > 0.001 {
                                    let beard_w = calc_beard_uv_weight(u, v, region);
                                    if beard_w > 0.001 {
                                        apply_beard_shading(&mut r, &mut g, &mut b, beard_w, beard_enhancement, beard_target_rgb);
                                    }
                                }

                                let [mk_r, mk_g, mk_b, mk_a] = sample_face_mask_bilinear_1024(master_makeup, u, v);
                                if mk_a > 0.001 {
                                    if region == 4 {
                                        // 1. CORRECTOR NATURAL DE OJERAS (Lifting de Sombras Infraorbitales):
                                        // En lugar de pintar corrector blanco opaco, elevamos suavemente la luminancia
                                        // de las sombras oscuras del surco lagrimal hacia el tono uniforme de la mejilla.
                                        let concealer_strength = (skin_intensity * 0.90).clamp(0.0, 1.0);
                                        let eff_alpha = (mk_a * concealer_strength * 0.70).clamp(0.0, 0.80);
                                        if eff_alpha > 0.001 {
                                            let cur_lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                                            // Solo aclara sombras por debajo de medios tonos (evita quemar o crear parches)
                                            let shadow_depth = (0.62 - cur_lum).max(0.0);
                                            let lift = shadow_depth * 0.40 * eff_alpha;
                                            r = (r + lift * 1.05).min(1.0);
                                            g = (g + lift * 0.98).min(1.0);
                                            b = (b + lift * 0.92).min(1.0);
                                        }
                                    } else {
                                        // 2. CEJAS Y MAQUILLAJE INTEGRADO:
                                        let is_eyebrow = v >= 0.67 && v <= 0.76 && ((u >= 0.20 && u <= 0.46) || (u >= 0.54 && u <= 0.80));
                                        if is_eyebrow {
                                            // Realce y Densificación Anatómica de Vellos Reales:
                                            // Profundiza los folículos pilosos reales donde el usuario ya tiene cejas,
                                            // adaptándose automáticamente a la forma anatómica real de cualquier persona.
                                            if eyebrows > 0.001 {
                                                let cur_lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                                                // Detección de vellos oscuros vs piel clara circundante
                                                let hair_contrast = ((0.42 - cur_lum) / 0.35).clamp(0.0, 1.0);
                                                let brow_strength = eyebrows.clamp(0.0, 1.0);
                                                // Relleno: profundiza fuertemente el vello real (75%) y solo sombrea sutilmente la base (25%)
                                                let fill_weight = (hair_contrast * 0.75 + 0.25) * mk_a * brow_strength;
                                                let dark_factor = (fill_weight * 0.45).clamp(0.0, 0.65);

                                                if dark_factor > 0.001 {
                                                    if let Some((tint_r, tint_g, tint_b)) = eyebrow_target_rgb {
                                                        let custom_r = tint_r * (0.20 + 0.80 * r);
                                                        let custom_g = tint_g * (0.20 + 0.80 * g);
                                                        let custom_b = tint_b * (0.20 + 0.80 * b);
                                                        r = r * (1.0 - dark_factor) + custom_r * dark_factor;
                                                        g = g * (1.0 - dark_factor) + custom_g * dark_factor;
                                                        b = b * (1.0 - dark_factor) + custom_b * dark_factor;
                                                    } else {
                                                        r *= 1.0 - dark_factor;
                                                        g *= 1.0 - dark_factor;
                                                        b *= 1.0 - dark_factor;
                                                    }
                                                }
                                            }
                                        } else if v >= 0.24 && v <= 0.33 {
                                            // Tinte Labial translúcido
                                            let eff_alpha = (mk_a * lipstick * 0.65).clamp(0.0, 0.85);
                                            if eff_alpha > 0.001 {
                                                r = r * (1.0 - eff_alpha) + mk_r * eff_alpha;
                                                g = g * (1.0 - eff_alpha) + mk_g * eff_alpha;
                                                b = b * (1.0 - eff_alpha) + mk_b * eff_alpha;
                                            }
                                        } else {
                                            // Rubor en pómulos / sombreado difuminado
                                            let eff_alpha = (mk_a * foundation_intensity.max(contour).clamp(0.0, 1.0) * 0.50).clamp(0.0, 0.80);
                                            if eff_alpha > 0.001 {
                                                r = r * (1.0 - eff_alpha) + mk_r * eff_alpha;
                                                g = g * (1.0 - eff_alpha) + mk_g * eff_alpha;
                                                b = b * (1.0 - eff_alpha) + mk_b * eff_alpha;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // =========================================================================
                    // PASO 2.58: SPRITE DE LABIOS DEFORMABLE POR 4 CURVAS 1080P (BILINEAL + ZERO-JITTER)
                    // Deforma el Sprite 128x64 en tiempo real sobre las 4 curvas labiales estabilizadas,
                    // con unión continua cuando los labios están cerrados y apertura progresiva para dientes.
                    // =========================================================================
                    if let Some(ref lip) = lip_accents {
                        if lip_sx >= lip.min_x && lip_sx <= lip.max_x && lip_sy >= lip.min_y && lip_sy <= lip.max_y {
                            if lip_sx >= lip.left_x && lip_sx <= lip.right_x {
                                let y_ot = sample_lip_y(&lip.outer_top, lip_sx);
                                let y_it = sample_lip_y(&lip.inner_top, lip_sx).max(y_ot + 0.5);
                                let y_ib = sample_lip_y(&lip.inner_bot, lip_sx).max(y_it);
                                let y_ob = sample_lip_y(&lip.outer_bot, lip_sx).max(y_ib + 0.5);

                                let u_lip = ((lip_sx - lip.left_x) * lip.inv_lip_width).clamp(0.0, 1.0);

                                // Detección continua de apertura bucal (mouth_open_t = 0 cuando los labios están cerrados)
                                let mouth_gap = (y_ib - y_it).max(0.0);
                                let mouth_open_t = ((mouth_gap - 1.6) / 3.2).clamp(0.0, 1.0);
                                let y_mid = 0.5 * (y_it + y_ib);
                                let y_it_eff = y_mid * (1.0 - mouth_open_t) + y_it * mouth_open_t;
                                let y_ib_eff = y_mid * (1.0 - mouth_open_t) + y_ib * mouth_open_t;

                                // A. LABIO SUPERIOR (y_ot .. y_it_eff) O LABIO INFERIOR (y_ib_eff .. y_ob)
                                let lip_sample_opt = if lip_sy >= y_ot && lip_sy <= y_it_eff {
                                    let v_up = ((lip_sy - y_ot) / (y_it_eff - y_ot).max(0.5)).clamp(0.0, 1.0);
                                    Some((v_up, 0usize, 1.0 - v_up))
                                } else if lip_sy >= y_ib_eff && lip_sy <= y_ob {
                                    let v_low = ((lip_sy - y_ib_eff) / (y_ob - y_ib_eff).max(0.5)).clamp(0.0, 1.0);
                                    Some((v_low, 32usize, v_low))
                                } else {
                                    None
                                };

                                if let Some((v_lip, row_offset, dist_from_inner)) = lip_sample_opt {
                                    if lipstick > 0.001 {
                                        if let Some((lip_r, lip_g, lip_b)) = lipstick_target_rgb {
                                            let [outer_alpha, cushion_3d, gloss_spec, liner_shade] =
                                                sample_lip_sprite_bilinear(lip_sprite_lut, u_lip, v_lip, row_offset);

                                            // Cuando la boca está cerrada (mouth_open_t == 0), no hay hueco sin pintar en el centro;
                                            // solo cuando la boca se abre (mouth_open_t > 0) se difumina el borde húmedo interior.
                                            let t_in_open = (dist_from_inner / 0.20).clamp(0.0, 1.0);
                                            let open_inner_feather = t_in_open * t_in_open * (3.0 - 2.0 * t_in_open);
                                            let in_feather = (1.0 - mouth_open_t) + mouth_open_t * open_inner_feather;

                                            // Sombra anatómica sutil en la línea de contacto central cuando los labios están cerrados
                                            let closed_seam = (1.0 - (dist_from_inner / 0.14).clamp(0.0, 1.0)) * (1.0 - mouth_open_t);
                                            let seam_shade = 1.0 - 0.14 * closed_seam;

                                            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                                            let cr = 0.5000 * r - 0.4542 * g - 0.0458 * b;
                                            let cb = -0.1146 * r - 0.3854 * g + 0.5000 * b;
                                            let lip_warmth = (cr - cb).max(0.0);

                                            // Compuerta anti-bigote/barba suave y continua
                                            let hair_guard = if lum < 0.15 && lip_warmth < 0.032 {
                                                ((lum - 0.07) * 12.5).clamp(0.0, 1.0)
                                            } else {
                                                1.0
                                            };

                                            // Alpha global translúcido controlado 100% por el slider de Labial (`lipstick`)
                                            let alpha = (outer_alpha * in_feather * lipstick * hair_guard * 0.68).clamp(0.0, 0.72);
                                            if alpha > 0.002 {
                                                let shade = (0.35 + 0.85 * lum).clamp(0.28, 1.12)
                                                    * cushion_3d
                                                    * seam_shade
                                                    * (1.0 - 0.14 * liner_shade);
                                                let target_r = (lip_r * shade).clamp(0.0, 1.0);
                                                let target_g = (lip_g * shade).clamp(0.0, 1.0);
                                                let target_b = (lip_b * shade).clamp(0.0, 1.0);

                                                r = r * (1.0 - alpha) + target_r * alpha;
                                                g = g * (1.0 - alpha) + target_g * alpha;
                                                b = b * (1.0 - alpha) + target_b * alpha;

                                                // Toque de brillo satinado 3D en el cojín central del labio inferior y arco de Cupido
                                                if gloss_spec > 0.01 {
                                                    let gloss = gloss_spec * alpha * 0.22 * lum.clamp(0.25, 1.0);
                                                    r = (r + gloss * 1.00).min(1.0);
                                                    g = (g + gloss * 0.92).min(1.0);
                                                    b = (b + gloss * 0.95).min(1.0);
                                                }
                                            }
                                        }
                                    }
                                }
                                // B. AGUJERO DE APERTURA BUCAL (y_it_eff < lip_sy < y_ib_eff): 0% labial, 100% libre para Dientes
                                else if mouth_open_t > 0.05 && lip_sy > y_it_eff && lip_sy < y_ib_eff && teeth_whitening > 0.001 {
                                    let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                                    if lum > 0.16 && r >= b * 0.88 {
                                        let mouth_top_feather = ((lip_sy - y_it_eff) * 0.85).clamp(0.0, 1.0);
                                        let mouth_bot_feather = ((y_ib_eff - lip_sy) * 0.85).clamp(0.0, 1.0);
                                        let w = teeth_whitening * mouth_open_t * mouth_top_feather * mouth_bot_feather * (u_lip * std::f32::consts::PI).sin();
                                        let yellow_stain = ((r * 0.45 + g * 0.55) - b).max(0.0);
                                        b = (b + yellow_stain * 0.96 * w).min(1.0);
                                        r = (r - yellow_stain * 0.18 * w).max(0.0);
                                        let lift = 0.20 * w * (1.0 - lum * 0.35);
                                        r = (r + lift).min(1.0);
                                        g = (g + lift).min(1.0);
                                        b = (b + lift).min(1.0);
                                    }
                                }
                            }
                        }
                    }

                    // =========================================================================
                    // PASO 2.6: RETOQUE DE APARIENCIA FACIAL (Piel, Base, Contorno, Sombra)
                    // =========================================================================
                    if beauty_enabled && (skin_val > 0.01 || eyes_val > 0.05 || teeth_val > 0.03 || lips_val > 0.03) {

                        // =========================================================================
                        // 2.8 ACLARADO FOTOMÉTRICO DE PIEL (SKIN WHITENING / GLOW ESTILO BIGO LIVE)
                        // Aclara homogéneamente la tez sin lavar negros (cejas/barba) ni quemar brillos.
                        // =========================================================================
                        if skin_whitening > 0.001 && skin_val > 0.02 {
                            let cur_lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;

                            // Protección anatómica estricta: ojos, labios y cejas/vello oscuro
                            let eye_prot = (1.0 - eyes_val * 2.0).clamp(0.0, 1.0);
                            let lip_prot = (1.0 - lips_val * 1.8).clamp(0.0, 1.0);
                            let is_dark_hair = cur_lum < 0.18; // cejas, fosas nasales, pestañas

                            if !is_dark_hair {
                                let w_skin = skin_whitening * skin_val * eye_prot * lip_prot;
                                if w_skin > 0.005 {
                                    // Curva de campana de medios tonos fotométrica:
                                    // Máximo realce en medios tonos (0.3 - 0.7), cero en sombras y cero en brillos quemados
                                    let midtone_bell = 4.0 * cur_lum * (1.0 - cur_lum);
                                    let luma_boost = 0.20 * w_skin * midtone_bell;

                                    let (y_sk, cb_sk, cr_sk) = rgb_to_ycbcr(r, g, b);
                                    let target_y = (y_sk + luma_boost).clamp(0.0, 1.0);

                                    // Armonización sutil hacia porcelana limpia
                                    let target_cr = cr_sk * (1.0 - 0.08 * w_skin);
                                    let target_cb = cb_sk * (1.0 - 0.05 * w_skin);

                                    let (wr_r, wr_g, wr_b) = ycbcr_to_rgb(target_y, target_cb, target_cr);
                                    r = r * (1.0 - w_skin) + wr_r * w_skin;
                                    g = g * (1.0 - w_skin) + wr_g * w_skin;
                                    b = b * (1.0 - w_skin) + wr_b * w_skin;
                                }
                            }
                        }

                        // 3. Base / Tez (Foundation Skin Tone Cosmética Translúcida)
                        // Modula crominancia (Cb, Cr) hacia el tono de base elegido, conservando al 100%
                        // la luminosidad Y original (poros, volumen 3D y sombras anatómicas).
                        if let Some(target_base) = foundation_rgb {
                            if skin_val > 0.02 && foundation_intensity > 0.01 {
                                let cur_lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                                if cur_lum > 0.16 {
                                    let (cur_y, cur_cb, cur_cr) = rgb_to_ycbcr(r, g, b);
                                    let (_, tgt_cb, tgt_cr) = rgb_to_ycbcr(target_base.0, target_base.1, target_base.2);
                                    let f_blend = foundation_intensity * skin_val * 0.55;
                                    let blend_cb = cur_cb * (1.0 - f_blend) + tgt_cb * f_blend;
                                    let blend_cr = cur_cr * (1.0 - f_blend) + tgt_cr * f_blend;
                                    let (fr, fg, fb) = ycbcr_to_rgb(cur_y, blend_cb, blend_cr);
                                    r = fr;
                                    g = fg;
                                    b = fb;
                                }
                            }
                        }

                        // 4. Contorno (Cheek & Jaw Contouring)
                        if contour > 0.001 && skin_val > 0.08 {
                            if let Some((chk_l, chk_r, eye_dist)) = makeup_data {
                                let r_chk = eye_dist * 0.40;
                                let r_chk_sq = r_chk * r_chk;
                                let d_l = (px - chk_l.0).powi(2) + (py - chk_l.1).powi(2);
                                let d_r = (px - chk_r.0).powi(2) + (py - chk_r.1).powi(2);
                                let min_d_sq = d_l.min(d_r);
                                if min_d_sq < r_chk_sq {
                                    let d = min_d_sq.sqrt();
                                    let contour_w = (1.0 - d / r_chk).powi(2) * contour * skin_val;
                                    let shade = (1.0 - 0.24 * contour_w).max(0.0);
                                    r *= shade;
                                    g *= shade * 0.98;
                                    b *= shade * 0.94;
                                }
                            }
                        }
                    }

                    // =========================================================================
                    // PASO 3: CALIBRACIÓN FOTOMÉTRICA SMARTPHONE HDR (Global, Continua y Natural)
                    // =========================================================================
                    // 3.1 Exposición Global Homogénea (Cero máscaras cortantes ni agujeros oscuros)
                    r *= exp_factor;
                    g *= exp_factor;
                    b *= exp_factor;

                    r += brightness;
                    g += brightness;
                    b += brightness;

                    // 3.1.5 Recuperación de Sombras estilo Smartphone HDR (Toe Lift / Zero Black Crush)
                    // Levanta de forma natural los pliegues y texturas de la ropa negra sin lavar los negros
                    let cur_l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                    if is_ai_calibrate_active && cur_l < 0.30 {
                        let shadow_mask = (1.0 - cur_l / 0.30).max(0.0);
                        let toe_lift = 0.20 * (shadow_mask * (0.60 + 0.40 * shadow_mask));
                        r += toe_lift;
                        g += toe_lift;
                        b += toe_lift;
                    }

                    // 3.1.6 Apertura de Medios Tonos (Midtone Luma Bell-Curve)
                    // Da luminosidad homogénea y apertura clara a toda la escena como una cámara frontal
                    let cur_l_mid = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                    if is_ai_calibrate_active && cur_l_mid > 0.02 && cur_l_mid < 0.95 {
                        let mid_lift = 0.040 * (4.0 * cur_l_mid * (1.0 - cur_l_mid));
                        r += mid_lift;
                        g += mid_lift;
                        b += mid_lift;
                    }

                    // 3.1.7 Matriz de Color Retrato Flagship Smartphone (Elimina dominante verdosa/amarillenta)
                    if is_ai_calibrate_active {
                        r *= 1.02;
                        g *= 0.98;
                        b *= 1.02;
                    }

                    // 3.2 Balance de Blancos Suave y Estable
                    r *= wb_r_gain;
                    g *= wb_g_gain;
                    b *= wb_b_gain;

                    // 3.1.8 Tono de Piel Retrato Radiante (Selfie Peach Glow / Anti-Ictericia)
                    // Corrige el déficit de azul del sensor y produce el tono melocotón/rosado saludable del celular
                    if is_ai_calibrate_active {
                        let is_skin_region = skin_val > 0.05 || (r > g && g > b && (r - g) > 0.035 && cur_l_mid > 0.20 && cur_l_mid < 0.88);
                        if is_skin_region {
                            let skin_weight = if skin_val > 0.05 { 1.0 } else { 0.85 };
                            let target_r = (r * 1.01).min(1.0);
                            let target_g = (g * 0.95).min(1.0);
                            let target_b = (b.max(r * 0.56 + 0.03)).min(1.0);
                            r = r * (1.0 - skin_weight) + target_r * skin_weight;
                            g = g * (1.0 - skin_weight) + target_g * skin_weight;
                            b = b * (1.0 - skin_weight) + target_b * skin_weight;
                        }
                    }

                    // 3.3 Contraste Dinámico mediante curva S-Curve
                    if (effective_contrast - 1.0).abs() > 0.01 {
                        let c_delta = (effective_contrast - 1.0).clamp(-0.5, 0.5);
                        let r_clamped = r.clamp(0.0, 1.0);
                        let g_clamped = g.clamp(0.0, 1.0);
                        let b_clamped = b.clamp(0.0, 1.0);

                        r = r + c_delta * (r_clamped * r_clamped * (3.0 - 2.0 * r_clamped) - r_clamped);
                        g = g + c_delta * (g_clamped * g_clamped * (3.0 - 2.0 * g_clamped) - g_clamped);
                        b = b + c_delta * (b_clamped * b_clamped * (3.0 - 2.0 * b_clamped) - b_clamped);
                    }

                    // 3.4 Saturación Cinematográfica Rec. 709
                    if (saturation - 1.0).abs() > 0.01 {
                        let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                        r = (luma + (r - luma) * saturation).clamp(0.0, 1.0);
                        g = (luma + (g - luma) * saturation).clamp(0.0, 1.0);
                        b = (luma + (b - luma) * saturation).clamp(0.0, 1.0);
                    }

                    // 3.4.5 Vibrance Natural Smartphone (Realce selectivo de tonos menos saturados)
                    if is_ai_calibrate_active {
                        let cur_luma_vib = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                        let max_c = r.max(g).max(b);
                        let min_c = r.min(g).min(b);
                        let sat_c = if max_c > 1e-4 { (max_c - min_c) / max_c } else { 0.0 };
                        let vib_boost = (1.0 - sat_c).max(0.0) * 0.18;
                        r = (cur_luma_vib + (r - cur_luma_vib) * (1.0 + vib_boost)).clamp(0.0, 1.0);
                        g = (cur_luma_vib + (g - cur_luma_vib) * (1.0 + vib_boost)).clamp(0.0, 1.0);
                        b = (cur_luma_vib + (b - cur_luma_vib) * (1.0 + vib_boost)).clamp(0.0, 1.0);
                    }

                    if hue_shift.abs() > 0.001 {
                        let (h, s, v) = rgb_to_hsv(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
                        let shifted_h = (h + hue_shift) % 1.0;
                        let (recon_r, recon_g, recon_b) = hsv_to_rgb(shifted_h, s, v);
                        r = recon_r;
                        g = recon_g;
                        b = recon_b;
                    }

                    // 3.5 Nitidez y Micro-Contraste 2D Bidireccional con Noise Coring Calibrado
                    // Realza detalles de ojos, cejas, cabello y contornos sin amplificar el ruido del sensor
                    let (mut final_r, mut final_g, mut final_b) = (r, g, b);
                    if sharpness > 0.01 && x > 0 && x < width - 1 {
                        let cur_raw_r = in_row[idx] as f32 / 255.0;
                        let cur_raw_g = in_row[idx + 1] as f32 / 255.0;
                        let cur_raw_b = in_row[idx + 2] as f32 / 255.0;
                        let cur_raw_luma = 0.2126 * cur_raw_r + 0.7152 * cur_raw_g + 0.0722 * cur_raw_b;

                        let left_r = in_row[idx - 4] as f32 / 255.0;
                        let left_g = in_row[idx - 3] as f32 / 255.0;
                        let left_b = in_row[idx - 2] as f32 / 255.0;
                        let l_luma = 0.2126 * left_r + 0.7152 * left_g + 0.0722 * left_b;

                        let right_r = in_row[idx + 4] as f32 / 255.0;
                        let right_g = in_row[idx + 5] as f32 / 255.0;
                        let right_b = in_row[idx + 6] as f32 / 255.0;
                        let r_luma = 0.2126 * right_r + 0.7152 * right_g + 0.0722 * right_b;

                        let up_r = row_up[idx] as f32 / 255.0;
                        let up_g = row_up[idx + 1] as f32 / 255.0;
                        let up_b = row_up[idx + 2] as f32 / 255.0;
                        let u_luma = 0.2126 * up_r + 0.7152 * up_g + 0.0722 * up_b;

                        let down_r = row_down[idx] as f32 / 255.0;
                        let down_g = row_down[idx + 1] as f32 / 255.0;
                        let down_b = row_down[idx + 2] as f32 / 255.0;
                        let d_luma = 0.2126 * down_r + 0.7152 * down_g + 0.0722 * down_b;

                        // Filtro Laplacian 2D (horizontal + vertical) homogéneo
                        let neighbor_luma = (l_luma + r_luma + u_luma + d_luma) * 0.25;
                        let high_freq = cur_raw_luma - neighbor_luma;
                        let abs_hf = high_freq.abs();

                        // Noise Coring calibrado: ignora grano menor a 0.005 pero resalta bordes reales
                        let noise_floor = 0.005_f32;
                        if abs_hf > noise_floor {
                            let cored_hf = (abs_hf - noise_floor) * high_freq.signum();
                            let boost = cored_hf * (sharpness * 1.5);
                            final_r = (r + boost).clamp(0.0, 1.0);
                            final_g = (g + boost).clamp(0.0, 1.0);
                            final_b = (b + boost).clamp(0.0, 1.0);
                        }
                    }

                    // H. Espacio de Color
                    match color_space {
                        ColorSpace::Rec709 => {}
                        ColorSpace::SRgb => {
                            let ir = ((final_r * 1023.0).clamp(0.0, 1023.0)) as usize;
                            let ig = ((final_g * 1023.0).clamp(0.0, 1023.0)) as usize;
                            let ib = ((final_b * 1023.0).clamp(0.0, 1023.0)) as usize;
                            final_r = lut_srgb[ir];
                            final_g = lut_srgb[ig];
                            final_b = lut_srgb[ib];
                        }
                        ColorSpace::Rec601 => {
                            let r_601 = 0.98 * final_r + 0.02 * final_g;
                            let b_601 = 0.02 * final_g + 0.98 * final_b;
                            final_r = r_601;
                            final_b = b_601;
                        }
                        ColorSpace::Rec2020 => {
                            final_r = (final_r * 1.06 - 0.06 * final_g).clamp(0.0, 1.0);
                            final_g = (final_g * 1.03 - 0.03 * final_b).clamp(0.0, 1.0);
                        }
                    }

                    // I. Segmentación de Fondo (si está activada)
                    if bg_mode != BackgroundMode::None {
                        let mask_val = if has_compact_seg {
                            let sx = (x * 255) / (width - 1).max(1);
                            mask[seg_y_offset + sx] as f32 / 255.0
                        } else if !mask_row.is_empty() {
                            mask_row[x] as f32 / 255.0
                        } else {
                            1.0
                        };
                        let low_edge = (seg_threshold - edge_feather).max(0.0);
                        let high_edge = (seg_threshold + edge_feather).min(1.0);
                        let alpha = if mask_val <= low_edge {
                            0.0
                        } else if mask_val >= high_edge {
                            1.0
                        } else {
                            (mask_val - low_edge) / (high_edge - low_edge)
                        };

                        let (bg_r, bg_g, bg_b) = match bg_mode {
                            BackgroundMode::Blur => (0.12, 0.10, 0.25),
                            BackgroundMode::VirtualImage => (0.08, 0.12, 0.18),
                            BackgroundMode::GreenScreen => (0.0, 1.0, 0.0),
                            BackgroundMode::None => (final_r, final_g, final_b),
                        };

                        final_r = bg_r * (1.0 - alpha) + final_r * alpha;
                        final_g = bg_g * (1.0 - alpha) + final_g * alpha;
                        final_b = bg_b * (1.0 - alpha) + final_b * alpha;
                    }

                    // J. Compresión Filmic de Altas Luces (Soft Highlight Roll-off)
                    // Se activa solo con Auto-Tune o incremento manual de exposición
                    if is_ai_calibrate_active || config.exposure > 0.05 {
                        final_r = filmic_soft_shoulder(final_r);
                        final_g = filmic_soft_shoulder(final_g);
                        final_b = filmic_soft_shoulder(final_b);
                    }

                    // K. Capa Superior Master de Gradación de Color / 3D LUTs
                    // Aplicada como capa superior global sobre el fotograma final sin afectar la calibración base
                    let filter_lut_id = config.filter_lut.as_str();
                    let filter_weight = config.filter_intensity.clamp(0.0, 1.0);
                    if filter_lut_id != "none" && !filter_lut_id.is_empty() && filter_weight > 0.001 {
                        let (gr_r, gr_g, gr_b) = apply_filter_lut(final_r, final_g, final_b, filter_lut_id);
                        final_r = final_r * (1.0 - filter_weight) + gr_r * filter_weight;
                        final_g = final_g * (1.0 - filter_weight) + gr_g * filter_weight;
                        final_b = final_b * (1.0 - filter_weight) + gr_b * filter_weight;
                    }

                    // L. Gama de Colores (Full 0-255 vs Limited 16-235)
                    let (out_r, out_g, out_b) = match color_range {
                        ColorRange::Full => (
                            (final_r.clamp(0.0, 1.0) * 255.0) as u8,
                            (final_g.clamp(0.0, 1.0) * 255.0) as u8,
                            (final_b.clamp(0.0, 1.0) * 255.0) as u8,
                        ),
                        ColorRange::Limited => (
                            (16.0 + final_r.clamp(0.0, 1.0) * 219.0) as u8,
                            (16.0 + final_g.clamp(0.0, 1.0) * 219.0) as u8,
                            (16.0 + final_b.clamp(0.0, 1.0) * 219.0) as u8,
                        ),
                    };

                    out_row[idx] = out_r;
                    out_row[idx + 1] = out_g;
                    out_row[idx + 2] = out_b;
                    out_row[idx + 3] = 255;
                }
            });

        // Renderizado de Accesorios 3D / Props (Cuernos de Diablo Low-Poly Mesh 3D)
        if let Some(pose) = head_pose_3d {
            let drawn_cnt = self.mesh3d_renderer.render_mesh(
                &self.devil_horns_mesh,
                &pose,
                config.effect_intensity,
                &mut self.output_buffer,
            );
            static mut LAST_PIXEL_LOG: u32 = 0;
            unsafe {
                LAST_PIXEL_LOG += 1;
                if LAST_PIXEL_LOG % 60 == 1 {
                    crate::log_info!("Effects", "Cuernos 3D renderizados: {} píxeles dibujados en búfer {}x{} (yaw={:.2})", drawn_cnt, width, height, pose.yaw_ratio);
                }
            }
        }

        (
            &self.output_buffer,
            PhotometricStats {
                detected_luma: effective_measured_luma,
                auto_exposure: if is_ai_calibrate_active { calculated_exp } else { 0.0 },
                auto_wb: if is_ai_calibrate_active { calculated_wb_val } else { 0.0 },
                auto_contrast: if is_ai_calibrate_active { calculated_contrast } else { 1.0 },
                recommended_exposure: rec_exp,
                recommended_contrast: rec_contrast,
                recommended_saturation: rec_sat,
                recommended_sharpness: rec_sharpness,
                recommended_wb: rec_wb,
            },
        )
    }
}
