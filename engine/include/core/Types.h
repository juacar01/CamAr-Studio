#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace CamAr {

enum class PixelFormat {
    RGBA32,
    BGRA32,
    NV12,
    YUY2
};

enum class BackgroundMode {
    NONE = 0,
    BLUR = 1,
    IMAGE = 2,
    GREEN_SCREEN = 3,
    COLOR = 4
};

struct Point2f {
    float x;
    float y;
};

struct Rect2f {
    float x;
    float y;
    float width;
    float height;
};

struct FaceLandmarks {
    Rect2f boundingBox;
    float confidence;
    std::vector<Point2f> landmarks; // 6 landmarks principales (ojos, nariz, boca) o 468 mesh
    bool detected = false;
};

struct ColorCorrectionParams {
    float brightness = 0.0f;     // -1.0 a 1.0 (default: 0.0)
    float contrast = 1.0f;       // 0.0 a 2.0 (default: 1.0)
    float saturation = 1.0f;     // 0.0 a 2.0 (default: 1.0)
    float exposure = 0.0f;       // -2.0 a 2.0 (default: 0.0)
    float temperature = 0.0f;    // -1.0 a 1.0 (calidez)
    float sharpness = 0.0f;      // 0.0 a 1.0
};

struct BeautifyParams {
    bool enabled = false;
    float skinSmoothing = 0.0f;  // 0.0 a 1.0 (filtro bilateral de piel)
    float skinWhitening = 0.0f;  // 0.0 a 1.0
    float eyeEnlarge = 0.0f;     // 0.0 a 1.0 (deformación de malla facial)
    float faceSlim = 0.0f;       // 0.0 a 1.0 (adelgazamiento de mejillas)
};

struct BackgroundParams {
    BackgroundMode mode = BackgroundMode::NONE;
    float blurRadius = 15.0f;    // Para modo BLUR (1.0 a 50.0)
    std::string customImagePath; // Para modo IMAGE
    float greenScreenThreshold = 0.4f;
    float edgeFeather = 1.5f;     // Suavizado de bordes de la máscara
};

struct AiParams {
    bool faceDetectionEnabled = true;
    bool personSegmentationEnabled = true;
    float faceConfidenceThreshold = 0.5f;
    float segmentationThreshold = 0.5f;
    bool drawFaceLandmarks = false;
    bool drawFaceBox = false;
};

struct StudioConfig {
    int targetWidth = 1920;
    int targetHeight = 1080;
    int targetFps = 60;
    std::string selectedCameraId = "";
    bool virtualCamActive = false;

    ColorCorrectionParams color;
    BeautifyParams beauty;
    BackgroundParams background;
    AiParams ai;
};

struct EngineStats {
    float currentFps = 0.0f;
    float frameTimeMs = 0.0f;
    float aiInferenceTimeMs = 0.0f;
    float gpuProcessingTimeMs = 0.0f;
    int facesDetected = 0;
    bool virtualCamRunning = false;
    std::string activeGpu = "";
};

} // namespace CamAr
