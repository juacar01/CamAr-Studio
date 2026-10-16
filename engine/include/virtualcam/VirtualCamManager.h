#pragma once

#include "core/Types.h"
#include <windows.h>
#include <string>
#include <vector>
#include <mutex>
#include <atomic>

namespace CamAr {

struct SharedFrameHeader {
    uint32_t magic;         // 0x43414D41 ("CAMA")
    uint32_t version;       // 1
    uint32_t width;
    uint32_t height;
    uint32_t pixelFormat;   // 0: RGBA32, 1: NV12
    uint64_t timestamp;
    uint32_t frameIndex;
    uint32_t dataSize;
};

class VirtualCamManager {
public:
    VirtualCamManager();
    ~VirtualCamManager();

    bool Start(int width, int height, int fps);
    void Stop();

    // Envía el fotograma procesado a la memoria compartida de la cámara virtual
    bool PublishFrame(const uint8_t* rgbaData, int width, int height);

    bool IsActive() const { return m_active; }
    std::string GetDeviceName() const { return m_deviceName; }

private:
    bool CreateSharedMemory(int width, int height);
    void CloseSharedMemory();

    std::string m_deviceName = "CamAr Studio Virtual Camera";
    std::string m_sharedMemoryName = "Local\\CamAr_VirtualCam_SharedMem";
    std::string m_syncEventName = "Local\\CamAr_VirtualCam_SyncEvent";

    int m_width = 1920;
    int m_height = 1080;
    int m_fps = 60;
    std::atomic<bool> m_active{false};
    uint32_t m_frameIndex = 0;

    HANDLE m_fileMapping = nullptr;
    void* m_sharedBuffer = nullptr;
    HANDLE m_syncEvent = nullptr;
    std::mutex m_mutex;
};

} // namespace CamAr
