export type BackgroundMode = 0 | 1 | 2 | 3 | 4; // 0: None, 1: Blur, 2: Image, 3: GreenScreen

export interface CameraFormatInfo {
  width: number;
  height: number;
  fps: number;
  format: string; // "NV12", "YUY2", "MJPG", "RGB24"
}

export interface CameraDeviceInfo {
  id: string;
  name: string;
  formats: CameraFormatInfo[];
}

export interface AudioDeviceInfo {
  id: string;
  name: string;
  is_default: boolean;
}

export interface DevicesPayload {
  type: 'DEVICES_LIST';
  cameras: CameraDeviceInfo[];
  microphones: AudioDeviceInfo[];
}

export interface EffectState {
  // Configuración de Hardware de Video (Cero quemado)
  selectedCameraId: string;
  selectedWidth: number;
  selectedHeight: number;
  selectedFps: number;
  selectedFormat: string;

  // Espacio y Gama de Color
  colorRange: number; // 0: Full (0-255), 1: Limited (16-235)
  colorSpace: number; // 0: Rec709, 1: sRGB, 2: Rec601, 3: Rec2020

  // Parámetros Manuales de Imagen
  saturation: number;
  contrast: number;
  sharpness: number;
  hue: number;
  whiteBalance: number;
  exposure: number;
  brightness: number;

  // Switches IA estilo Nvidia Broadcast
  aiLightingEnabled: boolean;
  aiDenoiseEnabled: boolean;
  aiAutoTuneEnabled: boolean; // Reajuste automático por IA de parámetros de imagen base

  // Audio Capture
  selectedAudioId: string;
  audioVolume: number;
  audioMuted: boolean;

  // Beauty & Facial Appearance (Mejorar > Apariencia)
  beautyEnabled: boolean;
  skinSmoothing: number;
  skinWhitening?: number;
  faceSlim: number;
  eyeEnlarge: number;
  eyeRound?: number;
  eyeBrighten?: number;
  eyeCatchlight?: number;
  eyelashes?: number;
  irisLighting?: number;
  irisColor?: string;
  browLift: number;
  noseSlim: number;
  lipPlump: number;
  foundationTone: string;
  foundationIntensity?: number;
  teethWhitening: number;
  lipstick: number;
  eyeshadow: number;
  contour: number;

  // Background
  bgMode: BackgroundMode;
  blurRadius: number;
  edgeFeather: number;
  segThreshold: number;

  // AI & Face
  faceDetection: boolean;
  personSegmentation: boolean;
  drawFaceBox: boolean;
  drawFaceLandmarks: boolean;

  // Aceleración de Hardware (CPU, GPU, NPU)
  videoBackend: string; // 'cpu' (default), 'gpu_d3d12'
  aiBackend: string;    // 'cpu' (default), 'npu', 'gpu'

  // Capa Master de Filtros Cinematográficos / LUTs
  filterLut?: string;
  filterIntensity?: number;

  // Perfiles de Hardware independientes por cámara
  cameraProfiles?: Record<string, CameraHardwareProfile>;

  // Gestión de presets y favoritos nativos en JSON
  favoritePresets?: string[];
  activePresetId?: string;
  makeupIntensity?: number;
  baseAppearance?: BaseAppearanceState;
}

export interface BaseAppearanceState {
  beautyEnabled?: boolean;
  skinSmoothing: number;
  skinWhitening?: number;
  faceSlim: number;
  eyeEnlarge: number;
  eyeRound?: number;
  eyeBrighten?: number;
  eyeCatchlight?: number;
  eyelashes?: number;
  browLift: number;
  noseSlim: number;
  lipPlump: number;
  foundationTone: string;
  foundationIntensity?: number;
  teethWhitening: number;
  lipstick: number;
  eyeshadow: number;
  contour: number;
  irisLighting?: number;
  irisColor?: string;
}

export interface CameraHardwareProfile {
  width: number;
  height: number;
  fps: number;
  format: string;
  saturation: number;
  contrast: number;
  sharpness: number;
  hue: number;
  whiteBalance: number;
  exposure: number;
  brightness: number;
  colorRange: number;
  colorSpace: number;
}

export interface HardwareOption {
  id: string;
  name: string;
  description: string;
  isRecommended: boolean;
  deviceType: 'CPU' | 'GPU' | 'NPU';
}

export interface SystemHardwareInfo {
  cpuName: string;
  cpuCores: number;
  gpuName: string;
  hasDedicatedGpu: boolean;
  npuName: string;
  hasNpu: boolean;
  videoBackends: HardwareOption[];
  aiBackends: HardwareOption[];
}

export interface EngineTelemetry {
  type: string;
  fps: number;
  frameTimeMs: number;
  aiMs: number;
  gpuMs: number;
  vcamActive: boolean;
  gpuName: string;
  faces: number;

  // Fotometría y Auto-ajustes
  detectedLuma: number;
  autoExposure: number;
  autoWb: number;
  aiLightingActive: boolean;
  aiDenoiseActive: boolean;
  aiAutoTuneActive: boolean;
  recommendedExposure: number;
  recommendedContrast: number;
  recommendedSaturation: number;
  recommendedSharpness: number;
  recommendedWb: number;

  // Audio metering
  audioPeakDb: number;
  audioRmsDb: number;

  // Aceleración de Hardware
  activeVideoBackend?: string;
  activeAiBackend?: string;
  hardware?: SystemHardwareInfo;
}

export interface PresetFaceLight {
  brightness: number;
  contrast: number;
  saturation: number;
  white_balance: number;
  exposure: number;
  foundation_tone: string;
  skin_smoothing: number;
}

export interface PresetMakeup {
  lipstick: number;
  eyeshadow: number;
  contour: number;
  teeth_whitening: number;
  eye_brighten: number;
  eye_catchlight?: number;
  eyelashes?: number;
  lip_plump: number;
  eye_enlarge?: number;
  eye_round?: number;
  iris_lighting?: number;
  iris_color?: string;
}

export interface MakeupPresetConfig {
  id: string;
  name: string;
  category: string;
  default_filter_intensity: number;
  default_makeup_intensity: number;
  face_light: PresetFaceLight;
  makeup: PresetMakeup;
}

export interface MakeupPresetItem {
  id: string;
  name: string;
  category: string;
  avatar: string;
  is_favorite: boolean;
  file?: string;
  isNone?: boolean;
}

