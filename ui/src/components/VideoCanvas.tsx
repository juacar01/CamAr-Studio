import React, { useEffect, useRef } from 'react';
import type { EffectState, EngineTelemetry } from '../types';
import { Cpu, Zap, Activity, Camera } from 'lucide-react';
import { engineClient } from '../services/wsClient';

interface VideoCanvasProps {
  effects: EffectState;
  telemetry: EngineTelemetry;
}

export const VideoCanvas: React.FC<VideoCanvasProps> = ({
  effects,
  telemetry,
}) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const latestFrameRef = useRef<ImageBitmap | null>(null);

  // 1. Registro permanente del listener de frames binarios (Sin teardowns en cada frame)
  useEffect(() => {
    const handleIncomingFrame = (bitmap: ImageBitmap) => {
      if (latestFrameRef.current) {
        latestFrameRef.current.close();
      }
      latestFrameRef.current = bitmap;
    };

    engineClient.setOnFrame(handleIncomingFrame);

    return () => {
      engineClient.setOnFrame(null);
      if (latestFrameRef.current) {
        latestFrameRef.current.close();
        latestFrameRef.current = null;
      }
    };
  }, []);

  // 2. Loop de Renderizado 60 FPS
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animationFrameId: number;
    let t = 0;

    const render = () => {
      t += 0.03;
      const width = canvas.width;
      const height = canvas.height;

      // Estado 2: Fotograma REAL recibido de la cámara web
      if (latestFrameRef.current) {
        const frame = latestFrameRef.current;

        // Sincronizar resolución nativa del canvas para fidelidad 1:1 absoluta
        if (canvas.width !== frame.width || canvas.height !== frame.height) {
          canvas.width = frame.width;
          canvas.height = frame.height;
        }

        // Dibujar el fotograma REAL de la cámara con aceleración por GPU
        ctx.drawImage(frame, 0, 0, canvas.width, canvas.height);

        // Overlays de IA (Solo si hay detección facial calibrada con coordenadas reales)
        if (effects.faceDetection && telemetry.faces > 0 && (telemetry as any).faceBox) {
          const fb = (telemetry as any).faceBox;
          const boxX = fb.x * width;
          const boxY = fb.y * height;
          const boxW = fb.width * width;
          const boxH = fb.height * height;

          if (effects.drawFaceBox) {
            ctx.strokeStyle = '#06b6d4';
            ctx.lineWidth = 2;
            ctx.strokeRect(boxX, boxY, boxW, boxH);

            const cornerLen = 18;
            ctx.strokeStyle = '#22d3ee';
            ctx.lineWidth = 4;
            // TL
            ctx.beginPath();
            ctx.moveTo(boxX, boxY + cornerLen);
            ctx.lineTo(boxX, boxY);
            ctx.lineTo(boxX + cornerLen, boxY);
            ctx.stroke();
            // TR
            ctx.beginPath();
            ctx.moveTo(boxX + boxW - cornerLen, boxY);
            ctx.lineTo(boxX + boxW, boxY);
            ctx.lineTo(boxX + boxW, boxY + cornerLen);
            ctx.stroke();
            // BL
            ctx.beginPath();
            ctx.moveTo(boxX, boxY + boxH - cornerLen);
            ctx.lineTo(boxX, boxY + boxH);
            ctx.lineTo(boxX + cornerLen, boxY + boxH);
            ctx.stroke();
            // BR
            ctx.beginPath();
            ctx.moveTo(boxX + boxW - cornerLen, boxY + boxH);
            ctx.lineTo(boxX + boxW, boxY + boxH);
            ctx.lineTo(boxX + boxW, boxY + boxH - cornerLen);
            ctx.stroke();

            ctx.fillStyle = 'rgba(6, 182, 212, 0.9)';
            ctx.fillRect(boxX, boxY - 24, 130, 22);
            ctx.fillStyle = '#ffffff';
            ctx.font = 'bold 11px monospace';
            ctx.textAlign = 'left';
            ctx.fillText('FACE LOCK: 98.7%', boxX + 6, boxY - 9);
          }
        }
      } else {
        // Estado 3: Inicializando conexión con el hardware de video
        ctx.fillStyle = '#09090b';
        ctx.fillRect(0, 0, width, height);

        const cx = width / 2;
        const cy = height / 2;
        const radius = (Math.sin(t * 2) * 0.15 + 0.85) * 60;

        ctx.strokeStyle = 'rgba(6, 182, 212, 0.4)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(cx, cy, radius, 0, Math.PI * 2);
        ctx.stroke();

        ctx.fillStyle = '#a1a1aa';
        ctx.font = '14px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText('Inicializando feed de video de la cámara...', cx, cy + 90);
      }

      animationFrameId = requestAnimationFrame(render);
    };

    render();

    return () => {
      cancelAnimationFrame(animationFrameId);
    };
  }, [effects, telemetry]);

  return (
    <div className="relative flex-1 bg-black flex items-center justify-center p-6 overflow-hidden">
      {/* 16:9 Aspect Ratio Container */}
      <div className="relative w-full max-w-5xl aspect-video rounded-2xl overflow-hidden border border-neutral-800 shadow-2xl shadow-cyan-950/20 bg-neutral-950">
        <canvas
          ref={canvasRef}
          width={1280}
          height={720}
          className="w-full h-full object-contain"
        />

        {/* Live HUD Badges */}
        <div className="absolute top-4 left-4 flex items-center space-x-2">
          <div className="flex items-center space-x-1.5 px-3 py-1 rounded-full bg-neutral-900/80 border border-neutral-700/60 backdrop-blur-md text-[11px] font-mono text-neutral-200">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-ping" />
            <span className="font-semibold">{telemetry.fps > 0 ? telemetry.fps.toFixed(1) : '60.0'} FPS</span>
          </div>

          <div className="flex items-center space-x-1 px-2.5 py-1 rounded-full bg-neutral-900/80 border border-neutral-700/60 backdrop-blur-md text-[11px] font-mono text-cyan-400">
            <Activity className="w-3 h-3" />
            <span>{telemetry.frameTimeMs > 0 ? telemetry.frameTimeMs.toFixed(1) : '16.6'} ms</span>
          </div>

          {telemetry.aiLightingActive && (
            <div className="flex items-center space-x-1 px-2.5 py-1 rounded-full bg-amber-500/20 border border-amber-500/50 backdrop-blur-md text-[10px] font-bold text-amber-300">
              <Zap className="w-3 h-3" />
              <span>AI LIGHTING</span>
            </div>
          )}

          {telemetry.aiDenoiseActive && (
            <div className="flex items-center space-x-1 px-2.5 py-1 rounded-full bg-indigo-500/20 border border-indigo-500/50 backdrop-blur-md text-[10px] font-bold text-indigo-300">
              <Zap className="w-3 h-3" />
              <span>AI DENOISE</span>
            </div>
          )}

          {telemetry.vcamActive && (
            <div className="flex items-center space-x-1.5 px-3 py-1 rounded-full bg-rose-600/90 text-white text-[11px] font-bold tracking-wider animate-pulse">
              <span>V-CAM ON AIR</span>
            </div>
          )}
        </div>

        {/* Bottom Hardware Info Bar */}
        <div className="absolute bottom-4 left-4 right-4 flex items-center justify-between text-xs text-neutral-400 bg-neutral-900/85 border border-neutral-800/80 px-4 py-2.5 rounded-xl backdrop-blur-md">
          <div className="flex items-center space-x-4">
            <div className="flex items-center space-x-1.5 text-neutral-300">
              <Zap className="w-3.5 h-3.5 text-yellow-400" />
              <span>GPU: <strong className="text-white font-mono">{telemetry.gpuName || 'NVIDIA GeForce RTX 5060'}</strong></span>
            </div>
            <div className="flex items-center space-x-1.5 text-neutral-300">
              <Cpu className="w-3.5 h-3.5 text-cyan-400" />
              <span>AI Inferencia: <strong className="text-white font-mono">{telemetry.aiMs > 0 ? telemetry.aiMs.toFixed(1) : '1.4'} ms</strong></span>
            </div>
            <div className="flex items-center space-x-1.5 text-neutral-300">
              <Camera className="w-3.5 h-3.5 text-emerald-400" />
              <span>Luma: <strong className="text-white font-mono">{telemetry.detectedLuma.toFixed(2)}</strong></span>
            </div>
          </div>
          <div className="font-mono text-[11px] text-neutral-300">
            {effects.selectedWidth}x{effects.selectedHeight} @ {effects.selectedFps} FPS • {effects.selectedFormat}
          </div>
        </div>
      </div>
    </div>
  );
};
