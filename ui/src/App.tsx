import React, { useState, useEffect, useRef } from 'react';
import { StudioHeader } from './components/StudioHeader';
import { VideoCanvas } from './components/VideoCanvas';
import { EffectsSidebar } from './components/EffectsSidebar';
import { SourcePropertiesModal } from './components/SourcePropertiesModal';
import { HardwareSettingsModal } from './components/HardwareSettingsModal';
import { LogsModal } from './components/LogsModal';
import type { EffectState, EngineTelemetry, CameraDeviceInfo, AudioDeviceInfo } from './types';
import { engineClient } from './services/wsClient';

const DEFAULT_EFFECTS: EffectState = {
  selectedCameraId: '',
  selectedWidth: 1920,
  selectedHeight: 1080,
  selectedFps: 60,
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
  skinWhitening: 0.0,
  faceSlim: 0.0,
  eyeEnlarge: 0.0,
  eyeShape: 'auto',
  eyeRound: 0.0,
  eyeBrighten: 0.0,
  eyeCatchlight: 0.0,
  eyelashes: 0.0,
  irisLighting: 0.0,
  irisColor: 'none',
  browLift: 0.0,
  noseSlim: 0.0,
  lipPlump: 0.0,
  foundationTone: 'none',
  foundationIntensity: 0.40,
  teethWhitening: 0.0,
  lipstick: 0.0,
  lipstickColor: 'rose_natural',
  eyeshadow: 0.0,
  eyeshadowColor: 'bronze',
  eyeshadowStyle: 'natural_nude',
  eyebrows: 0.0,
  eyebrowColor: 'dark_brown',
  eyebrowStyle: 'natural_fill',
  contour: 0.0,
  activeEffect: 'none',
  effectIntensity: 1.0,
  bgMode: 0,
  blurRadius: 16.0,
  edgeFeather: 1.5,
  segThreshold: 0.5,
  faceDetection: true,
  personSegmentation: true,
  drawFaceBox: false,
  drawFaceLandmarks: false,

  videoBackend: 'gpu_d3d12',
  aiBackend: 'npu',

  filterLut: 'none',
  filterIntensity: 0.80,
};

const getInitialEffects = (): EffectState => {
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
  const effectsRef = useRef<EffectState>(DEFAULT_EFFECTS);
  const [telemetry, setTelemetry] = useState<EngineTelemetry>(DEFAULT_TELEMETRY);
  const [connected, setConnected] = useState<boolean>(false);
  const [isSourceModalOpen, setIsSourceModalOpen] = useState<boolean>(false);
  const [isHardwareModalOpen, setIsHardwareModalOpen] = useState<boolean>(false);
  const [isLogsModalOpen, setIsLogsModalOpen] = useState<boolean>(false);

  // Dispositivos detectados dinámicamente en tiempo real (Cero quemado)
  const [cameras, setCameras] = useState<CameraDeviceInfo[]>([]);
  const [microphones, setMicrophones] = useState<AudioDeviceInfo[]>([]);
  const latestCamerasRef = useRef<CameraDeviceInfo[]>([]);

  useEffect(() => {
    engineClient.connect(
      (newTelemetry) => {
        setTelemetry(newTelemetry);
      },
      (devices) => {
        setCameras(devices.cameras);
        setMicrophones(devices.microphones);
        latestCamerasRef.current = devices.cameras;

        const currentSaved = effectsRef.current;
        const targetCamId = currentSaved.selectedCameraId;

        // Buscar coincidencia si había cámara guardada
        let matchedCam = targetCamId
          ? devices.cameras.find((c) => c.id === targetCamId)
          : null;

        // Si no había cámara seleccionada aún, preferir la primera cámara física detectada (en lugar del simulador)
        if (!matchedCam && (!targetCamId || targetCamId === 'auto')) {
          matchedCam = devices.cameras.find((c) => c.id !== 'synthetic') || devices.cameras[0];
        }

        let nextEffects = { ...currentSaved };

        if (matchedCam && matchedCam.formats.length > 0) {
          const matchedFormat =
            matchedCam.formats.find(
              (f) =>
                f.width === currentSaved.selectedWidth &&
                f.height === currentSaved.selectedHeight &&
                f.fps === currentSaved.selectedFps &&
                f.format === currentSaved.selectedFormat
            ) ||
            matchedCam.formats.find(
              (f) =>
                f.width === currentSaved.selectedWidth &&
                f.height === currentSaved.selectedHeight &&
                f.fps === currentSaved.selectedFps
            ) ||
            matchedCam.formats.find(
              (f) =>
                f.width === currentSaved.selectedWidth &&
                f.height === currentSaved.selectedHeight
            ) ||
            matchedCam.formats[0];

          nextEffects = {
            ...nextEffects,
            selectedCameraId: matchedCam.id,
            selectedWidth: matchedFormat.width,
            selectedHeight: matchedFormat.height,
            selectedFps: matchedFormat.fps,
            selectedFormat: matchedFormat.format,
          };
        } else if (targetCamId === 'synthetic' || !matchedCam) {
          const syntheticCam = devices.cameras.find((c) => c.id === 'synthetic');
          const simFormat = syntheticCam?.formats[0] || {
            width: 1920,
            height: 1080,
            fps: 60,
            format: 'NV12',
          };

          nextEffects = {
            ...nextEffects,
            selectedCameraId: 'synthetic',
            selectedWidth: currentSaved.selectedWidth || simFormat.width,
            selectedHeight: currentSaved.selectedHeight || simFormat.height,
            selectedFps: currentSaved.selectedFps || simFormat.fps,
            selectedFormat: currentSaved.selectedFormat || simFormat.format,
          };
        }

        if (devices.microphones.length > 0) {
          const defaultMic = devices.microphones.find((m) => m.is_default) || devices.microphones[0];
          nextEffects = {
            ...nextEffects,
            selectedAudioId: nextEffects.selectedAudioId || defaultMic.id,
          };
        }

        effectsRef.current = nextEffects;
        setEffects(nextEffects);
      },
      (isConnected) => {
        setConnected(isConnected);
      },
      (savedPreset) => {
        if (savedPreset) {
          const prev = effectsRef.current;
          const ba = savedPreset.base_appearance;
          const merged: EffectState = {
            ...prev,
            selectedCameraId: savedPreset.selected_camera_id || prev.selectedCameraId,
            selectedWidth: savedPreset.selected_width || prev.selectedWidth,
            selectedHeight: savedPreset.selected_height || prev.selectedHeight,
            selectedFps: savedPreset.selected_fps || prev.selectedFps,
            selectedFormat: savedPreset.selected_format || prev.selectedFormat,
            colorRange: savedPreset.color_range === 'Limited' ? 1 : 0,
            colorSpace: savedPreset.color_space === 'SRgb' ? 1 : savedPreset.color_space === 'Rec601' ? 2 : savedPreset.color_space === 'Rec2020' ? 3 : 0,
            saturation: savedPreset.saturation ?? prev.saturation,
            contrast: savedPreset.contrast ?? prev.contrast,
            sharpness: savedPreset.sharpness ?? prev.sharpness,
            hue: savedPreset.hue ?? prev.hue,
            whiteBalance: savedPreset.white_balance ?? prev.whiteBalance,
            exposure: (savedPreset.exposure !== undefined && Math.abs(savedPreset.exposure) <= 2.0) ? savedPreset.exposure : 0.0,
            brightness: (savedPreset.brightness !== undefined && Math.abs(savedPreset.brightness) <= 1.0) ? savedPreset.brightness : 0.0,
            aiLightingEnabled: savedPreset.ai_lighting_enabled ?? prev.aiLightingEnabled,
            aiDenoiseEnabled: savedPreset.ai_denoise_enabled ?? prev.aiDenoiseEnabled,
            aiAutoTuneEnabled: savedPreset.ai_auto_tune_enabled ?? prev.aiAutoTuneEnabled,
            videoBackend: savedPreset.video_backend || prev.videoBackend,
            aiBackend: savedPreset.ai_backend || prev.aiBackend,
            selectedAudioId: savedPreset.selected_audio_id || prev.selectedAudioId,
            beautyEnabled: savedPreset.beauty_enabled ?? prev.beautyEnabled,
            skinSmoothing: savedPreset.skin_smoothing ?? prev.skinSmoothing,
            skinWhitening: savedPreset.skin_whitening ?? prev.skinWhitening,
            faceSlim: savedPreset.face_slim ?? prev.faceSlim,
            eyeEnlarge: savedPreset.eye_enlarge ?? prev.eyeEnlarge,
            eyeRound: savedPreset.eye_round ?? prev.eyeRound,
            eyeBrighten: savedPreset.eye_brighten ?? prev.eyeBrighten,
            eyeCatchlight: savedPreset.eye_catchlight ?? prev.eyeCatchlight,
            eyelashes: savedPreset.eyelashes ?? prev.eyelashes,
            irisLighting: savedPreset.iris_lighting ?? prev.irisLighting,
            irisColor: savedPreset.iris_color || prev.irisColor,
            browLift: savedPreset.brow_lift ?? prev.browLift,
            noseSlim: savedPreset.nose_slim ?? prev.noseSlim,
            lipPlump: savedPreset.lip_plump ?? prev.lipPlump,
            foundationTone: savedPreset.foundation_tone || prev.foundationTone,
            foundationIntensity: savedPreset.foundation_intensity ?? prev.foundationIntensity,
            teethWhitening: savedPreset.teeth_whitening ?? prev.teethWhitening,
            lipstick: savedPreset.lipstick ?? prev.lipstick,
            lipstickColor: savedPreset.lipstick_color || prev.lipstickColor || 'rose_natural',
            eyeshadow: savedPreset.eyeshadow ?? prev.eyeshadow,
            eyebrows: savedPreset.eyebrows ?? prev.eyebrows,
            contour: savedPreset.contour ?? prev.contour,
            filterLut: savedPreset.filter_lut || prev.filterLut,
            filterIntensity: savedPreset.filter_intensity ?? prev.filterIntensity,
            audioVolume: savedPreset.audio_volume ?? prev.audioVolume,
            audioMuted: savedPreset.audio_muted ?? prev.audioMuted,
            favoritePresets: savedPreset.favorite_presets || prev.favoritePresets,
            activePresetId: savedPreset.active_preset_id || prev.activePresetId,
            makeupIntensity: savedPreset.makeup_intensity ?? prev.makeupIntensity,
            activeEffect: savedPreset.active_effect || prev.activeEffect || 'none',
            effectIntensity: savedPreset.effect_intensity ?? prev.effectIntensity ?? 1.0,
            customPresets: savedPreset.custom_presets || prev.customPresets,
            baseAppearance: {
              beautyEnabled: savedPreset.beauty_enabled ?? true,
              skinSmoothing: ba?.skin_smoothing ?? Math.round((savedPreset.skin_smoothing ?? 0.45) * 100),
              skinWhitening: ba?.skin_whitening ?? Math.round((savedPreset.skin_whitening ?? 0) * 100),
              faceSlim: ba?.face_slim ?? Math.round((savedPreset.face_slim ?? 0) * 100),
              eyeEnlarge: ba?.eye_enlarge ?? Math.round((savedPreset.eye_enlarge ?? 0) * 100),
              eyeRound: ba?.eye_round ?? Math.round((savedPreset.eye_round ?? 0) * 100),
              eyeBrighten: ba?.eye_brighten ?? Math.round((savedPreset.eye_brighten ?? 0) * 100),
              eyeCatchlight: ba?.eye_catchlight ?? Math.round((savedPreset.eye_catchlight ?? 0) * 100),
              eyelashes: ba?.eyelashes ?? Math.round((savedPreset.eyelashes ?? 0) * 100),
              browLift: ba?.brow_lift ?? Math.round((savedPreset.brow_lift ?? 0) * 100),
              noseSlim: ba?.nose_slim ?? Math.round((savedPreset.nose_slim ?? 0) * 50),
              lipPlump: ba?.lip_plump ?? Math.round((savedPreset.lip_plump ?? 0) * 100),
              foundationTone: ba?.foundation_tone || savedPreset.foundation_tone || 'none',
              foundationIntensity: ba?.foundation_intensity ?? Math.round((savedPreset.foundation_intensity ?? 0.40) * 100),
              teethWhitening: ba?.teeth_whitening ?? Math.round((savedPreset.teeth_whitening ?? 0) * 100),
              lipstick: ba?.lipstick ?? Math.round((savedPreset.lipstick ?? 0) * 100),
              eyeShape: ba?.eye_shape || savedPreset.eye_shape || 'auto',
              eyeshadow: ba?.eyeshadow ?? Math.round((savedPreset.eyeshadow ?? 0) * 100),
              eyeshadowColor: ba?.eyeshadow_color || savedPreset.eyeshadow_color || 'bronze',
              eyeshadowStyle: ba?.eyeshadow_style || savedPreset.eyeshadow_style || 'natural_nude',
              eyebrows: ba?.eyebrows ?? Math.round((savedPreset.eyebrows ?? 0) * 100),
              eyebrowColor: ba?.eyebrow_color || savedPreset.eyebrow_color || 'dark_brown',
              eyebrowStyle: ba?.eyebrow_style || savedPreset.eyebrow_style || 'natural_fill',
              contour: ba?.contour ?? Math.round((savedPreset.contour ?? 0) * 100),
              irisLighting: ba?.iris_lighting ?? Math.round((savedPreset.iris_lighting ?? 0) * 100),
              irisColor: ba?.iris_color || savedPreset.iris_color || 'none',
            },
          };
          effectsRef.current = merged;
          setEffects(merged);
        }
      }
    );

    return () => {
      engineClient.disconnect();
    };
  }, []);

  const handleEffectChange = (updated: EffectState) => {
    effectsRef.current = updated;
    setEffects(updated);
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

  // Guardar y Conectar Fuente -> aplica parámetros y notifica al motor
  const handleSaveAndConnectSource = () => {
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
    effectsRef.current = updated;
    setEffects(updated);
    engineClient.selectCamera(camId, w, h, fps, fmt);
    engineClient.sendConfigUpdate(updated);
  };

  const handleSelectAudio = (audioId: string) => {
    const updated = { ...effects, selectedAudioId: audioId };
    effectsRef.current = updated;
    setEffects(updated);
    engineClient.selectAudio(audioId);
  };

  const handleToggleMute = () => {
    const updated = { ...effects, audioMuted: !effects.audioMuted };
    effectsRef.current = updated;
    setEffects(updated);
    engineClient.toggleMute(updated.audioMuted);
  };

  const handleToggleVcam = () => {
    engineClient.toggleVirtualCam(!telemetry.vcamActive);
  };

  const handleReset = () => {
    effectsRef.current = DEFAULT_EFFECTS;
    setEffects(DEFAULT_EFFECTS);
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
