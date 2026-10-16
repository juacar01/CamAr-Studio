#pragma once

#include "core/Types.h"
#include <string>
#include <vector>
#include <functional>
#include <atomic>
#include <thread>
#include <mutex>
#include <mfapi.h>
#include <mfidl.h>
#include <mfreadwrite.h>

namespace CamAr {

struct CameraDeviceInfo {
    std::string id;
    std::string name;
    std::vector<std::string> supportedResolutions;
};

using FrameCallback = std::function<void(const uint8_t* rgbaData, int width, int height)>;

class CameraCapture {
public:
    CameraCapture();
    ~CameraCapture();

    static std::vector<CameraDeviceInfo> EnumerateCameras();

    bool Start(const std::string& deviceId, int width, int height, int fps, FrameCallback callback);
    void Stop();

    bool IsRunning() const { return m_running; }
    int GetWidth() const { return m_width; }
    int GetHeight() const { return m_height; }

private:
    void CaptureLoop();
    void GenerateSyntheticFrame(std::vector<uint8_t>& buffer, int width, int height, float timeSeconds);

    std::atomic<bool> m_running{false};
    std::thread m_workerThread;
    std::mutex m_mutex;

    int m_width = 1920;
    int m_height = 1080;
    int m_fps = 60;
    std::string m_deviceId;
    FrameCallback m_callback;

    bool m_useSyntheticFallback = false;
    IMFSourceReader* m_sourceReader = nullptr;
};

} // namespace CamAr
