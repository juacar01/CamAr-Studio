use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundMode {
    None = 0,
    Blur = 1,
    VirtualImage = 2,
    GreenScreen = 3,
}

impl Default for BackgroundMode {
    fn default() -> Self {
        BackgroundMode::None
    }
}

impl From<i32> for BackgroundMode {
    fn from(val: i32) -> Self {
        match val {
            1 => BackgroundMode::Blur,
            2 => BackgroundMode::VirtualImage,
            3 => BackgroundMode::GreenScreen,
            _ => BackgroundMode::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorRange {
    Full = 0,    // 0 - 255
    Limited = 1, // 16 - 235 (Broadcast Studio / TV)
}

impl Default for ColorRange {
    fn default() -> Self {
        ColorRange::Full
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorSpace {
    Rec709 = 0,  // Estándar HD / YouTube / Twitch
    SRgb = 1,    // Monitor PC estándar
    Rec601 = 2,  // SD NTSC / PAL
    Rec2020 = 3, // UHD Wide Gamut / HDR
}

impl Default for ColorSpace {
    fn default() -> Self {
        ColorSpace::Rec709
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CameraFormatInfo {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub format: String, // "NV12", "YUY2", "MJPG", "RGB24"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDeviceInfo {
    pub id: String,
    pub name: String,
    pub formats: Vec<CameraFormatInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point2f {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect2f {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FaceLandmarks {
    pub bounding_box: Rect2f,
    pub confidence: f32,
    pub landmarks: Vec<Point2f>,
    pub detected: bool,
}

impl Default for Rect2f {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FaceLighting {
    pub has_face: bool,
    pub face_r: f32,
    pub face_g: f32,
    pub face_b: f32,
    pub face_luma: f32,
    pub scene_r: f32,
    pub scene_g: f32,
    pub scene_b: f32,
    pub scene_luma: f32,
}

impl Default for FaceLighting {
    fn default() -> Self {
        Self {
            has_face: false,
            face_r: 0.5,
            face_g: 0.5,
            face_b: 0.5,
            face_luma: 0.5,
            scene_r: 0.5,
            scene_g: 0.5,
            scene_b: 0.5,
            scene_luma: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioConfig {
    // Selección de Cámara física (sin nada quemado)
    pub selected_camera_id: String,
    pub selected_width: u32,
    pub selected_height: u32,
    pub selected_fps: u32,
    pub selected_format: String,

    // Espacio y Gama de Color
    pub color_range: ColorRange,
    pub color_space: ColorSpace,

    // Parámetros de Imagen Manuales
    pub saturation: f32,       // 0.0 a 2.0 (default 1.0)
    pub contrast: f32,         // 0.0 a 2.0 (default 1.0)
    pub sharpness: f32,        // 0.0 a 1.0 (default 0.2)
    pub hue: f32,              // -1.0 a 1.0 (rotación de matiz)
    pub white_balance: f32,    // -1.0 a 1.0 (temperatura frío/cálido)
    pub exposure: f32,         // -2.0 a 2.0
    pub brightness: f32,       // -1.0 a 1.0

    // Switches IA estilo Nvidia Broadcast
    pub ai_lighting_enabled: bool, // Iluminación por IA
    pub ai_denoise_enabled: bool,  // Eliminación de Ruido por IA
    #[serde(default)]
    pub ai_auto_tune_enabled: bool, // Reajuste automático por IA de parámetros de imagen base

    // Auto-ajustes fotométricos en tiempo real
    pub auto_exposure: f32,
    pub auto_wb: f32,
    pub auto_contrast: f32,
    pub detected_luma: f32,

    // Captura de Audio
    pub selected_audio_id: String,
    pub audio_volume: f32,     // 0.0 a 2.0 (default 1.0)
    pub audio_muted: bool,

    // Belleza y Retoque Facial (Mejorar > Apariencia)
    pub beauty_enabled: bool,
    pub skin_smoothing: f32,
    #[serde(default)]
    pub skin_whitening: f32, // 0.0 a 1.0 (Aclarar Piel / Blanqueamiento fotométrico estilo Bigo)
    #[serde(default)]
    pub face_slim: f32, // 0.0 a 1.0 (Forma)
    #[serde(default)]
    pub eye_enlarge: f32, // 0.0 a 1.0 (Ojos)
    #[serde(default = "default_eye_shape")]
    pub eye_shape: String, // 'auto', 'almond', 'foxy', 'asian', 'doe', 'hooded'
    #[serde(default)]
    pub eye_round: f32, // 0.0 a 1.0 (Redondear / apertura palpebral vertical de ojos)
    #[serde(default)]
    pub eye_brighten: f32, // 0.0 a 1.0 (Aclarado de ojos / brillo estilo Bigo Live)
    #[serde(default)]
    pub eye_catchlight: f32, // 0.0 a 1.0 (Destello especular / ring-light en pupila estilo Bigo)
    #[serde(default)]
    pub eyelashes: f32, // 0.0 a 1.0 (Pestañas postizas y delineado anatómico estilo Bigo)
    #[serde(default)]
    pub iris_lighting: f32, // 0.0 a 1.0 (Iluminación y traslucidez de iris)
    #[serde(default = "default_foundation_tone")]
    pub iris_color: String, // 'none', 'honey', 'hazel', 'emerald', 'ice_blue', 'sapphire', 'gray', 'violet'
    #[serde(default)]
    pub brow_lift: f32, // 0.0 a 1.0 (Elevar cejas)
    #[serde(default)]
    pub nose_slim: f32, // -1.0 a 1.0 (Nariz)
    #[serde(default)]
    pub lip_plump: f32, // 0.0 a 1.0 (Labios)
    #[serde(default = "default_foundation_tone")]
    pub foundation_tone: String, // 'none' o id del tono
    #[serde(default = "default_foundation_intensity")]
    pub foundation_intensity: f32, // 0.0 a 1.0 (Intensidad de cobertura de base/tez)
    #[serde(default)]
    pub teeth_whitening: f32, // 0.0 a 1.0 (Dientes)
    #[serde(default)]
    pub lipstick: f32, // 0.0 a 1.0 (Labial)
    #[serde(default = "default_lipstick_color")]
    pub lipstick_color: String, // 'none', 'rose_natural', 'coral_peach', 'soft_pink', 'nude_velvet', 'classic_red', 'berry_plum', 'wine_burgundy'
    #[serde(default)]
    pub eyeshadow: f32, // 0.0 a 1.0 (Sombra)
    #[serde(default = "default_eyeshadow_color")]
    pub eyeshadow_color: String, // 'bronze', 'rose', 'peach', 'plum', 'smoky_black', 'gold', 'champagne', 'none'
    #[serde(default = "default_eyeshadow_style")]
    pub eyeshadow_style: String, // 'natural_nude', 'smoky_glam', 'cat_wing', 'sunset_glow'
    #[serde(default)]
    pub eyebrows: f32, // 0.0 a 1.0 (Cejas)
    #[serde(default = "default_eyebrow_color")]
    pub eyebrow_color: String, // 'natural_black', 'dark_brown', 'medium_brown', 'taupe', 'ash_blonde', 'none'
    #[serde(default = "default_eyebrow_style")]
    pub eyebrow_style: String, // 'natural_fill', 'defined_arch', 'korean_straight', 'fluffy_feathered'
    #[serde(default)]
    pub contour: f32, // 0.0 a 1.0 (Contorno)
    #[serde(default, alias = "beardEnhancement")]
    pub beard_enhancement: f32, // 0.0 a 1.0 (Realce / densificación / tinte de barba)
    #[serde(default = "default_beard_color", alias = "beardColor")]
    pub beard_color: String, // 'natural', 'black', 'dark_brown', 'medium_brown'

    pub bg_mode: BackgroundMode,
    pub blur_radius: f32,
    pub edge_feather: f32,
    pub seg_threshold: f32,

    // Detección Facial
    pub face_detection: bool,
    pub person_segmentation: bool,
    pub draw_face_box: bool,
    pub draw_face_landmarks: bool,

    pub vcam_active: bool,
    pub preview_paused: bool,

    // Aceleración de Hardware (CPU, GPU, NPU)
    #[serde(default = "default_backend_cpu")]
    pub video_backend: String, // "cpu" (default) o "gpu_d3d12"
    #[serde(default = "default_backend_cpu")]
    pub ai_backend: String,    // "cpu" (default), "npu", o "gpu"

    // Rotación y orientación de sensor de cámara (0: Horizontal, 90: Vertical Der, 180: Invertido, 270: Vertical Izq)
    #[serde(default, alias = "cameraRotation")]
    pub camera_rotation: i32,

    // Capa Superior de Filtros Cinematográficos / 3D LUTs
    #[serde(default = "default_filter_lut")]
    pub filter_lut: String,
    #[serde(default = "default_filter_intensity")]
    pub filter_intensity: f32,

    // Perfiles de Hardware independientes por cámara física (calibración sensor 1:1)
    #[serde(default)]
    pub camera_profiles: std::collections::HashMap<String, CameraHardwareProfile>,

    // Gestión nativa de presets y apariencia almacenados en camar_beauty.json
    #[serde(default = "default_favorite_presets")]
    pub favorite_presets: Vec<String>,
    #[serde(default = "default_none_str")]
    pub active_preset_id: String,
    #[serde(default = "default_makeup_intensity")]
    pub makeup_intensity: f32,
    #[serde(default = "default_none_str", alias = "activeEffect")]
    pub active_effect: String,
    #[serde(default = "default_effect_intensity", alias = "effectIntensity")]
    pub effect_intensity: f32,
    #[serde(default = "default_mouth_scale", alias = "mouthScale")]
    pub mouth_scale: f32,
    #[serde(default = "default_eyes_scale", alias = "eyesScale")]
    pub eyes_scale: f32,
    #[serde(default = "default_eyes_distance", alias = "eyesDistance")]
    pub eyes_distance: f32,
    #[serde(default)]
    pub base_appearance: BaseAppearanceConfig,
    #[serde(default)]
    pub custom_presets: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareJsonConfig {
    pub selected_camera_id: String,
    pub selected_width: u32,
    pub selected_height: u32,
    pub selected_fps: u32,
    pub selected_format: String,
    pub color_range: ColorRange,
    pub color_space: ColorSpace,
    pub saturation: f32,
    pub contrast: f32,
    pub sharpness: f32,
    pub hue: f32,
    pub white_balance: f32,
    pub exposure: f32,
    pub brightness: f32,
    pub ai_lighting_enabled: bool,
    pub ai_denoise_enabled: bool,
    pub ai_auto_tune_enabled: bool,
    pub selected_audio_id: String,
    pub audio_volume: f32,
    pub audio_muted: bool,
    pub video_backend: String,
    pub ai_backend: String,
    #[serde(default)]
    pub camera_rotation: i32,
    #[serde(default)]
    pub camera_profiles: std::collections::HashMap<String, CameraHardwareProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeautyJsonConfig {
    pub beauty_enabled: bool,
    pub skin_smoothing: f32,
    #[serde(default)]
    pub skin_whitening: f32,
    #[serde(default)]
    pub face_slim: f32,
    #[serde(default)]
    pub eye_enlarge: f32,
    #[serde(default = "default_eye_shape")]
    pub eye_shape: String,
    #[serde(default)]
    pub eye_round: f32,
    #[serde(default)]
    pub eye_brighten: f32,
    #[serde(default)]
    pub eye_catchlight: f32,
    #[serde(default)]
    pub eyelashes: f32,
    #[serde(default)]
    pub iris_lighting: f32,
    #[serde(default = "default_foundation_tone")]
    pub iris_color: String,
    #[serde(default)]
    pub brow_lift: f32,
    #[serde(default)]
    pub nose_slim: f32,
    #[serde(default)]
    pub lip_plump: f32,
    #[serde(default = "default_foundation_tone")]
    pub foundation_tone: String,
    #[serde(default = "default_foundation_intensity")]
    pub foundation_intensity: f32,
    #[serde(default)]
    pub teeth_whitening: f32,
    #[serde(default)]
    pub lipstick: f32,
    #[serde(default = "default_lipstick_color")]
    pub lipstick_color: String,
    #[serde(default)]
    pub eyeshadow: f32,
    #[serde(default = "default_eyeshadow_color")]
    pub eyeshadow_color: String,
    #[serde(default = "default_eyeshadow_style")]
    pub eyeshadow_style: String,
    #[serde(default)]
    pub eyebrows: f32,
    #[serde(default = "default_eyebrow_color")]
    pub eyebrow_color: String,
    #[serde(default = "default_eyebrow_style")]
    pub eyebrow_style: String,
    #[serde(default)]
    pub contour: f32,
    #[serde(default)]
    pub beard_enhancement: f32,
    #[serde(default = "default_beard_color")]
    pub beard_color: String,
    pub bg_mode: BackgroundMode,
    pub blur_radius: f32,
    pub edge_feather: f32,
    pub seg_threshold: f32,
    pub face_detection: bool,
    pub person_segmentation: bool,
    pub draw_face_box: bool,
    pub draw_face_landmarks: bool,
    #[serde(default = "default_filter_lut")]
    pub filter_lut: String,
    #[serde(default = "default_filter_intensity")]
    pub filter_intensity: f32,
    #[serde(default = "default_favorite_presets")]
    pub favorite_presets: Vec<String>,
    #[serde(default = "default_none_str")]
    pub active_preset_id: String,
    #[serde(default = "default_makeup_intensity")]
    pub makeup_intensity: f32,
    #[serde(default = "default_none_str", alias = "activeEffect")]
    pub active_effect: String,
    #[serde(default = "default_effect_intensity", alias = "effectIntensity")]
    pub effect_intensity: f32,
    #[serde(default = "default_mouth_scale", alias = "mouthScale")]
    pub mouth_scale: f32,
    #[serde(default = "default_eyes_scale", alias = "eyesScale")]
    pub eyes_scale: f32,
    #[serde(default = "default_eyes_distance", alias = "eyesDistance")]
    pub eyes_distance: f32,
    #[serde(default)]
    pub base_appearance: BaseAppearanceConfig,
    #[serde(default)]
    pub custom_presets: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseAppearanceConfig {
    #[serde(default = "default_smoothing_100", alias = "skinSmoothing")]
    pub skin_smoothing: f32,
    #[serde(default, alias = "skinWhitening")]
    pub skin_whitening: f32,
    #[serde(default, alias = "faceSlim")]
    pub face_slim: f32,
    #[serde(default, alias = "eyeEnlarge")]
    pub eye_enlarge: f32,
    #[serde(default = "default_eye_shape", alias = "eyeShape")]
    pub eye_shape: String,
    #[serde(default, alias = "eyeRound")]
    pub eye_round: f32,
    #[serde(default, alias = "eyeBrighten")]
    pub eye_brighten: f32,
    #[serde(default, alias = "eyeCatchlight")]
    pub eye_catchlight: f32,
    #[serde(default)]
    pub eyelashes: f32,
    #[serde(default, alias = "browLift")]
    pub brow_lift: f32,
    #[serde(default, alias = "noseSlim")]
    pub nose_slim: f32,
    #[serde(default, alias = "lipPlump")]
    pub lip_plump: f32,
    #[serde(default = "default_foundation_tone", alias = "foundationTone")]
    pub foundation_tone: String,
    #[serde(default = "default_foundation_intensity_100", alias = "foundationIntensity")]
    pub foundation_intensity: f32,
    #[serde(default, alias = "teethWhitening")]
    pub teeth_whitening: f32,
    #[serde(default)]
    pub lipstick: f32,
    #[serde(default = "default_lipstick_color", alias = "lipstickColor")]
    pub lipstick_color: String,
    #[serde(default)]
    pub eyeshadow: f32,
    #[serde(default = "default_eyeshadow_color", alias = "eyeshadowColor")]
    pub eyeshadow_color: String,
    #[serde(default = "default_eyeshadow_style", alias = "eyeshadowStyle")]
    pub eyeshadow_style: String,
    #[serde(default)]
    pub eyebrows: f32,
    #[serde(default = "default_eyebrow_color", alias = "eyebrowColor")]
    pub eyebrow_color: String,
    #[serde(default = "default_eyebrow_style", alias = "eyebrowStyle")]
    pub eyebrow_style: String,
    #[serde(default)]
    pub contour: f32,
    #[serde(default, alias = "beardEnhancement")]
    pub beard_enhancement: f32,
    #[serde(default = "default_beard_color", alias = "beardColor")]
    pub beard_color: String,
    #[serde(default, alias = "irisLighting")]
    pub iris_lighting: f32,
    #[serde(default = "default_none_str", alias = "irisColor")]
    pub iris_color: String,
    #[serde(default = "default_none_str", alias = "activeEffect")]
    pub active_effect: String,
    #[serde(default = "default_effect_intensity", alias = "effectIntensity")]
    pub effect_intensity: f32,
    #[serde(default = "default_mouth_scale", alias = "mouthScale")]
    pub mouth_scale: f32,
    #[serde(default = "default_eyes_scale", alias = "eyesScale")]
    pub eyes_scale: f32,
    #[serde(default = "default_eyes_distance", alias = "eyesDistance")]
    pub eyes_distance: f32,
}

fn default_smoothing_100() -> f32 { 45.0 }
fn default_foundation_intensity_100() -> f32 { 40.0 }
fn default_none_str() -> String { "none".to_string() }
fn default_lipstick_color() -> String { "rose_natural".to_string() }
fn default_eyeshadow_color() -> String { "bronze".to_string() }
fn default_eyeshadow_style() -> String { "natural_nude".to_string() }
fn default_eyebrow_color() -> String { "dark_brown".to_string() }
fn default_eyebrow_style() -> String { "defined_arch".to_string() }
fn default_eye_shape() -> String { "auto".to_string() }
fn default_favorite_presets() -> Vec<String> {
    vec![
        "charm".to_string(),
        "sakura_light".to_string(),
        "supermodel".to_string(),
        "natural_glow".to_string(),
        "masculino_clean".to_string(),
    ]
}
fn default_makeup_intensity() -> f32 { 0.65 }
fn default_effect_intensity() -> f32 { 1.0 }
fn default_mouth_scale() -> f32 { 2.2 }
fn default_eyes_scale() -> f32 { 1.7 }
fn default_eyes_distance() -> f32 { 1.0 }

impl Default for BaseAppearanceConfig {
    fn default() -> Self {
        Self {
            skin_smoothing: 45.0,
            skin_whitening: 0.0,
            face_slim: 0.0,
            eye_enlarge: 0.0,
            eye_shape: "auto".to_string(),
            eye_round: 0.0,
            eye_brighten: 0.0,
            eye_catchlight: 0.0,
            eyelashes: 0.0,
            brow_lift: 0.0,
            nose_slim: 0.0,
            lip_plump: 0.0,
            foundation_tone: "none".to_string(),
            foundation_intensity: 40.0,
            teeth_whitening: 0.0,
            lipstick: 0.0,
            lipstick_color: "rose_natural".to_string(),
            eyeshadow: 0.0,
            eyeshadow_color: "bronze".to_string(),
            eyeshadow_style: "natural_nude".to_string(),
            eyebrows: 45.0,
            eyebrow_color: "dark_brown".to_string(),
            eyebrow_style: "defined_arch".to_string(),
            contour: 0.0,
            beard_enhancement: 0.0,
            beard_color: "natural".to_string(),
            iris_lighting: 0.0,
            iris_color: "none".to_string(),
            active_effect: "none".to_string(),
            effect_intensity: 1.0,
            mouth_scale: 2.2,
            eyes_scale: 1.7,
            eyes_distance: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraHardwareProfile {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub format: String,
    pub saturation: f32,
    pub contrast: f32,
    pub sharpness: f32,
    pub hue: f32,
    pub white_balance: f32,
    pub exposure: f32,
    pub brightness: f32,
    pub color_range: ColorRange,
    pub color_space: ColorSpace,
    #[serde(default)]
    pub rotation: i32,
}

fn default_filter_lut() -> String {
    "none".to_string()
}

fn default_filter_intensity() -> f32 {
    0.80
}

fn default_foundation_tone() -> String {
    "none".to_string()
}

fn default_foundation_intensity() -> f32 {
    0.40
}

fn default_beard_color() -> String {
    "natural".to_string()
}

fn default_backend_cpu() -> String {
    "cpu".to_string()
}

impl StudioConfig {
    fn get_exe_dir() -> std::path::PathBuf {
        let cur = std::path::PathBuf::from(".");
        if cur.join("camar_beauty.json").exists() || cur.join("camar_presets.json").exists() {
            return cur;
        }
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                return parent.to_path_buf();
            }
        }
        std::path::PathBuf::from(".")
    }

    pub fn get_hardware_file_path() -> std::path::PathBuf {
        Self::get_exe_dir().join("camar_hardware.json")
    }

    pub fn get_beauty_file_path() -> std::path::PathBuf {
        Self::get_exe_dir().join("camar_beauty.json")
    }

    pub fn get_legacy_presets_file_path() -> std::path::PathBuf {
        Self::get_exe_dir().join("camar_presets.json")
    }

    pub fn sync_base_appearance_from_top_level(&mut self) {
        self.base_appearance.skin_smoothing = (self.skin_smoothing * 100.0).round();
        self.base_appearance.skin_whitening = (self.skin_whitening * 100.0).round();
        self.base_appearance.face_slim = (self.face_slim * 100.0).round();
        self.base_appearance.eye_enlarge = (self.eye_enlarge * 100.0).round();
        self.base_appearance.eye_shape = self.eye_shape.clone();
        self.base_appearance.eye_round = (self.eye_round * 100.0).round();
        self.base_appearance.eye_brighten = (self.eye_brighten * 100.0).round();
        self.base_appearance.eye_catchlight = (self.eye_catchlight * 100.0).round();
        self.base_appearance.eyelashes = (self.eyelashes * 100.0).round();
        self.base_appearance.brow_lift = (self.brow_lift * 100.0).round();
        self.base_appearance.nose_slim = (self.nose_slim * 50.0).round();
        self.base_appearance.lip_plump = (self.lip_plump * 100.0).round();
        self.base_appearance.foundation_tone = self.foundation_tone.clone();
        self.base_appearance.foundation_intensity = (self.foundation_intensity * 100.0).round();
        self.base_appearance.teeth_whitening = (self.teeth_whitening * 100.0).round();
        self.base_appearance.lipstick = (self.lipstick * 100.0).round();
        self.base_appearance.lipstick_color = self.lipstick_color.clone();
        self.base_appearance.eyeshadow = (self.eyeshadow * 100.0).round();
        self.base_appearance.eyeshadow_color = self.eyeshadow_color.clone();
        self.base_appearance.eyeshadow_style = self.eyeshadow_style.clone();
        self.base_appearance.eyebrows = (self.eyebrows * 100.0).round();
        self.base_appearance.eyebrow_color = self.eyebrow_color.clone();
        self.base_appearance.eyebrow_style = self.eyebrow_style.clone();
        self.base_appearance.contour = (self.contour * 100.0).round();
        self.base_appearance.beard_enhancement = (self.beard_enhancement * 100.0).round();
        self.base_appearance.beard_color = self.beard_color.clone();
        self.base_appearance.iris_lighting = (self.iris_lighting * 100.0).round();
        self.base_appearance.iris_color = self.iris_color.clone();
        self.base_appearance.active_effect = self.active_effect.clone();
        self.base_appearance.effect_intensity = self.effect_intensity;
        self.base_appearance.mouth_scale = self.mouth_scale;
        self.base_appearance.eyes_scale = self.eyes_scale;
        self.base_appearance.eyes_distance = self.eyes_distance;
    }

    pub fn to_hardware_json(&self) -> HardwareJsonConfig {
        HardwareJsonConfig {
            selected_camera_id: self.selected_camera_id.clone(),
            selected_width: self.selected_width,
            selected_height: self.selected_height,
            selected_fps: self.selected_fps,
            selected_format: self.selected_format.clone(),
            color_range: self.color_range,
            color_space: self.color_space,
            saturation: self.saturation,
            contrast: self.contrast,
            sharpness: self.sharpness,
            hue: self.hue,
            white_balance: self.white_balance,
            exposure: self.exposure,
            brightness: self.brightness,
            ai_lighting_enabled: self.ai_lighting_enabled,
            ai_denoise_enabled: self.ai_denoise_enabled,
            ai_auto_tune_enabled: self.ai_auto_tune_enabled,
            selected_audio_id: self.selected_audio_id.clone(),
            audio_volume: self.audio_volume,
            audio_muted: self.audio_muted,
            video_backend: self.video_backend.clone(),
            ai_backend: self.ai_backend.clone(),
            camera_rotation: self.camera_rotation,
            camera_profiles: self.camera_profiles.clone(),
        }
    }

    pub fn to_beauty_json(&self) -> BeautyJsonConfig {
        BeautyJsonConfig {
            beauty_enabled: self.beauty_enabled,
            skin_smoothing: self.skin_smoothing,
            skin_whitening: self.skin_whitening,
            face_slim: self.face_slim,
            eye_enlarge: self.eye_enlarge,
            eye_shape: self.eye_shape.clone(),
            eye_round: self.eye_round,
            eye_brighten: self.eye_brighten,
            eye_catchlight: self.eye_catchlight,
            eyelashes: self.eyelashes,
            iris_lighting: self.iris_lighting,
            iris_color: self.iris_color.clone(),
            brow_lift: self.brow_lift,
            nose_slim: self.nose_slim,
            lip_plump: self.lip_plump,
            foundation_tone: self.foundation_tone.clone(),
            foundation_intensity: self.foundation_intensity,
            teeth_whitening: self.teeth_whitening,
            lipstick: self.lipstick,
            lipstick_color: self.lipstick_color.clone(),
            eyeshadow: self.eyeshadow,
            eyeshadow_color: self.eyeshadow_color.clone(),
            eyeshadow_style: self.eyeshadow_style.clone(),
            eyebrows: self.eyebrows,
            eyebrow_color: self.eyebrow_color.clone(),
            eyebrow_style: self.eyebrow_style.clone(),
            contour: self.contour,
            beard_enhancement: self.beard_enhancement,
            beard_color: self.beard_color.clone(),
            bg_mode: self.bg_mode,
            blur_radius: self.blur_radius,
            edge_feather: self.edge_feather,
            seg_threshold: self.seg_threshold,
            face_detection: self.face_detection,
            person_segmentation: self.person_segmentation,
            draw_face_box: self.draw_face_box,
            draw_face_landmarks: self.draw_face_landmarks,
            filter_lut: self.filter_lut.clone(),
            filter_intensity: self.filter_intensity,
            favorite_presets: self.favorite_presets.clone(),
            active_preset_id: self.active_preset_id.clone(),
            makeup_intensity: self.makeup_intensity,
            active_effect: self.active_effect.clone(),
            effect_intensity: self.effect_intensity,
            mouth_scale: self.mouth_scale,
            eyes_scale: self.eyes_scale,
            eyes_distance: self.eyes_distance,
            base_appearance: self.base_appearance.clone(),
            custom_presets: self.custom_presets.clone(),
        }
    }

    pub fn load_presets() -> Self {
        let hw_path = Self::get_hardware_file_path();
        let beauty_path = Self::get_beauty_file_path();
        let legacy_path = Self::get_legacy_presets_file_path();

        let mut cfg = Self::default();

        // Migración automática desde el archivo antiguo camar_presets.json si aún existe y no se han creado los 2 nuevos
        if !hw_path.exists() && !beauty_path.exists() && legacy_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&legacy_path) {
                if let Ok(mut saved) = serde_json::from_str::<Self>(&content) {
                    saved.preview_paused = false;
                    let base_is_default = (saved.base_appearance.skin_smoothing - 45.0).abs() < 0.1
                        && saved.base_appearance.eye_enlarge == 0.0
                        && saved.base_appearance.eye_round == 0.0
                        && saved.base_appearance.eye_catchlight == 0.0
                        && saved.base_appearance.eyelashes == 0.0
                        && saved.base_appearance.foundation_tone == "none"
                        && saved.base_appearance.iris_color == "none";
                    if base_is_default {
                        saved.sync_base_appearance_from_top_level();
                    }
                    saved.save_hardware();
                    saved.save_beauty();
                    let _ = std::fs::remove_file(&legacy_path);
                    return saved;
                }
            }
        }

        // 1. Cargar configuración de Cámara y Hardware desde camar_hardware.json
        let mut hardware_updated = false;
        if hw_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&hw_path) {
                if let Ok(hw) = serde_json::from_str::<HardwareJsonConfig>(&content) {
                    cfg.selected_camera_id = hw.selected_camera_id;
                    cfg.selected_width = hw.selected_width;
                    cfg.selected_height = hw.selected_height;
                    cfg.selected_fps = hw.selected_fps;
                    cfg.selected_format = hw.selected_format;
                    cfg.color_range = hw.color_range;
                    cfg.color_space = hw.color_space;
                    cfg.saturation = hw.saturation;
                    cfg.contrast = hw.contrast;
                    cfg.sharpness = hw.sharpness;
                    cfg.hue = hw.hue;
                    cfg.white_balance = hw.white_balance;
                    cfg.exposure = hw.exposure;
                    cfg.brightness = hw.brightness;
                    cfg.ai_lighting_enabled = hw.ai_lighting_enabled;
                    cfg.ai_denoise_enabled = hw.ai_denoise_enabled;
                    cfg.ai_auto_tune_enabled = hw.ai_auto_tune_enabled;
                    cfg.selected_audio_id = hw.selected_audio_id;
                    cfg.audio_volume = hw.audio_volume;
                    cfg.audio_muted = hw.audio_muted;
                    cfg.video_backend = hw.video_backend;
                    cfg.ai_backend = hw.ai_backend;
                    cfg.camera_rotation = hw.camera_rotation;
                    cfg.camera_profiles = hw.camera_profiles;
                }
            }
        } else {
            // Primer inicio: Detección automática del mejor hardware disponible (NPU / GPU)
            let hw_info = crate::hardware::SystemHardwareInfo::detect();
            cfg.video_backend = hw_info.get_optimal_video_backend();
            cfg.ai_backend = hw_info.get_optimal_ai_backend();
            hardware_updated = true;
        }

        // Validación rigurosa de Hardware: Detección en tiempo real y cascada obligatoria (NPU -> GPU -> CPU)
        let hw_info = crate::hardware::SystemHardwareInfo::detect();

        // 1. Validar backend de IA: si no tiene NPU, usar GPU; si no tiene GPU, usar CPU
        if cfg.ai_backend == "npu" && !hw_info.has_npu {
            let fallback = if hw_info.has_dedicated_gpu { "gpu".to_string() } else { "cpu".to_string() };
            crate::log_warn!(
                "Hardware",
                "Backend guardado 'npu' no está presente en esta máquina. Cascada automática a '{}'.",
                fallback
            );
            cfg.ai_backend = fallback;
            hardware_updated = true;
        } else if cfg.ai_backend == "gpu" && !hw_info.has_dedicated_gpu {
            crate::log_warn!(
                "Hardware",
                "Backend guardado 'gpu' no soportado sin GPU dedicada. Cascada automática a 'cpu'."
            );
            cfg.ai_backend = "cpu".to_string();
            hardware_updated = true;
        } else if cfg.ai_backend == "cpu" && (hw_info.has_npu || hw_info.has_dedicated_gpu) {
            // Auto-promover si la máquina tiene NPU o GPU disponible
            let optimal = hw_info.get_optimal_ai_backend();
            crate::log_info!(
                "Hardware",
                "Hardware acelerado detectado. Promoviendo backend IA a '{}'.",
                optimal
            );
            cfg.ai_backend = optimal;
            hardware_updated = true;
        }

        // 2. Validar backend de Video:
        if cfg.video_backend == "gpu_d3d12" && !hw_info.has_dedicated_gpu {
            crate::log_warn!(
                "Hardware",
                "Aceleración de video 'gpu_d3d12' no soportada sin GPU dedicada. Cascada automática a 'cpu'."
            );
            cfg.video_backend = "cpu".to_string();
            hardware_updated = true;
        } else if cfg.video_backend == "cpu" && hw_info.has_dedicated_gpu {
            cfg.video_backend = hw_info.get_optimal_video_backend();
            hardware_updated = true;
        }

        if hardware_updated {
            cfg.save_hardware();
        }

        // 2. Cargar configuración de Belleza, Filtros y Presets de Usuario desde camar_beauty.json (100% independiente de la cámara)
        if beauty_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&beauty_path) {
                if let Ok(b) = serde_json::from_str::<BeautyJsonConfig>(&content) {
                    cfg.beauty_enabled = b.beauty_enabled;
                    cfg.skin_smoothing = b.skin_smoothing;
                    cfg.skin_whitening = b.skin_whitening;
                    cfg.face_slim = b.face_slim;
                    cfg.eye_enlarge = b.eye_enlarge;
                    cfg.eye_shape = b.eye_shape;
                    cfg.eye_round = b.eye_round;
                    cfg.eye_brighten = b.eye_brighten;
                    cfg.eye_catchlight = b.eye_catchlight;
                    cfg.eyelashes = b.eyelashes;
                    cfg.iris_lighting = b.iris_lighting;
                    cfg.iris_color = b.iris_color;
                    cfg.brow_lift = b.brow_lift;
                    cfg.nose_slim = b.nose_slim;
                    cfg.lip_plump = b.lip_plump;
                    cfg.foundation_tone = b.foundation_tone;
                    cfg.foundation_intensity = b.foundation_intensity;
                    cfg.teeth_whitening = b.teeth_whitening;
                    cfg.lipstick = b.lipstick;
                    cfg.lipstick_color = b.lipstick_color;
                    cfg.eyeshadow = b.eyeshadow;
                    cfg.eyeshadow_color = b.eyeshadow_color;
                    cfg.eyeshadow_style = b.eyeshadow_style;
                    cfg.eyebrows = b.eyebrows;
                    cfg.eyebrow_color = b.eyebrow_color;
                    cfg.eyebrow_style = b.eyebrow_style;
                    cfg.contour = b.contour;
                    cfg.beard_enhancement = b.beard_enhancement;
                    cfg.beard_color = b.beard_color;
                    cfg.bg_mode = b.bg_mode;
                    cfg.blur_radius = b.blur_radius;
                    cfg.edge_feather = b.edge_feather;
                    cfg.seg_threshold = b.seg_threshold;
                    cfg.face_detection = b.face_detection;
                    cfg.person_segmentation = b.person_segmentation;
                    cfg.draw_face_box = b.draw_face_box;
                    cfg.draw_face_landmarks = b.draw_face_landmarks;
                    cfg.filter_lut = b.filter_lut;
                    cfg.filter_intensity = b.filter_intensity;
                    cfg.favorite_presets = b.favorite_presets;
                    cfg.active_preset_id = b.active_preset_id;
                    cfg.makeup_intensity = b.makeup_intensity;
                    cfg.active_effect = b.active_effect;
                    cfg.effect_intensity = b.effect_intensity;
                    cfg.mouth_scale = b.mouth_scale;
                    cfg.eyes_scale = b.eyes_scale;
                    cfg.eyes_distance = b.eyes_distance;
                    cfg.base_appearance = b.base_appearance;
                    cfg.custom_presets = b.custom_presets;

                    let base_is_default = (cfg.base_appearance.skin_smoothing - 45.0).abs() < 0.1
                        && cfg.base_appearance.eye_enlarge == 0.0
                        && cfg.base_appearance.eye_round == 0.0
                        && cfg.base_appearance.eye_catchlight == 0.0
                        && cfg.base_appearance.eyelashes == 0.0
                        && cfg.base_appearance.foundation_tone == "none"
                        && cfg.base_appearance.iris_color == "none";
                    if base_is_default {
                        cfg.sync_base_appearance_from_top_level();
                    }
                }
            }
        } else {
            cfg.save_beauty();
        }

        // Eliminar el archivo antiguo si todavía existía junto a los nuevos
        if legacy_path.exists() {
            let _ = std::fs::remove_file(&legacy_path);
        }

        cfg
    }

    pub fn save_hardware(&self) {
        let path = Self::get_hardware_file_path();
        if let Ok(content) = serde_json::to_string_pretty(&self.to_hardware_json()) {
            let _ = std::fs::write(path, content);
        }
    }

    pub fn save_beauty(&self) {
        let path = Self::get_beauty_file_path();
        if let Ok(content) = serde_json::to_string_pretty(&self.to_beauty_json()) {
            let _ = std::fs::write(path, content);
        }
    }

    #[allow(dead_code)]
    pub fn save_presets(&self) {
        self.save_hardware();
        self.save_beauty();
    }
}

impl Default for StudioConfig {
    fn default() -> Self {
        Self {
            selected_camera_id: "synthetic".to_string(),
            selected_width: 1920,
            selected_height: 1080,
            selected_fps: 60,
            selected_format: "NV12".to_string(),

            color_range: ColorRange::Full,
            color_space: ColorSpace::Rec709,

            saturation: 1.0,
            contrast: 1.0,
            sharpness: 0.2,
            hue: 0.0,
            white_balance: 0.0,
            exposure: 0.0,
            brightness: 0.0,

            ai_lighting_enabled: false,
            ai_denoise_enabled: false,
            ai_auto_tune_enabled: false,

            auto_exposure: 0.0,
            auto_wb: 0.0,
            auto_contrast: 1.0,
            detected_luma: 0.5,

            selected_audio_id: "default".to_string(),
            audio_volume: 1.0,
            audio_muted: false,

            beauty_enabled: true,
            skin_smoothing: 0.45,
            skin_whitening: 0.0,
            face_slim: 0.0,
            eye_enlarge: 0.0,
            eye_shape: "auto".to_string(),
            eye_round: 0.0,
            eye_brighten: 0.0,
            eye_catchlight: 0.0,
            eyelashes: 0.0,
            iris_lighting: 0.0,
            iris_color: "none".to_string(),
            brow_lift: 0.0,
            nose_slim: 0.0,
            lip_plump: 0.0,
            foundation_tone: "none".to_string(),
            foundation_intensity: 0.40,
            teeth_whitening: 0.0,
            lipstick: 0.0,
            lipstick_color: "rose_natural".to_string(),
            eyeshadow: 0.0,
            eyeshadow_color: "bronze".to_string(),
            eyeshadow_style: "natural_nude".to_string(),
            eyebrows: 0.45,
            eyebrow_color: "dark_brown".to_string(),
            eyebrow_style: "defined_arch".to_string(),
            contour: 0.0,
            beard_enhancement: 0.0,
            beard_color: "natural".to_string(),
            bg_mode: BackgroundMode::None,
            blur_radius: 16.0,
            edge_feather: 1.5,
            seg_threshold: 0.5,
            face_detection: true,
            person_segmentation: true,
            draw_face_box: true,
            draw_face_landmarks: false,
            vcam_active: false,
            preview_paused: false,
            video_backend: "cpu".to_string(),
            ai_backend: "cpu".to_string(),
            camera_rotation: 0,
            filter_lut: "none".to_string(),
            filter_intensity: 0.80,
            camera_profiles: std::collections::HashMap::new(),
            favorite_presets: default_favorite_presets(),
            active_preset_id: "none".to_string(),
            makeup_intensity: 0.65,
            active_effect: "none".to_string(),
            effect_intensity: 1.0,
            mouth_scale: 2.2,
            eyes_scale: 1.7,
            eyes_distance: 1.0,
            base_appearance: BaseAppearanceConfig::default(),
            custom_presets: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineTelemetry {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub fps: f32,
    #[serde(rename = "frameTimeMs")]
    pub frame_time_ms: f32,
    #[serde(rename = "aiMs")]
    pub ai_ms: f32,
    #[serde(rename = "gpuMs")]
    pub gpu_ms: f32,
    #[serde(rename = "vcamActive")]
    pub vcam_active: bool,
    #[serde(rename = "gpuName")]
    pub gpu_name: String,
    pub faces: usize,

    // Métricas fotométricas e IA
    #[serde(rename = "detectedLuma")]
    pub detected_luma: f32,
    #[serde(rename = "autoExposure")]
    pub auto_exposure: f32,
    #[serde(rename = "autoWb")]
    pub auto_wb: f32,
    #[serde(rename = "aiLightingActive")]
    pub ai_lighting_active: bool,
    #[serde(rename = "aiDenoiseActive")]
    pub ai_denoise_active: bool,
    #[serde(rename = "aiAutoTuneActive")]
    pub ai_auto_tune_active: bool,
    #[serde(rename = "recommendedExposure")]
    pub recommended_exposure: f32,
    #[serde(rename = "recommendedContrast")]
    pub recommended_contrast: f32,
    #[serde(rename = "recommendedSaturation")]
    pub recommended_saturation: f32,
    #[serde(rename = "recommendedSharpness")]
    pub recommended_sharpness: f32,
    #[serde(rename = "recommendedWb")]
    pub recommended_wb: f32,

    // Audio metering en tiempo real
    #[serde(rename = "audioPeakDb")]
    pub audio_peak_db: f32,
    #[serde(rename = "audioRmsDb")]
    pub audio_rms_db: f32,

    // Aceleración de Hardware activa
    #[serde(rename = "activeVideoBackend")]
    pub active_video_backend: String,
    #[serde(rename = "activeAiBackend")]
    pub active_ai_backend: String,
    #[serde(rename = "hardware")]
    pub hardware: crate::hardware::SystemHardwareInfo,
}
