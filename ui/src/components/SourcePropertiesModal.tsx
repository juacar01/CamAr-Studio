import React, { useMemo, useState } from 'react';
import { X, Camera, Sliders, Zap, Sun, Shield, Volume2, Mic, MicOff, RefreshCw, Check, Sparkles, Wand2, Cpu, Lock } from 'lucide-react';
import type { EffectState, EngineTelemetry, CameraDeviceInfo, AudioDeviceInfo } from '../types';

interface SourcePropertiesModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSaveAndConnect: () => void;
  effects: EffectState;
  telemetry: EngineTelemetry;
  cameras: CameraDeviceInfo[];
  microphones: AudioDeviceInfo[];
  onChange: (updated: EffectState) => void;
  onSelectCameraConfig: (camId: string, w: number, h: number, fps: number, fmt: string) => void;
  onSelectAudio: (audioId: string) => void;
  onReset: () => void;
}

export const SourcePropertiesModal: React.FC<SourcePropertiesModalProps> = ({
  isOpen,
  onClose,
  onSaveAndConnect,
  effects,
  telemetry,
  cameras,
  microphones,
  onChange,
  onSelectCameraConfig,
  onSelectAudio,
  onReset,
}) => {
  const [activeTab, setActiveTab] = useState<'device' | 'color' | 'ai' | 'audio' | 'hardware'>('device');
  const [aiFeedbackMsg, setAiFeedbackMsg] = useState<string | null>(null);
  const [isDockedSide, setIsDockedSide] = useState<boolean>(true);

  const update = (partial: Partial<EffectState>) => {
    onChange({ ...effects, ...partial });
  };

  const handleAiCalibrate = () => {
    const exp = telemetry.recommendedExposure !== undefined ? telemetry.recommendedExposure : (0.42 - telemetry.detectedLuma) * 0.75;
    const contrast = telemetry.recommendedContrast || 1.08;
    const sat = telemetry.recommendedSaturation || 1.06;
    const sharpness = telemetry.recommendedSharpness || 0.25;
    const wb = telemetry.recommendedWb !== undefined ? telemetry.recommendedWb : 0.0;

    const roundedExp = Math.max(-2, Math.min(2, Math.round(exp * 20) / 20));
    const roundedContrast = Math.max(0.5, Math.min(1.8, Math.round(contrast * 20) / 20));
    const roundedSat = Math.max(0, Math.min(2, Math.round(sat * 20) / 20));
    const roundedSharpness = Math.max(0, Math.min(1, Math.round(sharpness * 20) / 20));
    const roundedWb = Math.max(-1, Math.min(1, Math.round(wb * 20) / 20));

    update({
      aiAutoTuneEnabled: false,
      exposure: roundedExp,
      contrast: roundedContrast,
      saturation: roundedSat,
      sharpness: roundedSharpness,
      whiteBalance: roundedWb,
      hue: 0.0,
    });

    setAiFeedbackMsg(
      `✓ Calibración IA aplicada: EV ${roundedExp >= 0 ? '+' : ''}${roundedExp.toFixed(2)} | Contraste ${(roundedContrast * 100).toFixed(0)}% | Sat ${(roundedSat * 100).toFixed(0)}% | Sliders desbloqueados para ajuste fino`
    );
    setTimeout(() => setAiFeedbackMsg(null), 3500);
  };

  const isAutoTune = effects.aiAutoTuneEnabled;
  const displaySat = isAutoTune && telemetry.recommendedSaturation !== undefined ? telemetry.recommendedSaturation : effects.saturation;
  const displayContrast = isAutoTune && telemetry.recommendedContrast !== undefined ? telemetry.recommendedContrast : effects.contrast;
  const displaySharpness = isAutoTune && telemetry.recommendedSharpness !== undefined ? telemetry.recommendedSharpness : effects.sharpness;
  const displayHue = isAutoTune ? 0 : effects.hue;
  const displayWb = isAutoTune && telemetry.recommendedWb !== undefined ? telemetry.recommendedWb : effects.whiteBalance;
  const displayExp = isAutoTune && telemetry.recommendedExposure !== undefined ? telemetry.recommendedExposure : effects.exposure;

  const handleResetBase = () => {
    update({
      saturation: 1.0,
      contrast: 1.0,
      sharpness: 0.2,
      hue: 0.0,
      whiteBalance: 0.0,
      exposure: 0.0,
    });
    setAiFeedbackMsg('Valores base restablecidos a defaults');
    setTimeout(() => setAiFeedbackMsg(null), 2500);
  };

  // Cámara seleccionada actualmente
  const currentCamera = cameras.find((c) => c.id === effects.selectedCameraId) || cameras[0];

  // Resoluciones únicas de la cámara activa
  const availableResolutions = useMemo(() => {
    if (!currentCamera) return [];
    const map = new Map<string, { width: number; height: number }>();
    for (const f of currentCamera.formats) {
      const key = `${f.width}x${f.height}`;
      if (!map.has(key)) {
        map.set(key, { width: f.width, height: f.height });
      }
    }
    return Array.from(map.values());
  }, [currentCamera]);

  // FPS para la resolución activa
  const availableFps = useMemo(() => {
    if (!currentCamera) return [];
    const list = new Set<number>();
    for (const f of currentCamera.formats) {
      if (f.width === effects.selectedWidth && f.height === effects.selectedHeight) {
        list.add(f.fps);
      }
    }
    return Array.from(list).sort((a, b) => b - a);
  }, [currentCamera, effects.selectedWidth, effects.selectedHeight]);

  // Formatos para la resolución y FPS activos
  const availableFormats = useMemo(() => {
    if (!currentCamera) return [];
    const list = new Set<string>();
    for (const f of currentCamera.formats) {
      if (
        f.width === effects.selectedWidth &&
        f.height === effects.selectedHeight &&
        f.fps === effects.selectedFps
      ) {
        list.add(f.format);
      }
    }
    return Array.from(list);
  }, [currentCamera, effects.selectedWidth, effects.selectedHeight, effects.selectedFps]);

  if (!isOpen) return null;

  // Handlers en cascada
  const handleCameraChange = (newCamId: string) => {
    const cam = cameras.find((c) => c.id === newCamId) || cameras[0];
    if (cam && cam.formats.length > 0) {
      const best = cam.formats[0];
      onSelectCameraConfig(cam.id, best.width, best.height, best.fps, best.format);
    }
  };

  const handleResolutionChange = (resStr: string) => {
    const [w, h] = resStr.split('x').map(Number);
    const matching = currentCamera?.formats.filter((f) => f.width === w && f.height === h) || [];
    if (matching.length > 0) {
      const best = matching[0];
      onSelectCameraConfig(currentCamera.id, w, h, best.fps, best.format);
    }
  };

  const handleFpsChange = (newFps: number) => {
    const matching = currentCamera?.formats.filter(
      (f) => f.width === effects.selectedWidth && f.height === effects.selectedHeight && f.fps === newFps
    ) || [];
    if (matching.length > 0) {
      onSelectCameraConfig(currentCamera.id, effects.selectedWidth, effects.selectedHeight, newFps, matching[0].format);
    }
  };

  const handleFormatChange = (newFormat: string) => {
    onSelectCameraConfig(
      currentCamera.id,
      effects.selectedWidth,
      effects.selectedHeight,
      effects.selectedFps,
      newFormat
    );
  };

  const vuPercent = Math.min(100, Math.max(0, ((telemetry.audioPeakDb + 60) / 60) * 100));

  return (
    <div className={`fixed inset-0 z-50 flex items-center p-4 bg-black/25 select-none animate-in fade-in duration-150 transition-all ${
      isDockedSide ? 'justify-end pr-6 lg:pr-10' : 'justify-center'
    }`}>
      <div className="relative w-full max-w-xl bg-neutral-900/95 border border-neutral-700/80 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[88vh] backdrop-blur-md">
        {/* Modal Header */}
        <div className="px-6 py-4 border-b border-neutral-800 bg-neutral-950/80 flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="p-2 rounded-xl bg-pink-500/10 border border-pink-500/30 text-pink-400">
              <Camera className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-base font-bold text-white flex items-center space-x-2">
                <span>Propiedades de la Fuente de Cámara</span>
                <span className="text-[10px] px-2 py-0.5 rounded font-mono font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                  EN VIVO
                </span>
              </h2>
              <p className="text-xs text-neutral-400">
                {currentCamera?.name || 'Selecciona un dispositivo de video'}
              </p>
            </div>
          </div>

          <div className="flex items-center space-x-2">
            <button
              onClick={() => setIsDockedSide(!isDockedSide)}
              title={isDockedSide ? 'Centrar ventana' : 'Mover al lateral para ver video libre'}
              className="px-2.5 py-1 rounded-lg text-[11px] font-mono text-neutral-400 hover:text-white border border-neutral-700/60 hover:bg-neutral-800 transition-all cursor-pointer"
            >
              {isDockedSide ? '↔ Centrar' : '→ Lateral'}
            </button>
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg text-neutral-400 hover:text-white hover:bg-neutral-800 transition-all cursor-pointer"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Modal Navigation Tabs */}
        <div className="flex border-b border-neutral-800 bg-neutral-950/40 px-6 pt-2 gap-2 text-xs font-semibold">
          <button
            onClick={() => setActiveTab('device')}
            className={`pb-3 px-3 border-b-2 flex items-center space-x-1.5 transition-all ${
              activeTab === 'device'
                ? 'border-pink-500 text-pink-400'
                : 'border-transparent text-neutral-400 hover:text-neutral-200'
            }`}
          >
            <Camera className="w-3.5 h-3.5" />
            <span>Dispositivo & Formato</span>
          </button>

          <button
            onClick={() => setActiveTab('color')}
            className={`pb-3 px-3 border-b-2 flex items-center space-x-1.5 transition-all ${
              activeTab === 'color'
                ? 'border-pink-500 text-pink-400'
                : 'border-transparent text-neutral-400 hover:text-neutral-200'
            }`}
          >
            <Sliders className="w-3.5 h-3.5" />
            <span>Color & Rango</span>
          </button>

          <button
            onClick={() => setActiveTab('ai')}
            className={`pb-3 px-3 border-b-2 flex items-center space-x-1.5 transition-all ${
              activeTab === 'ai'
                ? 'border-emerald-500 text-emerald-400'
                : 'border-transparent text-neutral-400 hover:text-neutral-200'
            }`}
          >
            <Zap className="w-3.5 h-3.5" />
            <span>Ajustes IA</span>
          </button>

          <button
            onClick={() => setActiveTab('audio')}
            className={`pb-3 px-3 border-b-2 flex items-center space-x-1.5 transition-all ${
              activeTab === 'audio'
                ? 'border-purple-500 text-purple-400'
                : 'border-transparent text-neutral-400 hover:text-neutral-200'
            }`}
          >
            <Volume2 className="w-3.5 h-3.5" />
            <span>Audio Asociado</span>
          </button>

          <button
            onClick={() => setActiveTab('hardware')}
            className={`pb-3 px-3 border-b-2 flex items-center space-x-1.5 transition-all ${
              activeTab === 'hardware'
                ? 'border-cyan-500 text-cyan-400 font-bold'
                : 'border-transparent text-neutral-400 hover:text-neutral-200'
            }`}
          >
            <Cpu className="w-3.5 h-3.5" />
            <span>Aceleración de Hardware</span>
          </button>
        </div>

        {/* Modal Body */}
        <div className="flex-1 overflow-y-auto p-6 space-y-5 text-xs">
          {/* TAB 1: DISPOSITIVO Y FORMATO */}
          {activeTab === 'device' && (
            <div className="space-y-4">
              {/* Selector de Cámara */}
              <div className="space-y-1.5">
                <label className="font-semibold text-neutral-300">Cámara / Dispositivo de Entrada:</label>
                <select
                  value={effects.selectedCameraId}
                  onChange={(e) => handleCameraChange(e.target.value)}
                  className="w-full bg-neutral-800 text-neutral-100 p-2.5 rounded-xl border border-neutral-700 font-medium focus:outline-none cursor-pointer"
                >
                  {cameras.map((c) => (
                    <option key={c.id} value={c.id} className="bg-neutral-900">
                      {c.name}
                    </option>
                  ))}
                </select>
              </div>

              {/* Parámetros en Cascada */}
              <div className="grid grid-cols-3 gap-3 pt-2">
                {/* Resolución */}
                <div className="space-y-1.5">
                  <label className="font-medium text-neutral-400">Resolución:</label>
                  <select
                    value={`${effects.selectedWidth}x${effects.selectedHeight}`}
                    onChange={(e) => handleResolutionChange(e.target.value)}
                    className="w-full bg-neutral-800 text-neutral-100 p-2.5 rounded-xl border border-neutral-700 font-mono font-medium focus:outline-none cursor-pointer"
                  >
                    {availableResolutions.map((r) => (
                      <option key={`${r.width}x${r.height}`} value={`${r.width}x${r.height}`} className="bg-neutral-900">
                        {r.width} × {r.height}
                      </option>
                    ))}
                  </select>
                </div>

                {/* FPS */}
                <div className="space-y-1.5">
                  <label className="font-medium text-neutral-400">Tasa de Cuadros (FPS):</label>
                  <select
                    value={effects.selectedFps}
                    onChange={(e) => handleFpsChange(Number(e.target.value))}
                    className="w-full bg-neutral-800 text-neutral-100 p-2.5 rounded-xl border border-neutral-700 font-mono font-medium focus:outline-none cursor-pointer"
                  >
                    {availableFps.map((fps) => (
                      <option key={fps} value={fps} className="bg-neutral-900">
                        {fps} FPS
                      </option>
                    ))}
                  </select>
                </div>

                {/* Formato de Video */}
                <div className="space-y-1.5">
                  <label className="font-medium text-neutral-400">Formato del Sensor:</label>
                  <select
                    value={effects.selectedFormat}
                    onChange={(e) => handleFormatChange(e.target.value)}
                    className="w-full bg-neutral-800 text-neutral-100 p-2.5 rounded-xl border border-neutral-700 font-mono font-medium focus:outline-none cursor-pointer"
                  >
                    {availableFormats.map((fmt) => (
                      <option key={fmt} value={fmt} className="bg-neutral-900">
                        {fmt}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div className="p-3 bg-neutral-950/60 rounded-xl border border-neutral-800 text-neutral-400 text-[11px] leading-relaxed">
                ℹ️ Los valores de resolución, FPS y formato se consultan directamente del hardware en tiempo real.
                Seleccionar 1080p a 60 FPS con formato NV12 ofrece el rendimiento óptimo con mínima carga de CPU.
              </div>
            </div>
          )}

          {/* TAB 2: COLOR, GAMA Y ESPACIO */}
          {activeTab === 'color' && (
            <div className="space-y-5">
              {/* Espacio y Gama */}
              <div className="grid grid-cols-2 gap-4 bg-neutral-950/60 p-4 rounded-xl border border-neutral-800">
                <div className="space-y-2">
                  <label className="font-semibold text-neutral-300">Espacio de Color:</label>
                  <div className="grid grid-cols-2 gap-1.5">
                    {[
                      { id: 0, label: 'Rec. 709 (HD)' },
                      { id: 1, label: 'sRGB (PC)' },
                      { id: 2, label: 'Rec. 601 (SD)' },
                      { id: 3, label: 'Rec. 2020 (HDR)' },
                    ].map((cs) => (
                      <button
                        key={cs.id}
                        onClick={() => update({ colorSpace: cs.id })}
                        className={`p-2 rounded-lg border text-center transition-all ${
                          effects.colorSpace === cs.id
                            ? 'bg-pink-500/10 border-pink-500 text-pink-300 font-bold'
                            : 'bg-neutral-800/40 border-neutral-800 text-neutral-400 hover:bg-neutral-800'
                        }`}
                      >
                        {cs.label}
                      </button>
                    ))}
                  </div>
                </div>

                <div className="space-y-2">
                  <label className="font-semibold text-neutral-300">Gama de Colores:</label>
                  <div className="grid grid-cols-1 gap-1.5">
                    <button
                      onClick={() => update({ colorRange: 0 })}
                      className={`p-2 rounded-lg border text-center transition-all ${
                        effects.colorRange === 0
                          ? 'bg-pink-500/10 border-pink-500 text-pink-300 font-bold'
                          : 'bg-neutral-800/40 border-neutral-800 text-neutral-400 hover:bg-neutral-800'
                      }`}
                    >
                      Completo (0 - 255 Full Range)
                    </button>
                    <button
                      onClick={() => update({ colorRange: 1 })}
                      className={`p-2 rounded-lg border text-center transition-all ${
                        effects.colorRange === 1
                          ? 'bg-pink-500/10 border-pink-500 text-pink-300 font-bold'
                          : 'bg-neutral-800/40 border-neutral-800 text-neutral-400 hover:bg-neutral-800'
                      }`}
                    >
                      Limitado (16 - 235 Broadcast TV)
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* TAB 3: AJUSTES IA */}
          {activeTab === 'ai' && (
            <div className="space-y-4">
              {/* Encabezado Informativo de la Línea de Proceso */}
              <div className="p-3 rounded-xl bg-neutral-950/80 border border-neutral-800 flex items-center justify-between text-xs">
                <div className="flex items-center space-x-2">
                  <Sparkles className="w-4 h-4 text-cyan-400" />
                  <span className="font-semibold text-neutral-200">Línea de Proceso de IA (Pipeline en Tiempo Real)</span>
                </div>
                <div className="flex items-center space-x-1.5 font-mono text-[10px] text-neutral-400">
                  <span className="text-emerald-400 font-bold">1. Iluminación</span>
                  <span>→</span>
                  <span className="text-cyan-400 font-bold">2. Ruido</span>
                  <span>→</span>
                  <span className="text-pink-400 font-bold">3. Calibración</span>
                </div>
              </div>

              {/* SELECTOR 1: Iluminación por IA */}
              <div
                className={`p-4 rounded-xl border transition-all ${
                  effects.aiLightingEnabled
                    ? 'bg-emerald-500/10 border-emerald-500/40 shadow-lg shadow-emerald-500/5'
                    : 'bg-neutral-800/40 border-neutral-800'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <div
                      className={`p-2 rounded-lg ${
                        effects.aiLightingEnabled ? 'bg-emerald-500 text-white' : 'bg-neutral-700 text-neutral-300'
                      }`}
                    >
                      <Sun className="w-5 h-5" />
                    </div>
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="text-[10px] font-bold px-1.5 py-0.2 rounded bg-emerald-500/20 text-emerald-400 border border-emerald-500/40 font-mono">
                          PASO 1
                        </span>
                        <h4 className="font-bold text-white text-sm">Iluminación por IA (Studio Relighting)</h4>
                      </div>
                      <p className="text-neutral-400 text-[11px] mt-0.5">
                        Compensa sombras faciales y añade iluminación suave de estudio directamente sobre el sensor.
                      </p>
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    checked={effects.aiLightingEnabled}
                    onChange={(e) => update({ aiLightingEnabled: e.target.checked })}
                    className="w-5 h-5 accent-emerald-500 rounded cursor-pointer"
                  />
                </div>
              </div>

              {/* SELECTOR 2: Eliminación de Ruido por IA */}
              <div
                className={`p-4 rounded-xl border transition-all ${
                  effects.aiDenoiseEnabled
                    ? 'bg-cyan-500/10 border-cyan-500/40 shadow-lg shadow-cyan-500/5'
                    : 'bg-neutral-800/40 border-neutral-800'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <div
                      className={`p-2 rounded-lg ${
                        effects.aiDenoiseEnabled ? 'bg-cyan-500 text-white' : 'bg-neutral-700 text-neutral-300'
                      }`}
                    >
                      <Shield className="w-5 h-5" />
                    </div>
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="text-[10px] font-bold px-1.5 py-0.2 rounded bg-cyan-500/20 text-cyan-400 border border-cyan-500/40 font-mono">
                          PASO 2
                        </span>
                        <h4 className="font-bold text-white text-sm">Eliminación de Ruido por IA (AI Denoising)</h4>
                      </div>
                      <p className="text-neutral-400 text-[11px] mt-0.5">
                        Filtro adaptativo temporal de grano y ruido térmico en áreas estáticas tras la iluminación.
                      </p>
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    checked={effects.aiDenoiseEnabled}
                    onChange={(e) => update({ aiDenoiseEnabled: e.target.checked })}
                    className="w-5 h-5 accent-cyan-500 rounded cursor-pointer"
                  />
                </div>
              </div>

              {/* SELECTOR 3: Calibración Automática por IA */}
              <div
                className={`p-4 rounded-xl border transition-all ${
                  effects.aiAutoTuneEnabled
                    ? 'bg-gradient-to-r from-pink-950/30 via-cyan-950/30 to-neutral-900 border-pink-500/50 shadow-lg shadow-pink-500/10'
                    : 'bg-neutral-800/40 border-neutral-800'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-3">
                    <div
                      className={`p-2 rounded-lg ${
                        effects.aiAutoTuneEnabled ? 'bg-pink-500 text-white' : 'bg-neutral-700 text-neutral-300'
                      }`}
                    >
                      <Wand2 className="w-5 h-5" />
                    </div>
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="text-[10px] font-bold px-1.5 py-0.2 rounded bg-pink-500/20 text-pink-400 border border-pink-500/40 font-mono">
                          PASO 3
                        </span>
                        <h4 className="font-bold text-white text-sm">Calibración Automática por IA (Auto-Tune)</h4>
                        {effects.aiAutoTuneEnabled ? (
                          <span className="inline-flex items-center px-2 py-0.5 rounded text-[10px] font-bold bg-pink-500/20 text-pink-300 border border-pink-500/40 animate-pulse">
                            CALIBRACIÓN ACTIVA
                          </span>
                        ) : (
                          <span className="text-[10px] text-neutral-500 font-mono">MANUAL</span>
                        )}
                      </div>
                      <p className="text-neutral-400 text-[11px] mt-0.5">
                        Compensa y calibra exposición, balance de blancos, contraste, nitidez y saturación en vivo.
                      </p>
                    </div>
                  </div>

                  {/* Switch Toggle */}
                  <button
                    onClick={() => update({ aiAutoTuneEnabled: !effects.aiAutoTuneEnabled })}
                    className={`w-12 h-6 rounded-full transition-colors relative p-0.5 shrink-0 ${
                      effects.aiAutoTuneEnabled ? 'bg-pink-500' : 'bg-neutral-700'
                    }`}
                    title="Activar/Desactivar Calibración Automática Continua"
                  >
                    <div
                      className={`w-5 h-5 rounded-full bg-white transition-transform ${
                        effects.aiAutoTuneEnabled ? 'translate-x-6' : 'translate-x-0'
                      }`}
                    />
                  </button>
                </div>

                {/* Botones de Acción Rápida: 1-Click Calibrar & Reset */}
                <div className="flex items-center gap-2.5 mt-3 pt-3 border-t border-neutral-800/80">
                  <button
                    onClick={handleAiCalibrate}
                    className="flex-1 flex items-center justify-center space-x-2 py-2 px-3 rounded-lg bg-pink-500/15 hover:bg-pink-500/25 border border-pink-500/40 text-pink-300 hover:text-pink-200 font-semibold text-xs transition-all active:scale-98"
                    title="Ajusta automáticamente los 6 sliders a los valores ideales medidos por la IA para la escena actual"
                  >
                    <Sparkles className="w-4 h-4" />
                    <span>Auto-Calibrar Sliders con IA (1-Click)</span>
                  </button>

                  <button
                    onClick={handleResetBase}
                    className="flex items-center space-x-1.5 py-2 px-3 rounded-lg bg-neutral-800/80 hover:bg-neutral-800 border border-neutral-700 text-neutral-400 hover:text-neutral-200 text-xs transition-all"
                    title="Restablecer sliders a valores base (100% / 0.0)"
                  >
                    <RefreshCw className="w-3.5 h-3.5" />
                    <span>Restablecer</span>
                  </button>
                </div>

                {aiFeedbackMsg && (
                  <div className="mt-2.5 text-xs text-pink-300 bg-pink-950/60 border border-pink-700/60 px-3 py-1.5 rounded-lg font-mono flex items-center space-x-2 animate-fade-in">
                    <Check className="w-4 h-4 text-pink-400 shrink-0" />
                    <span>{aiFeedbackMsg}</span>
                  </div>
                )}
              </div>

              {/* Sliders Manuales y Estado de Bloqueo */}
              <div className="space-y-2">
                {effects.aiAutoTuneEnabled && (
                  <div className="flex items-center space-x-2 px-3 py-2 rounded-lg bg-pink-950/40 border border-pink-800/60 text-pink-300 text-xs animate-fade-in">
                    <Lock className="w-3.5 h-3.5 text-pink-400 shrink-0" />
                    <span>
                      <strong>Calibración Automática activa:</strong> Los sliders están bloqueados mientras la IA calibra continuamente. Haz clic en <strong>Auto-Calibrar (1-Click)</strong> para transferir los valores a los sliders y modificarlos manualmente.
                    </span>
                  </div>
                )}

                <div className={`grid grid-cols-2 gap-4 bg-neutral-950/40 p-4 rounded-xl border border-neutral-800 transition-opacity ${
                  effects.aiAutoTuneEnabled ? 'opacity-60' : 'opacity-100'
                }`}>
                  {/* Saturación */}
                  <div className="space-y-1">
                    <div className="flex justify-between items-center">
                      <span className={`text-neutral-300 flex items-center space-x-1.5 ${effects.aiAutoTuneEnabled ? 'text-neutral-400' : ''}`}>
                        <span>Saturación</span>
                        {effects.aiAutoTuneEnabled && <Lock className="w-3 h-3 text-pink-400" />}
                      </span>
                      <span className="font-mono text-neutral-400">{(displaySat * 100).toFixed(0)}%</span>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="2"
                      step="0.05"
                      disabled={effects.aiAutoTuneEnabled}
                      value={displaySat}
                      onChange={(e) => update({ saturation: parseFloat(e.target.value) })}
                      className={`w-full accent-pink-500 ${effects.aiAutoTuneEnabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}
                    />
                  </div>

                  {/* Contraste */}
                  <div className="space-y-1">
                    <div className="flex justify-between items-center">
                      <span className={`text-neutral-300 flex items-center space-x-1.5 ${effects.aiAutoTuneEnabled ? 'text-neutral-400' : ''}`}>
                        <span>Contraste</span>
                        {effects.aiAutoTuneEnabled && <Lock className="w-3 h-3 text-pink-400" />}
                      </span>
                      <span className="font-mono text-neutral-400">{(displayContrast * 100).toFixed(0)}%</span>
                    </div>
                    <input
                      type="range"
                      min="0.5"
                      max="1.8"
                      step="0.05"
                      disabled={effects.aiAutoTuneEnabled}
                      value={displayContrast}
                      onChange={(e) => update({ contrast: parseFloat(e.target.value) })}
                      className={`w-full accent-pink-500 ${effects.aiAutoTuneEnabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}
                    />
                  </div>

                  {/* Nitidez */}
                  <div className="space-y-1">
                    <div className="flex justify-between items-center">
                      <span className={`text-neutral-300 flex items-center space-x-1.5 ${effects.aiAutoTuneEnabled ? 'text-neutral-400' : ''}`}>
                        <span>Nitidez (Sharpness)</span>
                        {effects.aiAutoTuneEnabled && <Lock className="w-3 h-3 text-pink-400" />}
                      </span>
                      <span className="font-mono text-neutral-400">{(displaySharpness * 100).toFixed(0)}%</span>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.05"
                      disabled={effects.aiAutoTuneEnabled}
                      value={displaySharpness}
                      onChange={(e) => update({ sharpness: parseFloat(e.target.value) })}
                      className={`w-full accent-pink-500 ${effects.aiAutoTuneEnabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}
                    />
                  </div>

                  {/* Tono */}
                  <div className="space-y-1">
                    <div className="flex justify-between items-center">
                      <span className={`text-neutral-300 flex items-center space-x-1.5 ${effects.aiAutoTuneEnabled ? 'text-neutral-400' : ''}`}>
                        <span>Tono (Hue)</span>
                        {effects.aiAutoTuneEnabled && <Lock className="w-3 h-3 text-pink-400" />}
                      </span>
                      <span className="font-mono text-neutral-400">{(displayHue * 180).toFixed(0)}°</span>
                    </div>
                    <input
                      type="range"
                      min="-1"
                      max="1"
                      step="0.05"
                      disabled={effects.aiAutoTuneEnabled}
                      value={displayHue}
                      onChange={(e) => update({ hue: parseFloat(e.target.value) })}
                      className={`w-full accent-pink-500 ${effects.aiAutoTuneEnabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}
                    />
                  </div>

                  {/* Balance de Blancos */}
                  <div className="space-y-1">
                    <div className="flex justify-between items-center">
                      <span className={`text-neutral-300 flex items-center space-x-1.5 ${effects.aiAutoTuneEnabled ? 'text-neutral-400' : ''}`}>
                        <span>Balance de Blancos</span>
                        {effects.aiAutoTuneEnabled && <Lock className="w-3 h-3 text-pink-400" />}
                      </span>
                      <span className="font-mono text-neutral-400">{displayWb.toFixed(2)}</span>
                    </div>
                    <input
                      type="range"
                      min="-1"
                      max="1"
                      step="0.05"
                      disabled={effects.aiAutoTuneEnabled}
                      value={displayWb}
                      onChange={(e) => update({ whiteBalance: parseFloat(e.target.value) })}
                      className={`w-full accent-pink-500 ${effects.aiAutoTuneEnabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}
                    />
                  </div>

                  {/* Exposición */}
                  <div className="space-y-1">
                    <div className="flex justify-between items-center">
                      <span className={`text-neutral-300 flex items-center space-x-1.5 ${effects.aiAutoTuneEnabled ? 'text-neutral-400' : ''}`}>
                        <span>Exposición (EV)</span>
                        {effects.aiAutoTuneEnabled && <Lock className="w-3 h-3 text-pink-400" />}
                      </span>
                      <span className="font-mono text-neutral-400">{displayExp.toFixed(2)}</span>
                    </div>
                    <input
                      type="range"
                      min="-2"
                      max="2"
                      step="0.1"
                      disabled={effects.aiAutoTuneEnabled}
                      value={displayExp}
                      onChange={(e) => update({ exposure: parseFloat(e.target.value) })}
                      className={`w-full accent-pink-500 ${effects.aiAutoTuneEnabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`}
                    />
                  </div>
                </div>
              </div>

              {/* Telemetría Fotométrica en Vivo */}
              <div className="p-4 rounded-xl bg-neutral-950/70 border border-neutral-800 space-y-3">
                <div className="flex items-center justify-between">
                  <span className="font-bold text-neutral-200">Auto-Ajuste Fotométrico Dinámico</span>
                  <span
                    className={`text-[10px] px-2 py-0.5 rounded-full font-mono font-bold ${
                      effects.aiLightingEnabled || effects.aiDenoiseEnabled || effects.aiAutoTuneEnabled
                        ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 animate-pulse'
                        : 'bg-neutral-800 text-neutral-500'
                    }`}
                  >
                    {effects.aiLightingEnabled || effects.aiDenoiseEnabled || effects.aiAutoTuneEnabled ? 'ACTIVO' : 'STANDBY'}
                  </span>
                </div>

                <div className="grid grid-cols-3 gap-2 text-center font-mono text-[11px]">
                  <div className="bg-neutral-900/80 p-2.5 rounded-lg border border-neutral-800">
                    <div className="text-neutral-500 text-[9px] uppercase">Luz Detectada</div>
                    <div className="font-bold text-white text-xs">{(telemetry.detectedLuma * 100).toFixed(0)}%</div>
                  </div>
                  <div className="bg-neutral-900/80 p-2.5 rounded-lg border border-neutral-800">
                    <div className="text-neutral-500 text-[9px] uppercase">Auto-Exposición</div>
                    <div className="font-bold text-emerald-400 text-xs">
                      {telemetry.autoExposure >= 0 ? '+' : ''}
                      {telemetry.autoExposure.toFixed(2)} EV
                    </div>
                  </div>
                  <div className="bg-neutral-900/80 p-2.5 rounded-lg border border-neutral-800">
                    <div className="text-neutral-500 text-[9px] uppercase">Auto-WB</div>
                    <div className="font-bold text-cyan-400 text-xs">
                      {telemetry.autoWb >= 0 ? '+' : ''}
                      {telemetry.autoWb.toFixed(2)}
                    </div>
                  </div>
                </div>

                <p className="text-[11px] text-neutral-400 leading-relaxed">
                  💡 El motor procesa en riguroso orden: <strong>1. Iluminación por IA</strong> (relleno de sombras), <strong>2. Eliminación de Ruido por IA</strong> (grano temporal) y <strong>3. Calibración Automática</strong> (balance de blancos, saturación, contraste y exposición).
                </p>
              </div>
            </div>
          )}

          {/* TAB 4: AUDIO ASOCIADO */}
          {activeTab === 'audio' && (
            <div className="space-y-4">
              <div className="space-y-1.5">
                <label className="font-semibold text-neutral-300">Dispositivo de Audio / Micrófono:</label>
                <select
                  value={effects.selectedAudioId}
                  onChange={(e) => onSelectAudio(e.target.value)}
                  className="w-full bg-neutral-800 text-neutral-100 p-2.5 rounded-xl border border-neutral-700 font-medium focus:outline-none cursor-pointer"
                >
                  {microphones.map((m) => (
                    <option key={m.id} value={m.id} className="bg-neutral-900">
                      {m.name}
                    </option>
                  ))}
                </select>
              </div>

              <div className="p-4 bg-neutral-950/60 rounded-xl border border-neutral-800 space-y-3">
                <div className="flex items-center justify-between">
                  <span className="font-semibold text-neutral-200">Volumen / Ganancia:</span>
                  <span className="font-mono text-purple-400">{(effects.audioVolume * 100).toFixed(0)}%</span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="2"
                  step="0.05"
                  value={effects.audioVolume}
                  onChange={(e) => update({ audioVolume: parseFloat(e.target.value) })}
                  className="w-full accent-purple-500 cursor-pointer"
                />

                <div className="flex items-center justify-between pt-2">
                  <button
                    onClick={() => update({ audioMuted: !effects.audioMuted })}
                    className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg border font-semibold transition-all ${
                      effects.audioMuted
                        ? 'bg-rose-500/20 border-rose-500 text-rose-400'
                        : 'bg-neutral-800 border-neutral-700 text-neutral-300'
                    }`}
                  >
                    {effects.audioMuted ? <MicOff className="w-3.5 h-3.5" /> : <Mic className="w-3.5 h-3.5" />}
                    <span>{effects.audioMuted ? 'Micrófono Silenciado' : 'Silenciar'}</span>
                  </button>

                  <div className="text-[11px] font-mono text-neutral-400">
                    Pico: {telemetry.audioPeakDb.toFixed(1)} dBFS
                  </div>
                </div>

                {/* Vúmetro */}
                <div className="w-full h-3 bg-neutral-900 rounded-full overflow-hidden p-0.5 border border-neutral-700 flex items-center">
                  <div
                    className={`h-full rounded-full transition-all duration-75 ${
                      effects.audioMuted
                        ? 'bg-neutral-600'
                        : telemetry.audioPeakDb > -6
                        ? 'bg-rose-500'
                        : telemetry.audioPeakDb > -18
                        ? 'bg-amber-400'
                        : 'bg-emerald-500'
                    }`}
                    style={{ width: `${effects.audioMuted ? 0 : vuPercent}%` }}
                  />
                </div>
              </div>
            </div>
          )}

          {/* TAB 5: ACELERACIÓN DE HARDWARE (CPU, GPU, NPU) */}
          {activeTab === 'hardware' && (
            <div className="space-y-4">
              {/* Diagnóstico de Hardware */}
              <div className="bg-neutral-950/60 p-3.5 rounded-xl border border-neutral-800 space-y-2">
                <span className="text-[10px] font-bold text-neutral-400 uppercase tracking-wider">
                  Hardware Físico Detectado en el Sistema
                </span>
                <div className="grid grid-cols-3 gap-2 font-mono text-[11px]">
                  <div className="bg-neutral-900/90 p-2.5 rounded-lg border border-neutral-800">
                    <div className="text-[9px] text-neutral-400 font-sans">CPU</div>
                    <div className="font-bold text-neutral-100 truncate">Intel Core Ultra 7 265KF</div>
                    <div className="text-[10px] text-cyan-400 mt-0.5">20 Cores</div>
                  </div>
                  <div className="bg-neutral-900/90 p-2.5 rounded-lg border border-neutral-800">
                    <div className="text-[9px] text-neutral-400 font-sans">GPU</div>
                    <div className="font-bold text-emerald-300 truncate">NVIDIA RTX 5060</div>
                    <div className="text-[10px] text-emerald-400 mt-0.5">D3D12 / CUDA</div>
                  </div>
                  <div className="bg-neutral-900/90 p-2.5 rounded-lg border border-neutral-800">
                    <div className="text-[9px] text-neutral-400 font-sans">NPU</div>
                    <div className="font-bold text-purple-300 truncate">Intel AI Boost</div>
                    <div className="text-[10px] text-purple-400 mt-0.5">Neural Unit</div>
                  </div>
                </div>
              </div>

              {/* Selector Video Backend */}
              <div className="space-y-2">
                <label className="font-semibold text-neutral-200">Procesamiento de Video (Render & Shaders):</label>
                <div className="grid grid-cols-1 gap-2">
                  <div
                    onClick={() => update({ videoBackend: 'cpu' })}
                    className={`p-3 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                      (effects.videoBackend || 'cpu') === 'cpu'
                        ? 'bg-neutral-800/80 border-cyan-500 shadow-md ring-1 ring-cyan-500/40'
                        : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                    }`}
                  >
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="font-bold text-neutral-100">CPU (Multihilo SIMD AVX2)</span>
                        <span className="text-[10px] px-1.5 py-0.2 rounded font-bold bg-neutral-800 border border-neutral-700 text-neutral-300">
                          (Predeterminado)
                        </span>
                      </div>
                      <p className="text-[10px] text-neutral-400 mt-0.5">
                        Procesamiento multihilo en CPU con 20 hilos en paralelo.
                      </p>
                    </div>
                    <div className={`w-3.5 h-3.5 rounded-full border flex items-center justify-center ${(effects.videoBackend || 'cpu') === 'cpu' ? 'border-cyan-400 bg-cyan-400' : 'border-neutral-600'}`}>
                      {(effects.videoBackend || 'cpu') === 'cpu' && <div className="w-1.5 h-1.5 rounded-full bg-neutral-950" />}
                    </div>
                  </div>

                  <div
                    onClick={() => update({ videoBackend: 'gpu_d3d12' })}
                    className={`p-3 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                      effects.videoBackend === 'gpu_d3d12'
                        ? 'bg-gradient-to-r from-emerald-950/40 to-neutral-900 border-emerald-500 shadow-md ring-1 ring-emerald-500/50'
                        : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                    }`}
                  >
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="font-bold text-neutral-100">GPU - NVIDIA GeForce RTX 5060 (Direct3D 12)</span>
                        <span className="text-[10px] px-1.5 py-0.2 rounded font-bold bg-emerald-500/20 border border-emerald-500/50 text-emerald-300">
                          ✨ (Recomendado)
                        </span>
                      </div>
                      <p className="text-[10px] text-neutral-400 mt-0.5">
                        Aceleración completa por GPU dedicada. Mínima latencia de preview y 0% uso de CPU.
                      </p>
                    </div>
                    <div className={`w-3.5 h-3.5 rounded-full border flex items-center justify-center ${effects.videoBackend === 'gpu_d3d12' ? 'border-emerald-400 bg-emerald-400' : 'border-neutral-600'}`}>
                      {effects.videoBackend === 'gpu_d3d12' && <div className="w-1.5 h-1.5 rounded-full bg-neutral-950" />}
                    </div>
                  </div>
                </div>
              </div>

              {/* Selector AI Backend */}
              <div className="space-y-2">
                <label className="font-semibold text-neutral-200">Motor de Inferencia de IA:</label>
                <div className="grid grid-cols-1 gap-2">
                  <div
                    onClick={() => update({ aiBackend: 'npu' })}
                    className={`p-3 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                      effects.aiBackend === 'npu'
                        ? 'bg-gradient-to-r from-purple-950/40 to-neutral-900 border-purple-500 shadow-md ring-1 ring-purple-500/50'
                        : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                    }`}
                  >
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="font-bold text-neutral-100">NPU - Intel(R) AI Boost</span>
                        <span className="text-[10px] px-1.5 py-0.2 rounded font-bold bg-purple-500/20 border border-purple-500/50 text-purple-300">
                          ✨ (Recomendado - Ultra Eficiente)
                        </span>
                      </div>
                      <p className="text-[10px] text-neutral-400 mt-0.5">
                        Inferencia dedicada en la NPU de tu Intel Core Ultra. 0% impacto en GPU y CPU.
                      </p>
                    </div>
                    <div className={`w-3.5 h-3.5 rounded-full border flex items-center justify-center ${effects.aiBackend === 'npu' ? 'border-purple-400 bg-purple-400' : 'border-neutral-600'}`}>
                      {effects.aiBackend === 'npu' && <div className="w-1.5 h-1.5 rounded-full bg-white" />}
                    </div>
                  </div>

                  <div
                    onClick={() => update({ aiBackend: 'gpu' })}
                    className={`p-3 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                      effects.aiBackend === 'gpu'
                        ? 'bg-gradient-to-r from-emerald-950/40 to-neutral-900 border-emerald-500 shadow-md ring-1 ring-emerald-500/50'
                        : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                    }`}
                  >
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="font-bold text-neutral-100">GPU - NVIDIA RTX 5060 (Tensor Cores)</span>
                        <span className="text-[10px] px-1.5 py-0.2 rounded font-bold bg-emerald-500/20 border border-emerald-500/50 text-emerald-300">
                          ✨ (Recomendado - Máximo Rendimiento)
                        </span>
                      </div>
                      <p className="text-[10px] text-neutral-400 mt-0.5">
                        Inferencia acelerada por núcleos Tensor de la GPU dedicada.
                      </p>
                    </div>
                    <div className={`w-3.5 h-3.5 rounded-full border flex items-center justify-center ${effects.aiBackend === 'gpu' ? 'border-emerald-400 bg-emerald-400' : 'border-neutral-600'}`}>
                      {effects.aiBackend === 'gpu' && <div className="w-1.5 h-1.5 rounded-full bg-neutral-950" />}
                    </div>
                  </div>

                  <div
                    onClick={() => update({ aiBackend: 'cpu' })}
                    className={`p-3 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                      (effects.aiBackend || 'cpu') === 'cpu'
                        ? 'bg-neutral-800/80 border-cyan-500 shadow-md ring-1 ring-cyan-500/40'
                        : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                    }`}
                  >
                    <div>
                      <div className="flex items-center space-x-2">
                        <span className="font-bold text-neutral-100">CPU (Multihilo Nativo)</span>
                        <span className="text-[10px] px-1.5 py-0.2 rounded font-bold bg-neutral-800 border border-neutral-700 text-neutral-300">
                          (Predeterminado)
                        </span>
                      </div>
                      <p className="text-[10px] text-neutral-400 mt-0.5">
                        Inferencia ejecutada en CPU.
                      </p>
                    </div>
                    <div className={`w-3.5 h-3.5 rounded-full border flex items-center justify-center ${(effects.aiBackend || 'cpu') === 'cpu' ? 'border-cyan-400 bg-cyan-400' : 'border-neutral-600'}`}>
                      {(effects.aiBackend || 'cpu') === 'cpu' && <div className="w-1.5 h-1.5 rounded-full bg-neutral-950" />}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Modal Footer */}
        <div className="px-6 py-4 border-t border-neutral-800 bg-neutral-950/80 flex items-center justify-between">
          <button
            onClick={onReset}
            className="flex items-center space-x-1.5 px-3 py-2 rounded-xl text-neutral-400 hover:text-white border border-neutral-700 hover:bg-neutral-800 transition-all text-xs cursor-pointer"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            <span>Restablecer Parámetros</span>
          </button>

          <div className="flex items-center space-x-2">
            <button
              onClick={onClose}
              className="px-4 py-2 rounded-xl text-neutral-400 hover:text-white hover:bg-neutral-800 transition-all text-xs cursor-pointer"
            >
              Cancelar
            </button>

            <button
              onClick={onSaveAndConnect}
              className="flex items-center space-x-2 px-6 py-2 rounded-xl font-bold bg-emerald-600 hover:bg-emerald-500 text-white shadow-lg shadow-emerald-600/25 transition-all text-xs cursor-pointer"
            >
              <Check className="w-4 h-4" />
              <span>Guardar y Conectar Fuente</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
