cmake_minimum_required(VERSION 3.20)
project(CamArStudioEngine LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_compile_definitions(
    NOMINMAX
    WIN32_LEAN_AND_MEAN
)

include_directories(
    include
)

set(SOURCES
    src/main.cpp
    src/D3D11Renderer.cpp
    src/CameraCapture.cpp
    src/OnnxAiEngine.cpp
    src/VirtualCamManager.cpp
    src/WebSocketServer.cpp
)

set(HEADERS
    include/core/Types.h
    include/processing/D3D11Renderer.h
    include/capture/CameraCapture.h
    include/ai/OnnxAiEngine.h
    include/virtualcam/VirtualCamManager.h
    include/ipc/WebSocketServer.h
)

add_executable(CamArStudioEngine ${SOURCES} ${HEADERS})

target_link_libraries(CamArStudioEngine PRIVATE
    d3d11
    dxgi
    d3dcompiler
    mf
    mfplat
    mfreadwrite
    mfuuid
    ws2_32
    crypt32
)

# Copiar shaders al directorio de salida
add_custom_command(TARGET CamArStudioEngine POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_directory
    "${CMAKE_CURRENT_SOURCE_DIR}/shaders"
    "$<TARGET_FILE_DIR:CamArStudioEngine>/shaders"
)
