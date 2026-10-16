#include "virtualcam/VirtualCamManager.h"
#include <iostream>
#include <chrono>

namespace CamAr {

VirtualCamManager::VirtualCamManager() = default;

VirtualCamManager::~VirtualCamManager() {
    Stop();
}

bool VirtualCamManager::Start(int width, int height, int fps) {
    std::lock_guard<std::mutex> lock(m_mutex);
    if (m_active) return true;

    m_width = width;
    m_height = height;
    m_fps = fps;

    if (!CreateSharedMemory(m_width, m_height)) {
        std::cerr << "[VirtualCam] Error al crear la memoria compartida del driver." << std::endl;
        return false;
    }

    m_active = true;
    m_frameIndex = 0;
    std::cout << "[VirtualCam] Dispositivo '" << m_deviceName << "' publicado a " << m_width << "x" << m_height << " @" << m_fps << " FPS" << std::endl;
    return true;
}

void VirtualCamManager::Stop() {
    std::lock_guard<std::mutex> lock(m_mutex);
    if (!m_active) return;

    m_active = false;
    CloseSharedMemory();
    std::cout << "[VirtualCam] Dispositivo detenido." << std::endl;
}

bool VirtualCamManager::CreateSharedMemory(int width, int height) {
    CloseSharedMemory();

    size_t dataSize = width * height * 4;
    size_t totalSize = sizeof(SharedFrameHeader) + dataSize;

    // Crear File Mapping en memoria paginada de Windows
    m_fileMapping = CreateFileMappingA(
        INVALID_HANDLE_VALUE,
        nullptr,
        PAGE_READWRITE,
        0,
        (DWORD)totalSize,
        m_sharedMemoryName.c_str()
    );

    if (!m_fileMapping) {
        return false;
    }

    m_sharedBuffer = MapViewOfFile(
        m_fileMapping,
        FILE_MAP_ALL_ACCESS,
        0,
        0,
        totalSize
    );

    if (!m_sharedBuffer) {
        CloseHandle(m_fileMapping);
        m_fileMapping = nullptr;
        return false;
    }

    // Evento de sincronización con señal de pulso
    m_syncEvent = CreateEventA(nullptr, FALSE, FALSE, m_syncEventName.c_str());

    // Inicializar cabecera
    SharedFrameHeader* header = (SharedFrameHeader*)m_sharedBuffer;
    header->magic = 0x43414D41; // "CAMA"
    header->version = 1;
    header->width = width;
    header->height = height;
    header->pixelFormat = 0; // RGBA32
    header->dataSize = (uint32_t)dataSize;
    header->frameIndex = 0;
    header->timestamp = 0;

    return true;
}

void VirtualCamManager::CloseSharedMemory() {
    if (m_sharedBuffer) {
        UnmapViewOfFile(m_sharedBuffer);
        m_sharedBuffer = nullptr;
    }
    if (m_fileMapping) {
        CloseHandle(m_fileMapping);
        m_fileMapping = nullptr;
    }
    if (m_syncEvent) {
        CloseHandle(m_syncEvent);
        m_syncEvent = nullptr;
    }
}

bool VirtualCamManager::PublishFrame(const uint8_t* rgbaData, int width, int height) {
    if (!m_active || !m_sharedBuffer || !rgbaData) return false;

    std::lock_guard<std::mutex> lock(m_mutex);
    SharedFrameHeader* header = (SharedFrameHeader*)m_sharedBuffer;

    if (header->width != (uint32_t)width || header->height != (uint32_t)height) {
        return false;
    }

    uint8_t* pixelDest = (uint8_t*)m_sharedBuffer + sizeof(SharedFrameHeader);
    size_t dataSize = width * height * 4;

    // Copia ultra rápida de memoria
    memcpy(pixelDest, rgbaData, dataSize);

    auto now = std::chrono::steady_clock::now().time_since_epoch();
    header->timestamp = std::chrono::duration_cast<std::chrono::microseconds>(now).count();
    header->frameIndex = ++m_frameIndex;

    // Notificar a los clientes de videollamada / streaming que hay un nuevo frame listo
    if (m_syncEvent) {
        SetEvent(m_syncEvent);
    }

    return true;
}

} // namespace CamAr
