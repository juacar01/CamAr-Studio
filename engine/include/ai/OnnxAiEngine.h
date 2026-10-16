#pragma once

#include "core/Types.h"
#include <vector>
#include <memory>
#include <string>
#include <mutex>

namespace CamAr {

struct DetectionResult {
    std::vector<FaceLandmarks> faces;
    std::vector<uint8_t> segmentationMask; // Tamaño: width * height (valores 0 a 255)
    float inferenceTimeMs = 0.0f;
};

class OnnxAiEngine {
public:
    OnnxAiEngine();
    ~OnnxAiEngine();

    bool Initialize(int width, int height, const std::string& modelDirectory);

    // Ejecuta la detección facial y segmentación sobre un frame RGBA
    DetectionResult ProcessFrame(const uint8_t* rgbaData, int width, int height, const AiParams& params);

    bool IsModelLoaded() const { return m_initialized; }
    std::string GetExecutionProvider() const { return m_executionProvider; }

private:
    void SmoothLandmarks(FaceLandmarks& current, const FaceLandmarks& previous, float alpha);

    bool m_initialized = false;
    int m_width = 1920;
    int m_height = 1080;
    std::string m_executionProvider = "DirectML / GPU (RTX 5060)";
    std::string m_modelsPath;

    // Buffer de máscara de segmentación cacheado
    std::vector<uint8_t> m_maskBuffer;
    FaceLandmarks m_prevFace;
    std::mutex m_mutex;
};

} // namespace CamAr
