import React, { useState, useEffect } from 'react';
import { X, Terminal, AlertTriangle, AlertCircle, Info, Copy, Check, Trash2, FileText } from 'lucide-react';
import { uiLogger, type LogEntry } from '../services/logger';

interface LogsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const LogsModal: React.FC<LogsModalProps> = ({ isOpen, onClose }) => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filter, setFilter] = useState<'ALL' | 'ERROR' | 'WARN' | 'INFO'>('ALL');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const unsubscribe = uiLogger.subscribe((newLogs) => {
      setLogs(newLogs);
    });
    return () => unsubscribe();
  }, []);

  if (!isOpen) return null;

  const filteredLogs = logs.filter((log) => {
    if (filter === 'ALL') return true;
    return log.level === filter;
  });

  const handleCopy = () => {
    const text = logs
      .map((l) => `[${l.timestamp}] [${l.level}] [${l.module}] ${l.message}`)
      .join('\n');
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const errorCount = logs.filter((l) => l.level === 'ERROR').length;
  const warnCount = logs.filter((l) => l.level === 'WARN').length;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md p-4 animate-in fade-in duration-150">
      <div className="bg-neutral-900 border border-neutral-700/80 rounded-2xl w-full max-w-4xl h-[700px] flex flex-col shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="p-4 border-b border-neutral-800 flex items-center justify-between bg-neutral-950/70">
          <div className="flex items-center space-x-3">
            <div className="p-2 rounded-xl bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
              <Terminal className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-base font-bold text-white flex items-center space-x-2">
                <span>Trazabilidad & Logs de Eventos</span>
                {errorCount > 0 && (
                  <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-rose-500/20 text-rose-400 border border-rose-500/40">
                    {errorCount} Errores
                  </span>
                )}
                {warnCount > 0 && (
                  <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/20 text-amber-400 border border-amber-500/40">
                    {warnCount} Avisos
                  </span>
                )}
              </h2>
              <p className="text-xs text-neutral-400">
                Registro en tiempo real de captura de hardware, audio, IA y WebSockets
              </p>
            </div>
          </div>

          <div className="flex items-center space-x-2">
            <button
              onClick={handleCopy}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-300 hover:text-white border border-neutral-700 text-xs transition-colors cursor-pointer"
            >
              {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
              <span>{copied ? 'Copiado' : 'Copiar'}</span>
            </button>
            <button
              onClick={() => uiLogger.clear()}
              title="Limpiar logs"
              className="p-1.5 rounded-lg bg-neutral-800 hover:bg-rose-950/40 hover:text-rose-400 text-neutral-400 border border-neutral-700 text-xs transition-colors cursor-pointer"
            >
              <Trash2 className="w-4 h-4" />
            </button>
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-400 hover:text-white transition-colors cursor-pointer ml-2"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Archivos persistentes info banner */}
        <div className="bg-neutral-950 px-4 py-2 border-b border-neutral-800 flex items-center justify-between text-xs text-neutral-400 font-mono">
          <div className="flex items-center space-x-2">
            <FileText className="w-3.5 h-3.5 text-cyan-400" />
            <span>Archivo persistente en disco: <strong className="text-neutral-200">logs/camar_studio.log</strong> & <strong className="text-rose-300">logs/error.log</strong></span>
          </div>

          {/* Filtros */}
          <div className="flex items-center space-x-1 bg-neutral-900 p-0.5 rounded-lg border border-neutral-800">
            {(['ALL', 'ERROR', 'WARN', 'INFO'] as const).map((lvl) => (
              <button
                key={lvl}
                onClick={() => setFilter(lvl)}
                className={`px-2 py-0.5 rounded text-[11px] font-bold transition-colors cursor-pointer ${
                  filter === lvl
                    ? 'bg-neutral-700 text-white shadow-sm'
                    : 'text-neutral-400 hover:text-neutral-200'
                }`}
              >
                {lvl}
              </button>
            ))}
          </div>
        </div>

        {/* Console Log Area */}
        <div className="flex-1 bg-neutral-950 p-4 overflow-y-auto font-mono text-xs space-y-1.5 select-text">
          {filteredLogs.length === 0 ? (
            <div className="h-full flex items-center justify-center text-neutral-600">
              No hay eventos ni errores registrados en esta categoría
            </div>
          ) : (
            filteredLogs.map((log) => {
              let badgeColor = 'bg-cyan-500/10 text-cyan-400 border-cyan-500/30';
              let Icon = Info;
              if (log.level === 'ERROR') {
                badgeColor = 'bg-rose-500/20 text-rose-300 border-rose-500/40';
                Icon = AlertCircle;
              } else if (log.level === 'WARN') {
                badgeColor = 'bg-amber-500/20 text-amber-300 border-amber-500/40';
                Icon = AlertTriangle;
              }

              return (
                <div
                  key={log.id}
                  className="flex items-start space-x-3 p-1.5 rounded hover:bg-neutral-900/60 transition-colors border-b border-neutral-900/60"
                >
                  <span className="text-neutral-500 shrink-0 text-[11px]">{log.timestamp}</span>
                  <span className={`px-1.5 py-0.2 rounded border text-[10px] font-bold shrink-0 flex items-center space-x-1 ${badgeColor}`}>
                    <Icon className="w-3 h-3 inline mr-1" />
                    <span>{log.level}</span>
                  </span>
                  <span className="text-pink-400 font-semibold shrink-0">[{log.module}]</span>
                  <span className="text-neutral-200 flex-1 break-all">{log.message}</span>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};
