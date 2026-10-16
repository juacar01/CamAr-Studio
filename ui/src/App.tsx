import React, { useState, useEffect } from 'react';
import { StudioHeader } from './components/StudioHeader';
import { VideoCanvas } from './components/VideoCanvas';
import { EffectsSidebar } from './components/EffectsSidebar';
import { SourcePropertiesModal } from './components/SourcePropertiesModal';
import { HardwareSettingsModal } from './components/HardwareSettingsModal';
import { LogsModal } from './components/LogsModal';
import type { EffectState, EngineTelemetry, CameraDeviceInfo, AudioDeviceInfo } from './types';
import { engineClient } from './services/wsClient';

const DEFAULT_EFFECTS: EffectState = {
  selectedCameraId: 'auto',
  selectedWidth: 1920,
  selectedHeight: 1080,
  selectedFps: 30,
  selectedFormat: 'NV12',

  colorRange: 0, // Full
  colorSpace: 0, // Rec709

  saturation: 1.0,
  contrast: 1.0,
  sharpness: 0.2,
  hue: 0.0,
  whiteBalance: 0.0,
  exposure: 0.0,
  brightness: 0.0,

  aiLightingEnabled: false,
  aiDenoiseEnabled: false,
  aiAutoTuneEnabled: false,

  selectedAudioId: 'default',
  audioVolume: 1.0,
  audioMuted: false,

  beautyEnabled: true,
  skinSmoothing: 0.45,
  faceSlim: 0.0,
  eyeEnlarge: 0.0,
  eyeBrighten: 0.0,
  browLift: 0.0,
  noseSlim: 0.0,
  lipPlump: 0.0,
  foundationTone: 'none',
  teethWhitening: 0.0,
  lipstick: 0.0,
  eyeshadow: 0.0,
  contour: 0.0,
  bgMode: 0,
  blurRadius: 16.0,
  edgeFeather: 1.5,
  segThreshold: 0.5,
  faceDetection: true,
  personSegmentation: true,
  drawFaceBox: false,
  drawFaceLandmarks: false,

  videoBackend: 'cpu',
  aiBackend: 'cpu',
};

const getInitialEffects = (): EffectState => {
  try {
    const raw = localStorage.getItem('camar_presets');
    if (raw) {
      const parsed = JSON.parse(raw);
      return { ...DEFAULT_EFFECTS, ...parsed };
    }
  } catch {}
  return DEFAULT_EFFECTS;
};

const DEFAULT_TELEMETRY: EngineTelemetry = {
  type: 'TELEMETRY',
  fps: 60.0,
  frameTimeMs: 16.6,
  aiMs: 1.4,
  gpuMs: 0.9,
  vcamActive: false,
  gpuName: 'NVIDIA GeForce RTX 5060 [Rust SIMD/GPU]',
  faces: 1,
  detectedLuma: 0.42,
  autoExposure: 0.0,
  autoWb: 0.0,
  aiLightingActive: false,
  aiDenoiseActive: false,
  aiAutoTuneActive: false,
  recommendedExposure: 0.0,
  recommendedContrast: 1.08,
  recommendedSaturation: 1.06,
  recommendedSharpness: 0.25,
  recommendedWb: 0.0,
  audioPeakDb: -42.0,
  audioRmsDb: -48.0,
};

export const App: React.FC = () => {
  const [effects, setEffects] = useState<EffectState>(getInitialEffects);
  const [telemetry, setTelemetry] = useState<EngineTelemetry>(DEFAULT_TELEMETRY);
  const [connected, setConnected] = useState<boolean>(false);
  const [isSourceModalOpen, setIsSourceModalOpen] = useState<boolean>(false);
  const [isHardwareModalOpen, setIsHardwareModalOpen] = useState<boolean>(false);
  const [isLogsModalOpen, setIsLogsModalOpen] = useState<boolean>(false);

  // Dispositivos detectados dinámicamente en tiempo real (Cero quemado)
  const [cameras, setCameras] = useState<CameraDeviceInfo[]>([]);
  const [microphones, setMicrophones] = useState<AudioDeviceInfo[]>([]);

  useEffect(() => {
    engineClient.connect(
      (newTelemetry) => {
        setTelemetry(newTelemetry);
      },
      (devices) => {
        setCameras(devices.cameras);
        setMicrophones(devices.microphones);

        // Comprobar si ya existe un preset guardado válido
        const currentSaved = getInitialEffects();
        const matchedCam = devices.cameras.find((c) => c.id === currentSaved.selectedCameraId);

        if (matchedCam && matchedCam.formats.length > 0) {
          // El preset guardado corresponde a una cámara conectada
          const matchedFormat =
            matchedCam.formats.find(
              (f) =>
                f.width === currentSaved.selectedWidth &&
                f.height === currentSaved.selectedHeight &&
                f.fps === currentSaved.selectedFps &&
                f.format === currentSaved.selectedFormat
            ) || matchedCam.formats[0];

          setEffects((prev) => ({
            ...prev,
            selectedCameraId: matchedCam.id,
            selectedWidth: matchedFormat.width,
            selectedHeight: matchedFormat.height,
            selectedFps: matchedFormat.fps,
            selectedFormat: matchedFormat.format,
          }));

          engineClient.selectCamera(
            matchedCam.id,
            matchedFormat.width,
            matchedFormat.height,
            matchedFormat.fps,
            matchedFormat.format
          );
        } else if (devices.cameras.length > 0) {
          // Si no había preset o la cámara no está conectada, elegir la primera física
          const firstCam = devices.cameras.find((c) => c.id !== 'synthetic') || devices.cameras[0];
          if (firstCam && firstCam.formats.length > 0) {
            const bestFormat = firstCam.formats[0];
            setEffects((prev) => ({
              ...prev,
              selectedCameraId: firstCam.id,
              selectedWidth: bestFormat.width,
              selectedHeight: bestFormat.height,
              selectedFps: bestFormat.fps,
              selectedFormat: bestFormat.format,
            }));
            engineClient.selectCamera(
              firstCam.id,
              bestFormat.width,
              bestFormat.height,
              bestFormat.fps,
              bestFormat.format
            );
          }
        }

        if (devices.microphones.length > 0) {
          const defaultMic = devices.microphones.find((m) => m.is_default) || devices.microphones[0];
          setEffects((prev) => ({
            ...prev,
            selectedAudioId: prev.selectedAudioId || defaultMic.id,
          }));
        }
      },
      (isConnected) => {
        setConnected(isConnected);
      },
      (savedPreset) => {
        if (savedPreset) {
          setEffects((prev) => {
            const merged = {
              ...prev,
              selectedCameraId: savedPreset.selected_camera_id || prev.selectedCameraId,
              selectedWidth: savedPreset.selected_width || prev.selectedWidth,
              selectedHeight: savedPreset.selected_height || prev.selectedHeight,
              selectedFps: savedPreset.selected_fps || prev.selectedFps,
              selectedFormat: savedPreset.selected_format || prev.selectedFormat,
              saturation: savedPreset.saturation ?? prev.saturation,
              contrast: savedPreset.contrast ?? prev.contrast,
              sharpness: savedPreset.sharpness ?? prev.sharpness,
              hue: savedPreset.hue ?? prev.hue,
              whiteBalance: savedPreset.white_balance ?? prev.whiteBalance,
              exposure: savedPreset.exposure ?? prev.exposure,
              brightness: savedPreset.brightness ?? prev.brightness,
              aiLightingEnabled: savedPreset.ai_lighting_enabled ?? prev.aiLightingEnabled,
              aiDenoiseEnabled: savedPreset.ai_denoise_enabled ?? prev.aiDenoiseEnabled,
              aiAutoTuneEnabled: savedPreset.ai_auto_tune_enabled ?? prev.aiAutoTuneEnabled,
              videoBackend: savedPreset.video_backend || prev.videoBackend,
              aiBackend: savedPreset.ai_backend || prev.aiBackend,
              selectedAudioId: savedPreset.selected_audio_id || prev.selectedAudioId,
              beautyEnabled: savedPreset.beauty_enabled ?? prev.beautyEnabled,
              skinSmoothing: savedPreset.skin_smoothing ?? prev.skinSmoothing,
              faceSlim: savedPreset.face_slim ?? prev.faceSlim,
              eyeEnlarge: savedPreset.eye_enlarge ?? prev.eyeEnlarge,
              browLift: savedPreset.brow_lift ?? prev.browLift,
              noseSlim: savedPreset.nose_slim ?? prev.noseSlim,
              lipPlump: savedPreset.lip_plump ?? prev.lipPlump,
              foundationTone: savedPreset.foundation_tone || prev.foundationTone,
              teethWhitening: savedPreset.teeth_whitening ?? prev.teethWhitening,
              lipstick: savedPreset.lipstick ?? prev.lipstick,
              eyeshadow: savedPreset.eyeshadow ?? prev.eyeshadow,
              contour: savedPreset.contour ?? prev.contour,
              audioVolume: savedPreset.audio_volume ?? prev.audioVolume,
              audioMuted: savedPreset.audio_muted ?? prev.audioMuted,
            };
            try {
              localStorage.setItem('camar_presets', JSON.stringify(merged));
            } catch {}
            return merged;
          });
        }
      }
    );

    return () => {
      engineClient.disconnect();
    };
  }, []);

  const handleEffectChange = (updated: EffectState) => {
    setEffects(updated);
    try {
      localStorage.setItem('camar_presets', JSON.stringify(updated));
    } catch {}
    engineClient.sendConfigUpdate(updated);
  };

  // Abrir selector de fuente -> mantiene preview en vivo para ajustes en tiempo real
  const handleOpenSourceModal = () => {
    setIsSourceModalOpen(true);
  };

  // Cerrar selector de fuente cancelando
  const handleCloseSourceModal = () => {
    setIsSourceModalOpen(false);
  };

  // Guardar y Conectar Fuente -> aplica parámetros, guarda preset
  const handleSaveAndConnectSource = () => {
    try {
      localStorage.setItem('camar_presets', JSON.stringify(effects));
    } catch {}

    engineClient.selectCamera(
      effects.selectedCameraId,
      effects.selectedWidth,
      effects.selectedHeight,
      effects.selectedFps,
      effects.selectedFormat
    );
    engineClient.sendConfigUpdate(effects);
    setIsSourceModalOpen(false);
  };

  const handleSelectCameraConfig = (
    camId: string,
    w: number,
    h: number,
    fps: number,
    fmt: string
  ) => {
    const updated = {
      ...effects,
      selectedCameraId: camId,
      selectedWidth: w,
      selectedHeight: h,
      selectedFps: fps,
      selectedFormat: fmt,
    };
    setEffects(updated);
    engineClient.selectCamera(camId, w, h, fps, fmt);
    engineClient.sendConfigUpdate(updated);
  };

  const handleSelectAudio = (audioId: string) => {
    const updated = { ...effects, selectedAudioId: audioId };
    setEffects(updated);
    engineClient.selectAudio(audioId);
  };

  const handleToggleMute = () => {
    const updated = { ...effects, audioMuted: !effects.audioMuted };
    setEffects(updated);
    engineClient.toggleMute(updated.audioMuted);
  };

  const handleToggleVcam = () => {
    engineClient.toggleVirtualCam(!telemetry.vcamActive);
  };

  const handleReset = () => {
    setEffects(DEFAULT_EFFECTS);
    try {
      localStorage.removeItem('camar_presets');
    } catch {}
    engineClient.sendConfigUpdate(DEFAULT_EFFECTS);
  };

  return (
    <div className="flex flex-col h-screen w-screen bg-neutral-950 text-neutral-100 overflow-hidden font-sans">
      {/* Top Header con Botón de Selección de Fuente */}
      <StudioHeader
        connected={connected}
        vcamActive={telemetry.vcamActive}
        onToggleVcam={handleToggleVcam}
        onOpenLogsModal={() => setIsLogsModalOpen(true)}
        onOpenHardwareModal={() => setIsHardwareModalOpen(true)}
        onOpenSourceModal={handleOpenSourceModal}
        cameras={cameras}
        selectedCameraId={effects.selectedCameraId}
        selectedWidth={effects.selectedWidth}
        selectedHeight={effects.selectedHeight}
        selectedFps={effects.selectedFps}
        selectedFormat={effects.selectedFormat}
        selectedAudioId={effects.selectedAudioId}
        audioMuted={effects.audioMuted}
        onToggleMute={handleToggleMute}
        audioPeakDb={telemetry.audioPeakDb}
      />

      {/* Main Studio Workspace */}
      <div className="flex flex-1 overflow-hidden">
        {/* Central Live Video Canvas */}
        <VideoCanvas
          effects={effects}
          telemetry={telemetry}
        />

        {/* Right Side Quick Controls */}
        <EffectsSidebar
          effects={effects}
          telemetry={telemetry}
          onChange={handleEffectChange}
          onReset={handleReset}
        />
      </div>

      {/* Modal Dedicado de Propiedades de la Fuente de Video */}
      <SourcePropertiesModal
        isOpen={isSourceModalOpen}
        onClose={handleCloseSourceModal}
        onSaveAndConnect={handleSaveAndConnectSource}
        effects={effects}
        telemetry={telemetry}
        cameras={cameras}
        microphones={microphones}
        onChange={handleEffectChange}
        onSelectCameraConfig={handleSelectCameraConfig}
        onSelectAudio={handleSelectAudio}
        onReset={handleReset}
      />

      {/* Modal de Configuración > Video & Aceleración de Hardware (CPU, GPU, NPU) */}
      <HardwareSettingsModal
        isOpen={isHardwareModalOpen}
        onClose={() => setIsHardwareModalOpen(false)}
        effects={effects}
        telemetry={telemetry}
        onChange={handleEffectChange}
      />

      {/* Modal de Trazabilidad y Logs de Errores */}
      <LogsModal
        isOpen={isLogsModalOpen}
        onClose={() => setIsLogsModalOpen(false)}
      />
    </div>
  );
};

export default App;
