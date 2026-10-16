#pragma once

#include "core/Types.h"
#include <d3d11.h>
#include <wrl/client.h>
#include <memory>
#include <string>

using Microsoft::WRL::ComPtr;

namespace CamAr {

struct EffectConstantBuffer {
    float brightness;
    float contrast;
    float saturation;
    float exposure;
    float temperature;
    float skinSmooth;
    int   backgroundMode;
    float blurRadius;
    float resolutionX;
    float resolutionY;
    float segThreshold;
    float edgeFeather;
    float padding[4]; // Alineación a 16 bytes requerida por D3D11
};

class D3D11Renderer {
public:
    D3D11Renderer();
    ~D3D11Renderer();

    bool Initialize(HWND previewHwnd, int width, int height);
    void Resize(int width, int height);

    // Carga de texturas desde buffers de memoria
    bool UploadCameraFrame(const uint8_t* rgbaData, int width, int height);
    bool UploadSegmentationMask(const uint8_t* maskData, int width, int height);
    bool UploadVirtualBackground(const uint8_t* rgbaData, int width, int height);

    // Ejecuta el pipeline de shaders con los parámetros activos
    bool Render(const StudioConfig& config);

    // Presenta en el HWND de la ventana de Preview
    void PresentPreview();

    // Obtiene una copia del frame final procesado (para pasar a la Cámara Virtual)
    bool ReadbackProcessedFrame(std::vector<uint8_t>& outRgba);

    // Obtiene el recurso compartido para Zero-Copy Virtual Cam
    HANDLE GetSharedTextureHandle() const { return m_sharedHandle; }

    std::string GetGpuName() const { return m_gpuName; }

private:
    bool CreateDeviceAndSwapChain(HWND hwnd);
    bool CreateShaders();
    bool CreateQuadBuffers();
    bool CreateTextures(int width, int height);

    int m_width = 1920;
    int m_height = 1080;
    HWND m_hwnd = nullptr;
    std::string m_gpuName = "Unknown GPU";
    HANDLE m_sharedHandle = nullptr;

    ComPtr<ID3D11Device> m_device;
    ComPtr<ID3D11DeviceContext> m_context;
    ComPtr<IDXGISwapChain> m_swapChain;
    ComPtr<ID3D11RenderTargetView> m_renderTargetView;

    ComPtr<ID3D11VertexShader> m_vertexShader;
    ComPtr<ID3D11PixelShader> m_pixelShader;
    ComPtr<ID3D11InputLayout> m_inputLayout;
    ComPtr<ID3D11Buffer> m_vertexBuffer;
    ComPtr<ID3D11Buffer> m_constantBuffer;
    ComPtr<ID3D11SamplerState> m_samplerState;

    // Texturas de entrada y render
    ComPtr<ID3D11Texture2D> m_mainTexture;
    ComPtr<ID3D11ShaderResourceView> m_mainSRV;

    ComPtr<ID3D11Texture2D> m_maskTexture;
    ComPtr<ID3D11ShaderResourceView> m_maskSRV;

    ComPtr<ID3D11Texture2D> m_bgTexture;
    ComPtr<ID3D11ShaderResourceView> m_bgSRV;

    // Textura de salida procesada compartida y de staging (CPU readback)
    ComPtr<ID3D11Texture2D> m_outputTexture;
    ComPtr<ID3D11RenderTargetView> m_outputRTV;
    ComPtr<ID3D11Texture2D> m_stagingTexture;
};

} // namespace CamAr
