#include "ipc/WebSocketServer.h"
#include "core/Types.h"
#include "processing/D3D11Renderer.h"
#include "capture/CameraCapture.h"
#include "ai/OnnxAiEngine.h"
#include "virtualcam/VirtualCamManager.h"

#include <iostream>
#include <chrono>
#include <thread>
#include <atomic>
#include <sstream>
#include <iomanip>

using namespace CamAr;

// Helper simple para parsing de valores JSON de control
static float ExtractJsonFloat(const std::string& json, const std::string& key, float defaultVal) {
    std::string needle = "\"" + key + "\":";
    size_t pos = json.find(needle);
    if (pos == std::string::npos) return defaultVal;
    pos += needle.length();
    while (pos < json.length() && (json[pos] == ' ' || json[pos] == '\t')) pos++;
    try {
        return std::stof(json.substr(pos));
    } catch (...) {
        return defaultVal;
    }
}

static bool ExtractJsonBool(const std::string& json, const std::string& key, bool defaultVal) {
    std::string needle = "\"" + key + "\":";
    size_t pos = json.find(needle);
    if (pos == std::string::npos) return defaultVal;
    pos += needle.length();
    while (pos < json.length() && (json[pos] == ' ' || json[pos] == '\t')) pos++;
    if (json.substr(pos, 4) == "true") return true;
    if (json.substr(pos, 5) == "false") return false;
    return defaultVal;
}

static int ExtractJsonInt(const std::string& json, const std::string& key, int defaultVal) {
    std::string needle = "\"" + key + "\":";
    size_t pos = json.find(needle);
    if (pos == std::string::npos) return defaultVal;
    pos += needle.length();
    while (pos < json.length() && (json[pos] == ' ' || json[pos] == '\t')) pos++;
    try {
        return std::stoi(json.substr(pos));
    } catch (...) {
        return defaultVal;
    }
}

int main(int argc, char* argv[]) {
    std::cout << "======================================================" << std::endl;
    std::cout << "        CamAr Studio Core Video Processing Engine     " << std::endl;
    std::cout << "  GPU-Accelerated Virtual Camera & Live Effects Core  " << std::endl;
    std::cout << "======================================================" << std::endl;

    StudioConfig config;
    config.targetWidth = 1920;
    config.targetHeight = 1080;
    config.targetFps = 60;

    // 1. Inicializar Motor D3D11
    auto renderer = std::make_unique<D3D11Renderer>();
    if (!renderer->Initialize(nullptr, config.targetWidth, config.targetHeight)) {
        std::cerr << "[Engine Error] No se pudo inicializar Direct3D 11." << std::endl;
        return 1;
    }

    // 2. Inicializar Motor de IA (Detección Facial + Segmentación)
    auto aiEngine = std::make_unique<OnnxAiEngine>();
    aiEngine->Initialize(config.targetWidth, config.targetHeight, "models/");

    // 3. Inicializar Virtual Camera Manager
    auto vcamManager = std::make_unique<VirtualCamManager>();

    // 4. Inicializar Captura de Cámara (por defecto el Simulador / Webcam activa)
    auto cameraCapture = std::make_unique<CameraCapture>();

    std::vector<uint8_t> latestRawFrame;
    std::mutex frameMutex;
    std::atomic<bool> newFrameAvailable{false};

    cameraCapture->Start("synthetic_camera", config.targetWidth, config.targetHeight, config.targetFps,
        [&](const uint8_t* rgbaData, int w, int h) {
            std::lock_guard<std::mutex> lock(frameMutex);
            size_t size = w * h * 4;
            if (latestRawFrame.size() != size) {
                latestRawFrame.resize(size);
            }
            memcpy(latestRawFrame.data(), rgbaData, size);
            newFrameAvailable = true;
        }
    );

    // 5. Inicializar Servidor IPC WebSocket para la UI
    auto wsServer = std::make_unique<WebSocketServer>();
    wsServer->Start(8765, [&](const std::string& msg) {
        // Procesar comandos de la UI
        if (msg.find("UPDATE_EFFECTS") != std::string::npos) {
            config.color.brightness = ExtractJsonFloat(msg, "brightness", config.color.brightness);
            config.color.contrast = ExtractJsonFloat(msg, "contrast", config.color.contrast);
            config.color.saturation = ExtractJsonFloat(msg, "saturation", config.color.saturation);
            config.color.exposure = ExtractJsonFloat(msg, "exposure", config.color.exposure);
            config.color.temperature = ExtractJsonFloat(msg, "temperature", config.color.temperature);

            config.beauty.enabled = ExtractJsonBool(msg, "beautyEnabled", config.beauty.enabled);
            config.beauty.skinSmoothing = ExtractJsonFloat(msg, "skinSmoothing", config.beauty.skinSmoothing);

            config.background.mode = static_cast<BackgroundMode>(ExtractJsonInt(msg, "bgMode", (int)config.background.mode));
            config.background.blurRadius = ExtractJsonFloat(msg, "blurRadius", config.background.blurRadius);
            config.background.edgeFeather = ExtractJsonFloat(msg, "edgeFeather", config.background.edgeFeather);

            config.ai.faceDetectionEnabled = ExtractJsonBool(msg, "faceDetection", config.ai.faceDetectionEnabled);
            config.ai.personSegmentationEnabled = ExtractJsonBool(msg, "personSegmentation", config.ai.personSegmentationEnabled);
            config.ai.segmentationThreshold = ExtractJsonFloat(msg, "segThreshold", config.ai.segmentationThreshold);
            config.ai.drawFaceBox = ExtractJsonBool(msg, "drawFaceBox", config.ai.drawFaceBox);
        } else if (msg.find("TOGGLE_VCAM") != std::string::npos) {
            bool activate = ExtractJsonBool(msg, "active", !vcamManager->IsActive());
            if (activate) {
                vcamManager->Start(config.targetWidth, config.targetHeight, config.targetFps);
            } else {
                vcamManager->Stop();
            }
        }
    });

    std::cout << "[Engine] Pipeline listo y corriendo a 60 FPS estables..." << std::endl;

    // Bucle principal de Renderizado a 60 FPS
    std::vector<uint8_t> localFrame(config.targetWidth * config.targetHeight * 4);
    std::vector<uint8_t> processedFrame;
    int frameCount = 0;
    auto lastFpsTime = std::chrono::steady_clock::now();
    float currentFps = 60.0f;
    float avgAiTime = 0.0f;
    float avgGpuTime = 0.0f;

    while (true) {
        auto frameStart = std::chrono::steady_clock::now();

        // 1. Obtener frame de cámara
        bool hasFrame = false;
        {
            std::lock_guard<std::mutex> lock(frameMutex);
            if (newFrameAvailable && !latestRawFrame.empty()) {
                memcpy(localFrame.data(), latestRawFrame.data(), localFrame.size());
                newFrameAvailable = false;
                hasFrame = true;
            }
        }

        if (hasFrame) {
            // 2. Inferencia de Inteligencia Artificial (Detección Facial + Segmentación)
            auto aiStart = std::chrono::steady_clock::now();
            DetectionResult aiResult = aiEngine->ProcessFrame(localFrame.data(), config.targetWidth, config.targetHeight, config.ai);
            auto aiEnd = std::chrono::steady_clock::now();
            avgAiTime = std::chrono::duration<float, std::milli>(aiEnd - aiStart).count();

            // 3. Subir texturas a GPU
            auto gpuStart = std::chrono::steady_clock::now();
            renderer->UploadCameraFrame(localFrame.data(), config.targetWidth, config.targetHeight);
            if (!aiResult.segmentationMask.empty()) {
                renderer->UploadSegmentationMask(aiResult.segmentationMask.data(), config.targetWidth, config.targetHeight);
            }

            // 4. Ejecutar Shaders y Renderizado GPU
            renderer->Render(config);
            renderer->PresentPreview();
            auto gpuEnd = std::chrono::steady_clock::now();
            avgGpuTime = std::chrono::duration<float, std::milli>(gpuEnd - gpuStart).count();

            // 5. Publicar a la Cámara Virtual si está activa
            if (vcamManager->IsActive()) {
                if (renderer->ReadbackProcessedFrame(processedFrame)) {
                    vcamManager->PublishFrame(processedFrame.data(), config.targetWidth, config.targetHeight);
                }
            }

            frameCount++;
        }

        // Medición de telemetría y transmisión a la UI
        auto now = std::chrono::steady_clock::now();
        float elapsedSec = std::chrono::duration<float>(now - lastFpsTime).count();
        if (elapsedSec >= 0.5f) {
            currentFps = frameCount / elapsedSec;
            frameCount = 0;
            lastFpsTime = now;

            // Enviar telemetría a la UI vía WebSocket
            std::ostringstream ss;
            ss << std::fixed << std::setprecision(1);
            ss << "{\"type\":\"TELEMETRY\","
               << "\"fps\":" << currentFps << ","
               << "\"frameTimeMs\":" << (1000.0f / (currentFps > 0 ? currentFps : 60.0f)) << ","
               << "\"aiMs\":" << avgAiTime << ","
               << "\"gpuMs\":" << avgGpuTime << ","
               << "\"vcamActive\":" << (vcamManager->IsActive() ? "true" : "false") << ","
               << "\"gpuName\":\"" << renderer->GetGpuName() << "\","
               << "\"faces\":" << 1
               << "}";
            wsServer->BroadcastText(ss.str());
        }

        // Pacing a 60 FPS
        auto frameEnd = std::chrono::steady_clock::now();
        auto frameDurationMs = std::chrono::duration_cast<std::chrono::milliseconds>(frameEnd - frameStart).count();
        if (frameDurationMs < 16) {
            std::this_thread::sleep_for(std::chrono::milliseconds(16 - frameDurationMs));
        }
    }

    return 0;
}
