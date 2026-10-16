cmake_minimum_required(VERSION 3.20)
project(CamArTracker LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

include_directories(
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
    "${CMAKE_CURRENT_SOURCE_DIR}/../deps/onnxruntime/include"
)

link_directories(
    "${CMAKE_CURRENT_SOURCE_DIR}/../deps/onnxruntime/lib"
)

add_definitions(-DCAMAR_AI_EXPORTS -DNOMINMAX -DWIN32_LEAN_AND_MEAN)

add_library(camar_tracker SHARED
    src/camar_face_tracker.cpp
)

target_link_libraries(camar_tracker PRIVATE
    onnxruntime
    user32
    gdi32
    ole32
    oleaut32
)
