#include "processing/D3D11Renderer.h"
#include <d3dcompiler.h>
#include <dxgi.h>
#include <iostream>
#include <fstream>
#include <sstream>

#pragma comment(lib, "d3d11.lib")
#pragma comment(lib, "dxgi.lib")
#pragma comment(lib, "d3dcompiler.lib")

namespace CamAr {

struct Vertex {
    float x, y, z;
    float u, v;
};

D3D11Renderer::D3D11Renderer() = default;

D3D11Renderer::~D3D11Renderer() {
    if (m_sharedHandle) {
        CloseHandle(m_sharedHandle);
        m_sharedHandle = nullptr;
    }
}

bool D3D11Renderer::Initialize(HWND previewHwnd, int width, int height) {
    m_hwnd = previewHwnd;
    m_width = width;
    m_height = height;

    if (!CreateDeviceAndSwapChain(m_hwnd)) {
        std::cerr << "[D3D11Renderer] Error al crear D3D11 Device y SwapChain." << std::endl;
        return false;
    }

    if (!CreateShaders()) {
        std::cerr << "[D3D11Renderer] Error al compilar o crear shaders." << std::endl;
        return false;
    }

    if (!CreateQuadBuffers()) {
        std::cerr << "[D3D11Renderer] Error al crear buffers de geometría." << std::endl;
        return false;
    }

    if (!CreateTextures(m_width, m_height)) {
        std::cerr << "[D3D11Renderer] Error al crear texturas de procesamiento." << std::endl;
        return false;
    }

    std::cout << "[D3D11Renderer] Inicializado con éxito en GPU: " << m_gpuName << " (" << m_width << "x" << m_height << ")" << std::endl;
    return true;
}

bool D3D11Renderer::CreateDeviceAndSwapChain(HWND hwnd) {
    DXGI_SWAP_CHAIN_DESC scd = {};
    scd.BufferCount = 2;
    scd.BufferDesc.Width = m_width;
    scd.BufferDesc.Height = m_height;
    scd.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    scd.BufferDesc.RefreshRate.Numerator = 60;
    scd.BufferDesc.RefreshRate.Denominator = 1;
    scd.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    scd.OutputWindow = hwnd ? hwnd : GetDesktopWindow();
    scd.SampleDesc.Count = 1;
    scd.SampleDesc.Quality = 0;
    scd.Windowed = TRUE;
    scd.SwapEffect = DXGI_SWAP_EFFECT_DISCARD;

    UINT createDeviceFlags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
#if defined(_DEBUG)
    createDeviceFlags |= D3D11_CREATE_DEVICE_DEBUG;
#endif

    D3D_FEATURE_LEVEL featureLevels[] = {
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0
    };
    D3D_FEATURE_LEVEL featureLevel;

    HRESULT hr = D3D11CreateDeviceAndSwapChain(
        nullptr,
        D3D_DRIVER_TYPE_HARDWARE,
        nullptr,
        createDeviceFlags,
        featureLevels,
        ARRAYSIZE(featureLevels),
        D3D11_SDK_VERSION,
        &scd,
        m_swapChain.GetAddressOf(),
        m_device.GetAddressOf(),
        &featureLevel,
        m_context.GetAddressOf()
    );

    if (FAILED(hr)) {
        // Fallback a WARP si no hay hardware driver
        hr = D3D11CreateDeviceAndSwapChain(
            nullptr,
            D3D_DRIVER_TYPE_WARP,
            nullptr,
            createDeviceFlags,
            featureLevels,
            ARRAYSIZE(featureLevels),
            D3D11_SDK_VERSION,
            &scd,
            m_swapChain.GetAddressOf(),
            m_device.GetAddressOf(),
            &featureLevel,
            m_context.GetAddressOf()
        );
        if (FAILED(hr)) return false;
    }

    // Obtener nombre de la GPU
    ComPtr<IDXGIDevice> dxgiDevice;
    if (SUCCEEDED(m_device.As(&dxgiDevice))) {
        ComPtr<IDXGIAdapter> adapter;
        if (SUCCEEDED(dxgiDevice->GetAdapter(adapter.GetAddressOf()))) {
            DXGI_ADAPTER_DESC desc;
            if (SUCCEEDED(adapter->GetDesc(&desc))) {
                char ch[128];
                char defChar = ' ';
                WideCharToMultiByte(CP_ACP, 0, desc.Description, -1, ch, 128, &defChar, NULL);
                m_gpuName = std::string(ch);
            }
        }
    }

    // Back buffer RTV
    ComPtr<ID3D11Texture2D> backBuffer;
    if (SUCCEEDED(m_swapChain->GetBuffer(0, __uuidof(ID3D11Texture2D), (void**)backBuffer.GetAddressOf()))) {
        m_device->CreateRenderTargetView(backBuffer.Get(), nullptr, m_renderTargetView.GetAddressOf());
    }

    return true;
}

static std::string ReadShaderFile(const std::string& path) {
    std::ifstream file(path);
    if (!file.is_open()) return "";
    std::stringstream buffer;
    buffer << file.rdbuf();
    return buffer.str();
}

bool D3D11Renderer::CreateShaders() {
    ComPtr<ID3DBlob> vsBlob;
    ComPtr<ID3DBlob> psBlob;
    ComPtr<ID3DBlob> errorBlob;

    std::string vsCode = ReadShaderFile("shaders/VertexShader.hlsl");
    std::string psCode = ReadShaderFile("shaders/VideoProcess.hlsl");

    if (vsCode.empty() || psCode.empty()) {
        std::cerr << "[D3D11Renderer] No se pudieron cargar los archivos HLSL de shaders/." << std::endl;
        return false;
    }

    HRESULT hr = D3DCompile(vsCode.c_str(), vsCode.size(), nullptr, nullptr, nullptr, "VSMain", "vs_5_0", 0, 0, vsBlob.GetAddressOf(), errorBlob.GetAddressOf());
    if (FAILED(hr)) {
        if (errorBlob) std::cerr << "VS Error: " << (char*)errorBlob->GetBufferPointer() << std::endl;
        return false;
    }

    hr = D3DCompile(psCode.c_str(), psCode.size(), nullptr, nullptr, nullptr, "PSMain", "ps_5_0", 0, 0, psBlob.GetAddressOf(), errorBlob.GetAddressOf());
    if (FAILED(hr)) {
        if (errorBlob) std::cerr << "PS Error: " << (char*)errorBlob->GetBufferPointer() << std::endl;
        return false;
    }

    m_device->CreateVertexShader(vsBlob->GetBufferPointer(), vsBlob->GetBufferSize(), nullptr, m_vertexShader.GetAddressOf());
    m_device->CreatePixelShader(psBlob->GetBufferPointer(), psBlob->GetBufferSize(), nullptr, m_pixelShader.GetAddressOf());

    D3D11_INPUT_ELEMENT_DESC layout[] = {
        { "POSITION", 0, DXGI_FORMAT_R32G32B32_FLOAT, 0, 0, D3D11_INPUT_PER_VERTEX_DATA, 0 },
        { "TEXCOORD", 0, DXGI_FORMAT_R32G32_FLOAT, 0, 12, D3D11_INPUT_PER_VERTEX_DATA, 0 },
    };
    m_device->CreateInputLayout(layout, ARRAYSIZE(layout), vsBlob->GetBufferPointer(), vsBlob->GetBufferSize(), m_inputLayout.GetAddressOf());

    // Sampler State lineal
    D3D11_SAMPLER_DESC sampDesc = {};
    sampDesc.Filter = D3D11_FILTER_MIN_MAG_MIP_LINEAR;
    sampDesc.AddressU = D3D11_TEXTURE_ADDRESS_CLAMP;
    sampDesc.AddressV = D3D11_TEXTURE_ADDRESS_CLAMP;
    sampDesc.AddressW = D3D11_TEXTURE_ADDRESS_CLAMP;
    sampDesc.ComparisonFunc = D3D11_COMPARISON_NEVER;
    m_device->CreateSamplerState(&sampDesc, m_samplerState.GetAddressOf());

    // Constant Buffer para parámetros
    D3D11_BUFFER_DESC cbDesc = {};
    cbDesc.ByteWidth = sizeof(EffectConstantBuffer);
    cbDesc.Usage = D3D11_USAGE_DYNAMIC;
    cbDesc.BindFlags = D3D11_BIND_CONSTANT_BUFFER;
    cbDesc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;
    m_device->CreateBuffer(&cbDesc, nullptr, m_constantBuffer.GetAddressOf());

    return true;
}

bool D3D11Renderer::CreateQuadBuffers() {
    Vertex vertices[] = {
        { -1.0f,  1.0f, 0.0f, 0.0f, 0.0f },
        {  1.0f,  1.0f, 0.0f, 1.0f, 0.0f },
        { -1.0f, -1.0f, 0.0f, 0.0f, 1.0f },
        {  1.0f, -1.0f, 0.0f, 1.0f, 1.0f },
    };

    D3D11_BUFFER_DESC bd = {};
    bd.Usage = D3D11_USAGE_DEFAULT;
    bd.ByteWidth = sizeof(vertices);
    bd.BindFlags = D3D11_BIND_VERTEX_BUFFER;

    D3D11_SUBRESOURCE_DATA initData = {};
    initData.pSysMem = vertices;

    HRESULT hr = m_device->CreateBuffer(&bd, &initData, m_vertexBuffer.GetAddressOf());
    return SUCCEEDED(hr);
}

bool D3D11Renderer::CreateTextures(int width, int height) {
    // 1. Textura de Cámara Principal (RGBA32)
    D3D11_TEXTURE2D_DESC texDesc = {};
    texDesc.Width = width;
    texDesc.Height = height;
    texDesc.MipLevels = 1;
    texDesc.ArraySize = 1;
    texDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    texDesc.SampleDesc.Count = 1;
    texDesc.Usage = D3D11_USAGE_DYNAMIC;
    texDesc.BindFlags = D3D11_BIND_SHADER_RESOURCE;
    texDesc.CPUAccessFlags = D3D11_CPU_ACCESS_WRITE;

    m_device->CreateTexture2D(&texDesc, nullptr, m_mainTexture.GetAddressOf());
    m_device->CreateShaderResourceView(m_mainTexture.Get(), nullptr, m_mainSRV.GetAddressOf());

    // 2. Textura de Máscara de Segmentación
    texDesc.Format = DXGI_FORMAT_R8_UNORM;
    m_device->CreateTexture2D(&texDesc, nullptr, m_maskTexture.GetAddressOf());
    m_device->CreateShaderResourceView(m_maskTexture.Get(), nullptr, m_maskSRV.GetAddressOf());

    // 3. Textura de Fondo Virtual
    texDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    m_device->CreateTexture2D(&texDesc, nullptr, m_bgTexture.GetAddressOf());
    m_device->CreateShaderResourceView(m_bgTexture.Get(), nullptr, m_bgSRV.GetAddressOf());

    // 4. Textura de Salida Procesada (Con RenderTarget y Keyed Mutex / Shared Handle para la Virtual Cam)
    D3D11_TEXTURE2D_DESC outDesc = {};
    outDesc.Width = width;
    outDesc.Height = height;
    outDesc.MipLevels = 1;
    outDesc.ArraySize = 1;
    outDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    outDesc.SampleDesc.Count = 1;
    outDesc.Usage = D3D11_USAGE_DEFAULT;
    outDesc.BindFlags = D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE;
    outDesc.MiscFlags = D3D11_RESOURCE_MISC_SHARED;

    HRESULT hr = m_device->CreateTexture2D(&outDesc, nullptr, m_outputTexture.GetAddressOf());
    if (SUCCEEDED(hr)) {
        m_device->CreateRenderTargetView(m_outputTexture.Get(), nullptr, m_outputRTV.GetAddressOf());
        ComPtr<IDXGIResource> dxgiResource;
        if (SUCCEEDED(m_outputTexture.As(&dxgiResource))) {
            dxgiResource->GetSharedHandle(&m_sharedHandle);
        }
    }

    // 5. Staging Texture (Para lectura CPU rápida si se requiere)
    D3D11_TEXTURE2D_DESC stagingDesc = outDesc;
    stagingDesc.Usage = D3D11_USAGE_STAGING;
    stagingDesc.BindFlags = 0;
    stagingDesc.MiscFlags = 0;
    stagingDesc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    m_device->CreateTexture2D(&stagingDesc, nullptr, m_stagingTexture.GetAddressOf());

    return true;
}

bool D3D11Renderer::UploadCameraFrame(const uint8_t* rgbaData, int width, int height) {
    if (!rgbaData || !m_mainTexture) return false;
    D3D11_MAPPED_SUBRESOURCE mapped;
    HRESULT hr = m_context->Map(m_mainTexture.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
    if (SUCCEEDED(hr)) {
        const uint8_t* src = rgbaData;
        uint8_t* dst = (uint8_t*)mapped.pData;
        int rowBytes = width * 4;
        for (int y = 0; y < height; ++y) {
            memcpy(dst, src, rowBytes);
            src += rowBytes;
            dst += mapped.RowPitch;
        }
        m_context->Unmap(m_mainTexture.Get(), 0);
        return true;
    }
    return false;
}

bool D3D11Renderer::UploadSegmentationMask(const uint8_t* maskData, int width, int height) {
    if (!maskData || !m_maskTexture) return false;
    D3D11_MAPPED_SUBRESOURCE mapped;
    HRESULT hr = m_context->Map(m_maskTexture.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
    if (SUCCEEDED(hr)) {
        const uint8_t* src = maskData;
        uint8_t* dst = (uint8_t*)mapped.pData;
        for (int y = 0; y < height; ++y) {
            memcpy(dst, src, width);
            src += width;
            dst += mapped.RowPitch;
        }
        m_context->Unmap(m_maskTexture.Get(), 0);
        return true;
    }
    return false;
}

bool D3D11Renderer::UploadVirtualBackground(const uint8_t* rgbaData, int width, int height) {
    if (!rgbaData || !m_bgTexture) return false;
    D3D11_MAPPED_SUBRESOURCE mapped;
    HRESULT hr = m_context->Map(m_bgTexture.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped);
    if (SUCCEEDED(hr)) {
        const uint8_t* src = rgbaData;
        uint8_t* dst = (uint8_t*)mapped.pData;
        int rowBytes = width * 4;
        for (int y = 0; y < height; ++y) {
            memcpy(dst, src, rowBytes);
            src += rowBytes;
            dst += mapped.RowPitch;
        }
        m_context->Unmap(m_bgTexture.Get(), 0);
        return true;
    }
    return false;
}

bool D3D11Renderer::Render(const StudioConfig& config) {
    if (!m_context) return false;

    // 1. Actualizar Constant Buffer con los parámetros de la UI
    D3D11_MAPPED_SUBRESOURCE mapped;
    if (SUCCEEDED(m_context->Map(m_constantBuffer.Get(), 0, D3D11_MAP_WRITE_DISCARD, 0, &mapped))) {
        EffectConstantBuffer* cb = (EffectConstantBuffer*)mapped.pData;
        cb->brightness = config.color.brightness;
        cb->contrast = config.color.contrast;
        cb->saturation = config.color.saturation;
        cb->exposure = config.color.exposure;
        cb->temperature = config.color.temperature;
        cb->skinSmooth = config.beauty.enabled ? config.beauty.skinSmoothing : 0.0f;
        cb->backgroundMode = static_cast<int>(config.background.mode);
        cb->blurRadius = config.background.blurRadius;
        cb->resolutionX = static_cast<float>(m_width);
        cb->resolutionY = static_cast<float>(m_height);
        cb->segThreshold = config.ai.segmentationThreshold;
        cb->edgeFeather = config.background.edgeFeather;
        m_context->Unmap(m_constantBuffer.Get(), 0);
    }

    // Configurar Viewport
    D3D11_VIEWPORT vp = {};
    vp.Width = (float)m_width;
    vp.Height = (float)m_height;
    vp.MinDepth = 0.0f;
    vp.MaxDepth = 1.0f;
    m_context->RSSetViewports(1, &vp);

    // Dibujar hacia la textura de salida procesada (Output Texture)
    ID3D11RenderTargetView* rtvs[1] = { m_outputRTV.Get() };
    m_context->OMSetRenderTargets(1, rtvs, nullptr);

    float clearColor[4] = { 0.0f, 0.0f, 0.0f, 1.0f };
    m_context->ClearRenderTargetView(m_outputRTV.Get(), clearColor);

    // Bind Shaders & Geometría
    UINT stride = sizeof(Vertex);
    UINT offset = 0;
    m_context->IASetVertexBuffers(0, 1, m_vertexBuffer.GetAddressOf(), &stride, &offset);
    m_context->IASetInputLayout(m_inputLayout.Get());
    m_context->IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);

    m_context->VSSetShader(m_vertexShader.Get(), nullptr, 0);
    m_context->PSSetShader(m_pixelShader.Get(), nullptr, 0);
    m_context->PSSetConstantBuffers(0, 1, m_constantBuffer.GetAddressOf());
    m_context->PSSetSamplers(0, 1, m_samplerState.GetAddressOf());

    ID3D11ShaderResourceView* srvs[3] = { m_mainSRV.Get(), m_maskSRV.Get(), m_bgSRV.Get() };
    m_context->PSSetShaderResources(0, 3, srvs);

    // Render Quad
    m_context->Draw(4, 0);

    // Si hay una ventana de preview activa, copiar también al SwapChain RTV
    if (m_renderTargetView) {
        m_context->OMSetRenderTargets(1, m_renderTargetView.GetAddressOf(), nullptr);
        m_context->ClearRenderTargetView(m_renderTargetView.Get(), clearColor);
        m_context->Draw(4, 0);
    }

    // Limpiar bindings
    ID3D11ShaderResourceView* nullSrvs[3] = { nullptr, nullptr, nullptr };
    m_context->PSSetShaderResources(0, 3, nullSrvs);

    return true;
}

void D3D11Renderer::PresentPreview() {
    if (m_swapChain) {
        m_swapChain->Present(1, 0); // VSync sincronizado a 60 FPS
    }
}

bool D3D11Renderer::ReadbackProcessedFrame(std::vector<uint8_t>& outRgba) {
    if (!m_outputTexture || !m_stagingTexture) return false;

    // Copiar de RenderTarget a Staging en GPU
    m_context->CopyResource(m_stagingTexture.Get(), m_outputTexture.Get());

    D3D11_MAPPED_SUBRESOURCE mapped;
    HRESULT hr = m_context->Map(m_stagingTexture.Get(), 0, D3D11_MAP_READ, 0, &mapped);
    if (SUCCEEDED(hr)) {
        size_t totalBytes = m_width * m_height * 4;
        if (outRgba.size() != totalBytes) {
            outRgba.resize(totalBytes);
        }

        const uint8_t* src = (const uint8_t*)mapped.pData;
        uint8_t* dst = outRgba.data();
        int rowBytes = m_width * 4;

        for (int y = 0; y < m_height; ++y) {
            memcpy(dst, src, rowBytes);
            src += mapped.RowPitch;
            dst += rowBytes;
        }

        m_context->Unmap(m_stagingTexture.Get(), 0);
        return true;
    }
    return false;
}

} // namespace CamAr
