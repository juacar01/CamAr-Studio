import type { EffectState, EngineTelemetry, DevicesPayload } from '../types';
import { uiLogger } from './logger';

export class EngineSocketClient {
  private socket: WebSocket | null = null;
  private url: string;
  private onTelemetryCallback: ((telemetry: EngineTelemetry) => void) | null = null;
  private onDevicesCallback: ((devices: DevicesPayload) => void) | null = null;
  private onStatusChangeCallback: ((connected: boolean) => void) | null = null;
  private reconnectTimer: number | null = null;
  private pendingUpdateTimer: number | null = null;
  private latestPendingEffects: EffectState | null = null;

  private onFrameCallback: ((bitmap: ImageBitmap) => void) | null = null;
  private isDecodingFrame: boolean = false;

  private onPresetLoadedCallback: ((preset: any) => void) | null = null;

  constructor(url: string = 'ws://127.0.0.1:8765') {
    this.url = url;
  }

  public connect(
    onTelemetry: (telemetry: EngineTelemetry) => void,
    onDevices: (devices: DevicesPayload) => void,
    onStatusChange: (connected: boolean) => void,
    onPresetLoaded?: (preset: any) => void
  ) {
    this.onTelemetryCallback = onTelemetry;
    this.onDevicesCallback = onDevices;
    this.onStatusChangeCallback = onStatusChange;
    this.onPresetLoadedCallback = onPresetLoaded || null;
    this.initSocket();
  }

  public setOnFrame(callback: ((bitmap: ImageBitmap) => void) | null) {
    this.onFrameCallback = callback;
  }

  private initSocket() {
    try {
      this.socket = new WebSocket(this.url);
      this.socket.binaryType = 'blob';

      this.socket.onopen = () => {
        uiLogger.info('WebSocket', 'Conectado exitosamente con el motor multimedia nativo [Rust 60 FPS]');
        this.onStatusChangeCallback?.(true);
        if (this.reconnectTimer) {
          window.clearTimeout(this.reconnectTimer);
          this.reconnectTimer = null;
        }
      };

      this.socket.onmessage = (event) => {
        if (event.data instanceof Blob) {
          if (this.onFrameCallback) {
            if (this.isDecodingFrame) {
              return;
            }
            this.isDecodingFrame = true;
            createImageBitmap(event.data)
              .then((bitmap) => {
                this.isDecodingFrame = false;
                this.onFrameCallback?.(bitmap);
              })
              .catch((err) => {
                this.isDecodingFrame = false;
                uiLogger.warn('Preview', `Aviso al decodificar frame bitmap: ${err}`);
              });
          }
          return;
        }

        try {
          const data = JSON.parse(event.data);
          if (data.type === 'TELEMETRY' && this.onTelemetryCallback) {
            this.onTelemetryCallback(data);
          } else if (data.type === 'DEVICES_LIST' && this.onDevicesCallback) {
            uiLogger.info('Hardware', `Enumeración de dispositivos: ${data.cameras?.length || 0} cámaras, ${data.microphones?.length || 0} micrófonos`);
            this.onDevicesCallback(data);
          } else if (data.type === 'LOAD_PRESET' && this.onPresetLoadedCallback) {
            uiLogger.info('Config', `Preset restaurado desde almacenamiento de CamAr Studio`);
            this.onPresetLoadedCallback(data.config);
          }
        } catch (e) {
          uiLogger.error('WebSocket', `Error al interpretar mensaje JSON: ${e}`);
        }
      };

      this.socket.onclose = () => {
        uiLogger.warn('WebSocket', 'Conexión con el motor cerrada. Intentando reconexión automática...');
        this.onStatusChangeCallback?.(false);
        this.scheduleReconnect();
      };

      this.socket.onerror = () => {
        uiLogger.error('WebSocket', 'Fallo o error de comunicación en WebSocket');
        this.socket?.close();
      };
    } catch (e) {
      uiLogger.error('WebSocket', `Error al iniciar WebSocket: ${e}`);
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect() {
    if (!this.reconnectTimer) {
      this.reconnectTimer = window.setTimeout(() => {
        this.reconnectTimer = null;
        this.initSocket();
      }, 2000);
    }
  }

  public sendConfigUpdate(effects: EffectState) {
    this.latestPendingEffects = effects;
    if (!this.pendingUpdateTimer) {
      this.pendingUpdateTimer = window.requestAnimationFrame(() => {
        if (this.socket && this.socket.readyState === WebSocket.OPEN && this.latestPendingEffects) {
          const payload = {
            command: 'UPDATE_CONFIG',
            ...this.latestPendingEffects,
          };
          this.socket.send(JSON.stringify(payload));
        }
        this.pendingUpdateTimer = null;
      });
    }
  }

  public selectCamera(cameraId: string, width: number, height: number, fps: number, format: string) {
    uiLogger.info('Camera', `Cambiando cámara a ID '${cameraId}' (${width}x${height} @ ${fps} FPS [${format}])`);
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({
        command: 'SELECT_CAMERA',
        cameraId,
        width,
        height,
        fps,
        format,
      }));
    }
  }

  public selectAudio(audioId: string) {
    uiLogger.info('Audio', `Cambiando dispositivo de audio a '${audioId}'`);
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({
        command: 'SELECT_AUDIO',
        audioId,
      }));
    }
  }

  public setAudioVolume(volume: number) {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({
        command: 'SET_AUDIO_VOLUME',
        volume,
      }));
    }
  }

  public toggleMute(muted?: boolean) {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({
        command: 'TOGGLE_MUTE',
        muted,
      }));
    }
  }

  public toggleVirtualCam(active: boolean) {
    uiLogger.info('VirtualCam', `Cámara Virtual ${active ? 'INICIADA' : 'DETENIDA'}`);
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({
        command: 'TOGGLE_VCAM',
        active,
      }));
    }
  }

  public pausePreview() {
    uiLogger.info('Preview', 'Pausando stream de preview por configuración');
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({
        command: 'PAUSE_PREVIEW',
      }));
    }
  }

  public resumePreview() {
    uiLogger.info('Preview', 'Reanudando stream de preview');
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({
        command: 'RESUME_PREVIEW',
      }));
    }
  }

  public savePresetFile(preset: any, item?: any) {
    uiLogger.info('Presets', `Guardando preset '${preset?.name || preset?.id}' directamente en archivo JSON en disco`);
    if (this.socket && this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(JSON.stringify({
        command: 'SAVE_PRESET_FILE',
        preset,
        item,
      }));
    }
  }

  public disconnect() {
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
  }
}

export const engineClient = new EngineSocketClient();
