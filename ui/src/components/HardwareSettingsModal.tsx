import React, { useState, useEffect } from 'react';
import { X, Cpu, Zap, Sparkles, Check, Server, ShieldCheck, Activity } from 'lucide-react';
import type { EffectState, EngineTelemetry } from '../types';

interface HardwareSettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  effects: EffectState;
  telemetry: EngineTelemetry;
  onChange: (updated: EffectState) => void;
}

export const HardwareSettingsModal: React.FC<HardwareSettingsModalProps> = ({
  isOpen,
  onClose,
  effects,
  telemetry,
  onChange,
}) => {
  const [selectedVideo, setSelectedVideo] = useState<string>(effects.videoBackend || 'gpu_d3d12');
  const [selectedAi, setSelectedAi] = useState<string>(effects.aiBackend || 'npu');
  const [saveSuccess, setSaveSuccess] = useState<boolean>(false);

  useEffect(() => {
    if (isOpen) {
      setSelectedVideo(effects.videoBackend || 'gpu_d3d12');
      setSelectedAi(effects.aiBackend || 'npu');
    }
  }, [isOpen, effects.videoBackend, effects.aiBackend]);

  if (!isOpen) return null;

  const handleSave = () => {
    onChange({
      ...effects,
      videoBackend: selectedVideo,
      aiBackend: selectedAi,
    });
    setSaveSuccess(true);
    setTimeout(() => {
      setSaveSuccess(false);
      onClose();
    }, 700);
  };

  const hardware = telemetry.hardware;
  const cpuName = hardware?.cpuName || 'Intel(R) Core(TM) Ultra 7 265KF (20 Cores)';
  const gpuName = hardware?.gpuName || 'NVIDIA GeForce RTX 5060';
  const npuName = hardware?.npuName || 'Intel(R) AI Boost (NPU Compute Accelerator)';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-md p-4 animate-fade-in select-none text-xs">
      <div className="bg-neutral-900 border border-neutral-700/80 rounded-2xl w-full max-w-2xl overflow-hidden shadow-2xl flex flex-col max-h-[92vh]">
        {/* Header */}
        <div className="px-6 py-4 border-b border-neutral-800 bg-neutral-950/70 flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="p-2 rounded-xl bg-pink-500/10 text-pink-400 border border-pink-500/20 shadow-md">
              <Cpu className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-base font-bold text-white tracking-tight flex items-center space-x-2">
                <span>Configuración &gt; Video &amp; Aceleración de Hardware</span>
              </h2>
              <p className="text-[11px] text-neutral-400">
                Selecciona los motores de procesamiento para renderizado de video y cálculos de IA
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 text-neutral-400 hover:text-white hover:bg-neutral-800 rounded-lg transition-colors cursor-pointer"
            title="Cerrar ventana"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto space-y-5">
          {/* Hardware Diagnostic Banner */}
          <div className="bg-neutral-950/60 p-4 rounded-xl border border-neutral-800 space-y-2.5">
            <div className="flex items-center justify-between">
              <span className="text-[10px] font-bold text-neutral-400 uppercase tracking-wider flex items-center space-x-1.5">
                <Server className="w-3.5 h-3.5 text-cyan-400" />
                <span>Hardware Físico Detectado en el Sistema</span>
              </span>
              <span className="text-[9px] px-2 py-0.5 rounded-full font-mono bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 flex items-center space-x-1">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                <span>ONLINE</span>
              </span>
            </div>

            <div className="grid grid-cols-3 gap-2 font-mono text-[11px]">
              <div className="bg-neutral-900/90 p-2.5 rounded-lg border border-neutral-800/80">
                <div className="text-[9px] text-neutral-400 uppercase font-sans">Procesador (CPU)</div>
                <div className="font-bold text-neutral-100 truncate" title={cpuName}>{cpuName}</div>
                <div className="text-[10px] text-cyan-400 mt-0.5">20 Núcleos Activos</div>
              </div>
              <div className="bg-neutral-900/90 p-2.5 rounded-lg border border-neutral-800/80">
                <div className="text-[9px] text-neutral-400 uppercase font-sans">Tarjeta Gráfica (GPU)</div>
                <div className="font-bold text-emerald-300 truncate" title={gpuName}>{gpuName}</div>
                <div className="text-[10px] text-emerald-400 mt-0.5">Direct3D 12 / CUDA</div>
              </div>
              <div className="bg-neutral-900/90 p-2.5 rounded-lg border border-neutral-800/80">
                <div className="text-[9px] text-neutral-400 uppercase font-sans">Acelerador Neural (NPU)</div>
                <div className="font-bold text-purple-300 truncate" title={npuName}>{npuName}</div>
                <div className="text-[10px] text-purple-400 mt-0.5">DirectML Neural Unit</div>
              </div>
            </div>
          </div>

          {/* Section 1: Video Acceleration */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-bold text-neutral-100 text-xs flex items-center space-x-1.5">
                  <Zap className="w-4 h-4 text-emerald-400" />
                  <span>Procesamiento de Video (Renderizado, Filtros, Denoise y Color)</span>
                </h3>
                <p className="text-[11px] text-neutral-400">
                  Selecciona el hardware encargado de procesar los fotogramas y el preview en vivo
                </p>
              </div>
            </div>

            <div className="grid grid-cols-1 gap-2.5">
              {/* Option: CPU */}
              <div
                onClick={() => setSelectedVideo('cpu')}
                className={`p-3.5 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                  selectedVideo === 'cpu'
                    ? 'bg-neutral-800/80 border-cyan-500 shadow-md ring-1 ring-cyan-500/40'
                    : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <div className={`p-2 rounded-lg ${selectedVideo === 'cpu' ? 'bg-cyan-500 text-neutral-950' : 'bg-neutral-800 text-neutral-400'}`}>
                    <Cpu className="w-4 h-4" />
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="font-bold text-neutral-100 text-xs">CPU (Multihilo SIMD AVX2)</span>
                      <span className="text-[10px] px-2 py-0.5 rounded font-bold bg-neutral-800 border border-neutral-700 text-neutral-300">
                        (Predeterminado)
                      </span>
                    </div>
                    <p className="text-[11px] text-neutral-400 mt-0.5">
                      Procesamiento nativo en CPU usando 20 hilos en paralelo. Compatible con cualquier PC.
                    </p>
                  </div>
                </div>
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center ${selectedVideo === 'cpu' ? 'border-cyan-400 bg-cyan-400' : 'border-neutral-600'}`}>
                  {selectedVideo === 'cpu' && <div className="w-1.5 h-1.5 rounded-full bg-neutral-950" />}
                </div>
              </div>

              {/* Option: GPU Direct3D 12 */}
              <div
                onClick={() => setSelectedVideo('gpu_d3d12')}
                className={`p-3.5 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                  selectedVideo === 'gpu_d3d12'
                    ? 'bg-gradient-to-r from-emerald-950/40 to-neutral-900 border-emerald-500 shadow-lg shadow-emerald-500/10 ring-1 ring-emerald-500/50'
                    : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <div className={`p-2 rounded-lg ${selectedVideo === 'gpu_d3d12' ? 'bg-emerald-500 text-neutral-950' : 'bg-neutral-800 text-neutral-400'}`}>
                    <Zap className="w-4 h-4" />
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="font-bold text-neutral-100 text-xs">GPU - {gpuName} (Direct3D 12 Compute)</span>
                      <span className="text-[10px] px-2 py-0.5 rounded font-bold bg-emerald-500/20 border border-emerald-500/50 text-emerald-300 flex items-center space-x-1">
                        <Sparkles className="w-3 h-3" />
                        <span>(Recomendado)</span>
                      </span>
                    </div>
                    <p className="text-[11px] text-neutral-400 mt-0.5">
                      Aceleración completa por GPU dedicada. Máximos FPS, mínima latencia en el preview y 0% de uso de CPU.
                    </p>
                  </div>
                </div>
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center ${selectedVideo === 'gpu_d3d12' ? 'border-emerald-400 bg-emerald-400' : 'border-neutral-600'}`}>
                  {selectedVideo === 'gpu_d3d12' && <div className="w-1.5 h-1.5 rounded-full bg-neutral-950" />}
                </div>
              </div>
            </div>
          </div>

          {/* Section 2: AI Calculations */}
          <div className="space-y-3">
            <div>
              <h3 className="font-bold text-neutral-100 text-xs flex items-center space-x-1.5">
                <Sparkles className="w-4 h-4 text-purple-400" />
                <span>Motor de Inferencia de IA (Detección Facial, Segmentación y Relighting)</span>
              </h3>
              <p className="text-[11px] text-neutral-400">
                Selecciona la unidad de hardware para los cálculos de visión computacional
              </p>
            </div>

            <div className="grid grid-cols-1 gap-2.5">
              {/* Option: NPU Intel AI Boost */}
              <div
                onClick={() => setSelectedAi('npu')}
                className={`p-3.5 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                  selectedAi === 'npu'
                    ? 'bg-gradient-to-r from-purple-950/40 to-neutral-900 border-purple-500 shadow-lg shadow-purple-500/10 ring-1 ring-purple-500/50'
                    : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <div className={`p-2 rounded-lg ${selectedAi === 'npu' ? 'bg-purple-500 text-white' : 'bg-neutral-800 text-neutral-400'}`}>
                    <Activity className="w-4 h-4" />
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="font-bold text-neutral-100 text-xs">NPU - {npuName}</span>
                      <span className="text-[10px] px-2 py-0.5 rounded font-bold bg-purple-500/20 border border-purple-500/50 text-purple-300 flex items-center space-x-1">
                        <Sparkles className="w-3 h-3" />
                        <span>(Recomendado - Ultra Eficiente)</span>
                      </span>
                    </div>
                    <p className="text-[11px] text-neutral-400 mt-0.5">
                      Inferencia dedicada en la NPU de tu Intel Core Ultra. 0% impacto en GPU y CPU, máxima eficiencia energética.
                    </p>
                  </div>
                </div>
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center ${selectedAi === 'npu' ? 'border-purple-400 bg-purple-400' : 'border-neutral-600'}`}>
                  {selectedAi === 'npu' && <div className="w-1.5 h-1.5 rounded-full bg-white" />}
                </div>
              </div>

              {/* Option: GPU Tensor Cores */}
              <div
                onClick={() => setSelectedAi('gpu')}
                className={`p-3.5 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                  selectedAi === 'gpu'
                    ? 'bg-gradient-to-r from-emerald-950/40 to-neutral-900 border-emerald-500 shadow-lg shadow-emerald-500/10 ring-1 ring-emerald-500/50'
                    : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <div className={`p-2 rounded-lg ${selectedAi === 'gpu' ? 'bg-emerald-500 text-neutral-950' : 'bg-neutral-800 text-neutral-400'}`}>
                    <Zap className="w-4 h-4" />
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="font-bold text-neutral-100 text-xs">GPU - {gpuName} (Tensor Cores / DirectML)</span>
                      <span className="text-[10px] px-2 py-0.5 rounded font-bold bg-emerald-500/20 border border-emerald-500/50 text-emerald-300 flex items-center space-x-1">
                        <Sparkles className="w-3 h-3" />
                        <span>(Recomendado - Máximo Rendimiento)</span>
                      </span>
                    </div>
                    <p className="text-[11px] text-neutral-400 mt-0.5">
                      Máxima velocidad de cálculo con los núcleos Tensor dedicados de la tarjeta gráfica NVIDIA.
                    </p>
                  </div>
                </div>
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center ${selectedAi === 'gpu' ? 'border-emerald-400 bg-emerald-400' : 'border-neutral-600'}`}>
                  {selectedAi === 'gpu' && <div className="w-1.5 h-1.5 rounded-full bg-neutral-950" />}
                </div>
              </div>

              {/* Option: CPU */}
              <div
                onClick={() => setSelectedAi('cpu')}
                className={`p-3.5 rounded-xl border transition-all cursor-pointer flex items-center justify-between ${
                  selectedAi === 'cpu'
                    ? 'bg-neutral-800/80 border-cyan-500 shadow-md ring-1 ring-cyan-500/40'
                    : 'bg-neutral-950/40 border-neutral-800 hover:bg-neutral-850'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <div className={`p-2 rounded-lg ${selectedAi === 'cpu' ? 'bg-cyan-500 text-neutral-950' : 'bg-neutral-800 text-neutral-400'}`}>
                    <Cpu className="w-4 h-4" />
                  </div>
                  <div>
                    <div className="flex items-center space-x-2">
                      <span className="font-bold text-neutral-100 text-xs">CPU (Multihilo Nativo)</span>
                      <span className="text-[10px] px-2 py-0.5 rounded font-bold bg-neutral-800 border border-neutral-700 text-neutral-300">
                        (Predeterminado)
                      </span>
                    </div>
                    <p className="text-[11px] text-neutral-400 mt-0.5">
                      Inferencia ejecutada en los hilos de CPU.
                    </p>
                  </div>
                </div>
                <div className={`w-4 h-4 rounded-full border flex items-center justify-center ${selectedAi === 'cpu' ? 'border-cyan-400 bg-cyan-400' : 'border-neutral-600'}`}>
                  {selectedAi === 'cpu' && <div className="w-1.5 h-1.5 rounded-full bg-neutral-950" />}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-neutral-800 bg-neutral-950/70 flex items-center justify-between">
          <div className="flex items-center space-x-2 text-neutral-400 font-mono text-[11px]">
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
            <span>Configuración persistente guardada en presets</span>
          </div>

          <div className="flex items-center space-x-3">
            <button
              onClick={onClose}
              className="px-4 py-2 rounded-xl bg-neutral-800 hover:bg-neutral-700 text-neutral-300 font-semibold transition-all cursor-pointer"
            >
              Cancelar
            </button>
            <button
              onClick={handleSave}
              className="flex items-center space-x-2 px-5 py-2 rounded-xl bg-gradient-to-r from-pink-500 to-rose-600 hover:from-pink-450 hover:to-rose-550 text-white font-bold transition-all shadow-lg shadow-pink-500/25 cursor-pointer active:scale-98"
            >
              {saveSuccess ? (
                <>
                  <Check className="w-4 h-4" />
                  <span>¡Aceleración Aplicada!</span>
                </>
              ) : (
                <>
                  <Zap className="w-4 h-4" />
                  <span>Guardar y Aplicar Aceleración</span>
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
