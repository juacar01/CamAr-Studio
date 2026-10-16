# CamAr Studio [Standalone Desktop Application]

Aplicación de escritorio nativa para Windows desarrollada en **Rust**, que integra:
- **Interfaz de Usuario Embebida:** HTML5 / TypeScript / React / Tailwind CSS renderizada mediante el motor WebView2 nativo de Windows (sin dependencias externas de NodeJS o Chrome en tiempo de ejecución).
- **Core Video Processing Engine:** Pipeline multihilo con **Rayon** a 60 FPS fijos.
- **Detección Facial & Segmentación:** Filtro temporal anti-jitter (EMA) y segmentación de silueta con *feathering*.
- **Salida a Cámara Virtual:** Memoria compartida Win32 (`Local\CamAr_VirtualCam_SharedMem`) sincronizada por eventos con latencia sub-milisegundo.
- **Un solo binario `.exe`:** Todo el software, interfaz y motor pesan apenas **1.2 MB**.

---

## 🚀 Cómo Iniciar la Aplicación

Simplemente haz doble clic en el archivo raíz:
```text
Iniciar_CamAr_Studio.bat
```
o ejecuta directamente el binario compilado:
```text
engine-rs\target\release\camar-engine.exe
```

No necesitas abrir navegadores externos, ni ejecutar servidores web, ni tener Node.js instalado para correr la aplicación.

---

## 🏗️ Arquitectura Técnica

```text
[ camar-engine.exe (1.2 MB) ]
 ├── Hilo Principal (GUI Thread - TAO + WRY):
 │    ├── Ventana Nativa de Escritorio (1440x860)
 │    ├── WebView2 incrustado nativo de Windows
 │    └── Protocolo "camar://": Sirve los assets compilados de React/Tailwind directamente desde memoria (include_dir)
 │
 └── Hilo Secundario (Multimedia & Engine Thread - Tokio Runtime):
      ├── Ingesta de Video: 1080p @ 60 FPS
      ├── Pipeline IA: Detección facial (6 landmarks) + Segmentación de personas
      ├── Pipeline de Efectos: Color grading, Beautify bilateral y Fondos (Bokeh/Estudio/Chroma)
      ├── Salida de Cámara Virtual: Win32 Shared Memory (OBS, Zoom, Meet, Teams, TikTok)
      └── Servidor IPC WebSocket: Comunicación bidireccional local a 60 FPS con la interfaz
```
