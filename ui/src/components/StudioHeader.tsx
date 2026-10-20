import React from 'react';
import { Video, VideoOff, Wifi, WifiOff, Sparkles, SlidersHorizontal, Camera, Mic, MicOff, Terminal, Cpu, RotateCw } from 'lucide-react';
import type { CameraDeviceInfo } from '../types';

interface StudioHeaderProps {
  connected: boolean;
  vcamActive: boolean;
  onToggleVcam: () => void;

  // Botón modal de logs y trazabilidad
  onOpenLogsModal: () => void;

  // Botón modal de hardware & video
  onOpenHardwareModal: () => void;

  // Botón modal de fuente
  onOpenSourceModal: () => void;
  cameras: CameraDeviceInfo[];
  selectedCameraId: string;
  selectedWidth: number;
  selectedHeight: number;
  selectedFps: number;
  selectedFormat: string;
  cameraRotation?: number;
  onRotateCamera?: () => void;

  // Audio resumen
  selectedAudioId: string;
  audioMuted: boolean;
  onToggleMute: () => void;
  audioPeakDb: number;
}

export const StudioHeader: React.FC<StudioHeaderProps> = ({
  connected,
  vcamActive,
  onToggleVcam,
  onOpenLogsModal,
  onOpenHardwareModal,
  onOpenSourceModal,
  cameras,
  selectedCameraId,
  selectedWidth,
  selectedHeight,
  selectedFps,
  selectedFormat,
  cameraRotation = 0,
  onRotateCamera,
  audioMuted,
  onToggleMute,
  audioPeakDb,
}) => {
  const currentCamera = cameras.find((c) => c.id === selectedCameraId) || cameras[0];
  const camDisplayName = currentCamera ? currentCamera.name : 'CamAr Studio Simulator';

  const isPortrait = cameraRotation === 90 || cameraRotation === 270;
  const displayW = isPortrait ? selectedHeight : selectedWidth;
  const displayH = isPortrait ? selectedWidth : selectedHeight;
  const vuPercent = Math.min(100, Math.max(0, ((audioPeakDb + 60) / 60) * 100));

  return (
    <header className="h-16 border-b border-neutral-800 bg-neutral-900/95 px-6 flex items-center justify-between backdrop-blur-md select-none text-xs">
      {/* Brand & Title */}
      <div className="flex items-center space-x-3">
        <div className="w-10 h-10 rounded-xl bg-gradient-to-tr from-pink-500 via-rose-500 to-cyan-400 p-[2px] shadow-lg shadow-pink-500/20">
          <div className="w-full h-full bg-neutral-950 rounded-[10px] flex items-center justify-center">
            <Sparkles className="w-5 h-5 text-pink-400" />
          </div>
        </div>
        <div>
          <div className="flex items-center space-x-2">
            <h1 className="font-bold text-sm tracking-tight text-white">CamAr Studio</h1>
            <span className="text-[9px] px-1.5 py-0.5 rounded font-mono font-bold bg-orange-500/10 text-orange-400 border border-orange-500/30">
              RUST 60 FPS
            </span>
          </div>
          <p className="text-[11px] text-neutral-400">Live Video Engine & Virtual Camera</p>
        </div>
      </div>

      {/* BOTÓN CENTRAL: SELECCIÓN Y CONFIGURACIÓN DE FUENTE */}
      <div className="flex items-center space-x-2.5">
        <button
          onClick={onOpenSourceModal}
          className="group flex items-center space-x-3 px-4 py-2 rounded-xl bg-neutral-800 hover:bg-neutral-750 border border-neutral-700/80 hover:border-pink-500/50 shadow-md transition-all cursor-pointer"
        >
          <div className="p-1.5 rounded-lg bg-pink-500/10 text-pink-400 group-hover:bg-pink-500 group-hover:text-white transition-all">
            <Camera className="w-4 h-4" />
          </div>

          <div className="text-left">
            <div className="flex items-center space-x-2">
              <span className="font-bold text-neutral-100 group-hover:text-pink-300 transition-colors max-w-[220px] truncate">
                {camDisplayName}
              </span>
              <span className="text-[10px] px-1.5 py-0.2 rounded font-mono bg-neutral-900 border border-neutral-700 text-neutral-300">
                {displayW}×{displayH} @ {selectedFps} FPS • {selectedFormat}
              </span>
            </div>
            <div className="text-[10px] text-neutral-400 flex items-center space-x-1">
              <SlidersHorizontal className="w-3 h-3 text-pink-400" />
              <span>Clic para modificar fuente, color, IA y audio</span>
            </div>
          </div>
        </button>

        {/* Botón Rápido de Rotación 90° (Orientación Vertical / Apaisado) */}
        {onRotateCamera && (
          <button
            onClick={onRotateCamera}
            title={`Orientación actual: ${cameraRotation}°. Clic para rotar 90°`}
            className={`flex items-center space-x-1.5 px-3 py-2 rounded-xl border transition-all cursor-pointer ${
              isPortrait
                ? 'bg-pink-500/20 border-pink-500/60 text-pink-300 shadow-sm shadow-pink-500/20 font-bold'
                : 'bg-neutral-800 hover:bg-neutral-750 border-neutral-700/80 text-neutral-300 hover:text-white'
            }`}
          >
            <RotateCw className={`w-3.5 h-3.5 ${isPortrait ? 'text-pink-400' : 'text-neutral-400'}`} />
            <span className="font-mono text-[11px]">{cameraRotation}°</span>
            {isPortrait && (
              <span className="text-[9px] px-1 py-0.2 rounded font-bold bg-pink-500/30 text-pink-200">9:16</span>
            )}
          </button>
        )}

        {/* Mini Vúmetro de Audio */}
        <div className="flex items-center space-x-2 bg-neutral-950/60 px-3 py-2 rounded-xl border border-neutral-800">
          <button
            onClick={onToggleMute}
            title={audioMuted ? 'Activar micrófono' : 'Silenciar micrófono'}
            className={`p-1 rounded-md transition-all ${
              audioMuted ? 'text-rose-400' : 'text-emerald-400'
            }`}
          >
            {audioMuted ? <MicOff className="w-3.5 h-3.5" /> : <Mic className="w-3.5 h-3.5" />}
          </button>
          <div className="w-12 h-2.5 bg-neutral-800 rounded-full overflow-hidden p-0.5 border border-neutral-700 flex items-center">
            <div
              className={`h-full rounded-full transition-all duration-75 ${
                audioMuted
                  ? 'bg-neutral-600'
                  : vuPercent > 85
                  ? 'bg-rose-500'
                  : vuPercent > 60
                  ? 'bg-amber-400'
                  : 'bg-emerald-500'
              }`}
              style={{ width: `${audioMuted ? 0 : vuPercent}%` }}
            />
          </div>
        </div>
      </div>

      {/* Estado del Engine y Botón Master de Cámara Virtual */}
      <div className="flex items-center space-x-3">
        <button
          onClick={onOpenHardwareModal}
          title="Configuración de Hardware: Seleccionar CPU, GPU y NPU para procesamiento de video e IA"
          className="flex items-center space-x-1.5 px-3 py-1.5 rounded-xl bg-neutral-800/80 hover:bg-neutral-750 border border-neutral-700/80 hover:border-pink-500/50 text-neutral-300 hover:text-pink-300 transition-all cursor-pointer"
        >
          <Cpu className="w-3.5 h-3.5 text-pink-400" />
          <span className="font-semibold text-[11px]">Configuración &gt; Video</span>
        </button>

        <button
          onClick={onOpenLogsModal}
          title="Ver registro de eventos, trazabilidad y logs de errores"
          className="flex items-center space-x-1.5 px-3 py-1.5 rounded-xl bg-neutral-800/80 hover:bg-neutral-750 border border-neutral-700/80 hover:border-cyan-500/50 text-neutral-300 hover:text-cyan-300 transition-all cursor-pointer"
        >
          <Terminal className="w-3.5 h-3.5 text-cyan-400" />
          <span className="font-semibold text-[11px]">Logs & Errores</span>
        </button>

        <div className="flex items-center space-x-1.5 px-3 py-1.5 rounded-xl bg-neutral-800/60 border border-neutral-700/60 text-[11px]">
          {connected ? (
            <>
              <Wifi className="w-3 h-3 text-emerald-400 animate-pulse" />
              <span className="text-emerald-400 font-semibold font-mono">60.0 FPS</span>
            </>
          ) : (
            <>
              <WifiOff className="w-3 h-3 text-amber-400" />
              <span className="text-amber-400 font-medium">Reconectando</span>
            </>
          )}
        </div>

        <button
          onClick={onToggleVcam}
          className={`flex items-center space-x-2 px-5 py-2.5 rounded-xl font-bold tracking-wide transition-all duration-200 shadow-lg ${
            vcamActive
              ? 'bg-rose-600 hover:bg-rose-500 text-white shadow-rose-600/30 ring-2 ring-rose-400/50 animate-pulse'
              : 'bg-emerald-600 hover:bg-emerald-500 text-white shadow-emerald-600/20'
          }`}
        >
          {vcamActive ? (
            <>
              <VideoOff className="w-4 h-4" />
              <span>DETENER CÁMARA VIRTUAL</span>
            </>
          ) : (
            <>
              <Video className="w-4 h-4" />
              <span>ACTIVAR CÁMARA VIRTUAL</span>
            </>
          )}
        </button>
      </div>
    </header>
  );
};
