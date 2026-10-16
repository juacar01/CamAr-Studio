#include "capture/CameraCapture.h"
#include <iostream>
#include <chrono>
#include <cmath>
#include <algorithm>

#pragma comment(lib, "mf.lib")
#pragma comment(lib, "mfplat.lib")
#pragma comment(lib, "mfreadwrite.lib")
#pragma comment(lib, "mfuuid.lib")

namespace CamAr {

CameraCapture::CameraCapture() {
    MFStartup(MF_VERSION);
}

CameraCapture::~CameraCapture() {
    Stop();
    MFShutdown();
}

std::vector<CameraDeviceInfo> CameraCapture::EnumerateCameras() {
    std::vector<CameraDeviceInfo> devices;

    // Dispositivo Sintético siempre disponible para pruebas y desarrollo
    devices.push_back({ "synthetic_camera", "CamAr Studio Simulator (Test Pattern & Avatar)", { "1920x1080 @ 60fps", "1280x720 @ 60fps" } });

    IMFAttributes* attributes = nullptr;
    HRESULT hr = MFCreateAttributes(&attributes, 1);
    if (SUCCEEDED(hr)) {
        hr = attributes->SetGUID(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID);
        if (SUCCEEDED(hr)) {
            IMFActivate** ppDevices = nullptr;
            UINT32 count = 0;
            hr = MFEnumDeviceSources(attributes, &ppDevices, &count);
            if (SUCCEEDED(hr) && count > 0) {
                for (UINT32 i = 0; i < count; ++i) {
                    WCHAR* friendlyName = nullptr;
                    UINT32 nameLen = 0;
                    WCHAR* symLink = nullptr;
                    UINT32 symLen = 0;

                    ppDevices[i]->GetAllocatedString(MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME, &friendlyName, &nameLen);
                    ppDevices[i]->GetAllocatedString(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK, &symLink, &symLen);

                    if (friendlyName && symLink) {
                        char nameA[256];
                        char idA[512];
                        WideCharToMultiByte(CP_UTF8, 0, friendlyName, -1, nameA, sizeof(nameA), nullptr, nullptr);
                        WideCharToMultiByte(CP_UTF8, 0, symLink, -1, idA, sizeof(idA), nullptr, nullptr);

                        devices.push_back({ std::string(idA), std::string(nameA), { "1920x1080 @ 30fps", "1280x720 @ 60fps" } });
                    }

                    if (friendlyName) CoTaskMemFree(friendlyName);
                    if (symLink) CoTaskMemFree(symLink);
                    ppDevices[i]->Release();
                }
                CoTaskMemFree(ppDevices);
            }
        }
        attributes->Release();
    }

    return devices;
}

bool CameraCapture::Start(const std::string& deviceId, int width, int height, int fps, FrameCallback callback) {
    Stop();

    m_deviceId = deviceId;
    m_width = width;
    m_height = height;
    m_fps = fps;
    m_callback = callback;

    if (deviceId.empty() || deviceId == "synthetic_camera") {
        m_useSyntheticFallback = true;
    } else {
        // Inicializar Media Foundation Source Reader
        IMFAttributes* attributes = nullptr;
        MFCreateAttributes(&attributes, 2);
        attributes->SetGUID(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE, MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_GUID);

        int wideLen = MultiByteToWideChar(CP_UTF8, 0, deviceId.c_str(), -1, nullptr, 0);
        std::vector<wchar_t> wideSym(wideLen);
        MultiByteToWideChar(CP_UTF8, 0, deviceId.c_str(), -1, wideSym.data(), wideLen);

        attributes->SetString(MF_DEVSOURCE_ATTRIBUTE_SOURCE_TYPE_VIDCAP_SYMBOLIC_LINK, wideSym.data());

        IMFMediaSource* mediaSource = nullptr;
        HRESULT hr = MFCreateDeviceSource(attributes, &mediaSource);
        attributes->Release();

        if (SUCCEEDED(hr)) {
            IMFAttributes* readerAttr = nullptr;
            MFCreateAttributes(&readerAttr, 1);
            readerAttr->SetUINT32(MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, TRUE);

            hr = MFCreateSourceReaderFromMediaSource(mediaSource, readerAttr, &m_sourceReader);
            readerAttr->Release();
            mediaSource->Release();

            if (SUCCEEDED(hr)) {
                // Configurar tipo de medio de salida a RGB32
                IMFMediaType* mediaType = nullptr;
                MFCreateMediaType(&mediaType);
                mediaType->SetGUID(MF_MT_MAJOR_TYPE, MFMediaType_Video);
                mediaType->SetGUID(MF_MT_SUBTYPE, MFVideoFormat_RGB32);
                MFSetAttributeSize(mediaType, MF_MT_FRAME_SIZE, width, height);

                hr = m_sourceReader->SetCurrentMediaType((DWORD)MF_SOURCE_READER_FIRST_VIDEO_STREAM, nullptr, mediaType);
                mediaType->Release();
                m_useSyntheticFallback = FAILED(hr);
            } else {
                m_useSyntheticFallback = true;
            }
        } else {
            m_useSyntheticFallback = true;
        }
    }

    m_running = true;
    m_workerThread = std::thread(&CameraCapture::CaptureLoop, this);
    std::cout << "[CameraCapture] Stream iniciado en " << m_width << "x" << m_height << " @" << m_fps << " FPS (Modo: " 
              << (m_useSyntheticFallback ? "Simulador Realista" : "Hardware Directo") << ")" << std::endl;
    return true;
}

void CameraCapture::Stop() {
    m_running = false;
    if (m_workerThread.joinable()) {
        m_workerThread.join();
    }
    if (m_sourceReader) {
        m_sourceReader->Release();
        m_sourceReader = nullptr;
    }
}

void CameraCapture::CaptureLoop() {
    std::vector<uint8_t> frameBuffer(m_width * m_height * 4);
    auto startTime = std::chrono::steady_clock::now();
    int frameIntervalMs = 1000 / m_fps;

    while (m_running) {
        auto frameStart = std::chrono::steady_clock::now();

        if (m_useSyntheticFallback) {
            auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(frameStart - startTime).count();
            float timeSec = elapsed / 1000.0f;
            GenerateSyntheticFrame(frameBuffer, m_width, m_height, timeSec);

            if (m_callback) {
                m_callback(frameBuffer.data(), m_width, m_height);
            }
        } else if (m_sourceReader) {
            DWORD streamIndex = 0;
            DWORD flags = 0;
            LONGLONG timestamp = 0;
            IMFSample* sample = nullptr;

            HRESULT hr = m_sourceReader->ReadSample(
                (DWORD)MF_SOURCE_READER_FIRST_VIDEO_STREAM,
                0,
                &streamIndex,
                &flags,
                &timestamp,
                &sample
            );

            if (SUCCEEDED(hr) && sample) {
                IMFMediaBuffer* buffer = nullptr;
                if (SUCCEEDED(sample->ConvertToContiguousBuffer(&buffer))) {
                    BYTE* data = nullptr;
                    DWORD maxLen = 0, currentLen = 0;
                    if (SUCCEEDED(buffer->Lock(&data, &maxLen, &currentLen))) {
                        if (currentLen >= (DWORD)(m_width * m_height * 4)) {
                            // RGB32 en Windows suele ser B, G, R, A -> convertimos si es necesario o pasamos directo
                            if (m_callback) {
                                m_callback(data, m_width, m_height);
                            }
                        }
                        buffer->Unlock();
                    }
                    buffer->Release();
                }
                sample->Release();
            }
        }

        auto frameEnd = std::chrono::steady_clock::now();
        auto elapsedMs = std::chrono::duration_cast<std::chrono::milliseconds>(frameEnd - frameStart).count();
        if (elapsedMs < frameIntervalMs) {
            std::this_thread::sleep_for(std::chrono::milliseconds(frameIntervalMs - elapsedMs));
        }
    }
}

void CameraCapture::GenerateSyntheticFrame(std::vector<uint8_t>& buffer, int width, int height, float timeSeconds) {
    // Genera un patrón realista tipo streamer: fondo con degradado de estudio y avatar humanoide centrado en movimiento sutil
    float headCenterX = width * (0.5f + 0.04f * std::sin(timeSeconds * 1.5f));
    float headCenterY = height * (0.42f + 0.02f * std::cos(timeSeconds * 2.0f));
    float headRadius = height * 0.18f;

    for (int y = 0; y < height; ++y) {
        float ny = (float)y / height;
        uint8_t* row = &buffer[y * width * 4];

        for (int x = 0; x < width; ++x) {
            float nx = (float)x / width;
            int idx = x * 4;

            // Fondo: Iluminación de estudio RGB moderna (Cyberpunk / Studio Neon)
            float bgR = (0.15f + 0.1f * std::sin(nx * 3.14f + timeSeconds * 0.5f));
            float bgG = (0.10f + 0.05f * std::cos(ny * 3.14f));
            float bgB = (0.35f + 0.15f * std::sin((nx + ny) * 3.14f));

            // Cuadrícula sutil de fondo
            if ((x % 80 == 0) || (y % 80 == 0)) {
                bgR += 0.08f;
                bgG += 0.08f;
                bgB += 0.12f;
            }

            // Cuerpo y Cabeza de la persona simulada
            float dx = x - headCenterX;
            float dy = y - headCenterY;
            float distHead = std::sqrt(dx * dx + dy * dy);

            // Torso (Trapezoide inferior)
            float torsoY = y - (headCenterY + headRadius * 0.8f);
            float torsoWidth = (torsoY > 0) ? (headRadius * 1.2f + torsoY * 0.8f) : 0;
            bool inTorso = (torsoY > 0 && std::abs(dx) < torsoWidth && y < height);

            if (distHead < headRadius) {
                // Cabeza / Rostro (Tono piel realista)
                float skinShade = 1.0f - (distHead / headRadius) * 0.3f;
                row[idx + 0] = (uint8_t)(245 * skinShade); // R
                row[idx + 1] = (uint8_t)(195 * skinShade); // G
                row[idx + 2] = (uint8_t)(170 * skinShade); // B
                row[idx + 3] = 255;                         // A

                // Ojos
                float eyeY = headCenterY - headRadius * 0.15f;
                float leftEyeX = headCenterX - headRadius * 0.35f;
                float rightEyeX = headCenterX + headRadius * 0.35f;
                float dEyeL = std::sqrt((x - leftEyeX)*(x - leftEyeX) + (y - eyeY)*(y - eyeY));
                float dEyeR = std::sqrt((x - rightEyeX)*(x - rightEyeX) + (y - eyeY)*(y - eyeY));
                if (dEyeL < headRadius * 0.12f || dEyeR < headRadius * 0.12f) {
                    row[idx + 0] = 30; row[idx + 1] = 40; row[idx + 2] = 50;
                }
                // Labios
                float mouthY = headCenterY + headRadius * 0.45f;
                if (std::abs(y - mouthY) < headRadius * 0.08f && std::abs(dx) < headRadius * 0.25f) {
                    row[idx + 0] = 200; row[idx + 1] = 70; row[idx + 2] = 80;
                }
            } else if (inTorso) {
                // Ropa / Camisa azul oscuro
                row[idx + 0] = 45;
                row[idx + 1] = 60;
                row[idx + 2] = 95;
                row[idx + 3] = 255;
            } else {
                // Fondo
                row[idx + 0] = (uint8_t)((std::min)(1.0f, bgR) * 255);
                row[idx + 1] = (uint8_t)((std::min)(1.0f, bgG) * 255);
                row[idx + 2] = (uint8_t)((std::min)(1.0f, bgB) * 255);
                row[idx + 3] = 255;
            }
        }
    }
}

} // namespace CamAr
