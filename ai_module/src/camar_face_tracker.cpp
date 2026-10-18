#ifndef NOMINMAX
#define NOMINMAX
#endif
#include "camar_face_tracker.h"
#include "snapjm-facemesh-topology.h"
#include "snapjm-facemesh-regions.h"
#include <onnxruntime_cxx_api.h>
#include <dml_provider_factory.h>
#include <vector>
#include <cmath>
#include <cstring>
#include <algorithm>
#include <windows.h>
#include <cstdarg>
static void snapjm_log(const char *fmt, ...)
{
    char buf[1024];
    va_list args;
    va_start(args, fmt);
    vsnprintf(buf, sizeof(buf), fmt, args);
    va_end(args);
    OutputDebugStringA(buf);
    OutputDebugStringA("\n");
    FILE *f = fopen("camar_ai.log", "a");
    if (f) {
        fprintf(f, "%s\n", buf);
        fclose(f);
    }
}
#define LOG_INFO 0
#define LOG_WARNING 1
#define blog(lvl, fmt, ...) snapjm_log(fmt, ##__VA_ARGS__)

// MediaPipe 468 FaceMesh Landmark Indices
static const int FACE_OVAL[] = {
    10, 338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288, 397, 365, 379, 378,
    400, 377, 152, 148, 176, 149, 150, 136, 172, 58, 132, 93, 234, 127, 162, 21,
    54, 103, 67, 109
};
static const int NUM_FACE_OVAL = sizeof(FACE_OVAL) / sizeof(FACE_OVAL[0]);

static const int LIP_OUTER[] = {
    61, 185, 40, 39, 37, 0, 267, 269, 270, 409, 291, 375, 321, 405, 314, 17, 84, 181, 91, 146
};
static const int NUM_LIP_OUTER = sizeof(LIP_OUTER) / sizeof(LIP_OUTER[0]);

static const int LIP_INNER[] = {
    78, 191, 80, 81, 82, 13, 312, 311, 310, 415, 308, 324, 318, 402, 317, 14, 87, 178, 88, 95
};
static const int NUM_LIP_INNER = sizeof(LIP_INNER) / sizeof(LIP_INNER[0]);

static const int EYE_LEFT[] = {
    33, 7, 163, 144, 145, 153, 154, 155, 133, 173, 157, 158, 159, 160, 161, 246
};
static const int NUM_EYE_LEFT = sizeof(EYE_LEFT) / sizeof(EYE_LEFT[0]);

static const int EYE_RIGHT[] = {
    362, 382, 381, 380, 374, 373, 390, 249, 263, 466, 388, 387, 386, 385, 384, 398
};
static const int NUM_EYE_RIGHT = sizeof(EYE_RIGHT) / sizeof(EYE_RIGHT[0]);

struct Point2D {
    float x;
    float y;
};

struct BoundingBox {
    float x1, y1, x2, y2;
    float score;
};

enum PolyChannelAction {
    ACT_SET_RED,
    ACT_CLEAR_RED,
    ACT_SET_GREEN,
    ACT_SET_BLUE,
    ACT_SET_ALPHA,
    ACT_CLEAR_ALPHA
};

// Rotation coordinate transform:
// Given normalized coordinates (rx, ry) in detector space [0, 1],
// maps back to original raw camera frame coordinates (ox, oy) in [0, 1].
// rot: 1 = Normal (0 deg), 2 = Vertical 90 deg CCW, 3 = 180 deg, 4 = Vertical 90 deg CW (270 deg)
static inline void rotated_to_original(float rx, float ry, int rot, float *ox, float *oy)
{
    switch (rot) {
    case 2: // 90 deg CCW (Camera rotated right / head tilted right in frame)
        *ox = 1.0f - ry;
        *oy = rx;
        break;
    case 4: // 90 deg CW / 270 deg (Camera rotated left / head tilted left in frame)
        *ox = ry;
        *oy = 1.0f - rx;
        break;
    case 3: // 180 deg
        *ox = 1.0f - rx;
        *oy = 1.0f - ry;
        break;
    case 1: // Normal Upright (0 deg)
    default:
        *ox = rx;
        *oy = ry;
        break;
    }
}

// Scanline polygon rasterizer on mask bitmap
static void rasterize_polygon(uint8_t *bitmap, int width, int height,
                              const Point2D *pts, int count,
                              PolyChannelAction action)
{
    if (count < 3) return;

    float min_y = pts[0].y;
    float max_y = pts[0].y;
    for (int i = 1; i < count; i++) {
        if (pts[i].y < min_y) min_y = pts[i].y;
        if (pts[i].y > max_y) max_y = pts[i].y;
    }

    int start_y = (int)std::floor(min_y);
    int end_y   = (int)std::ceil(max_y);
    if (start_y < 0) start_y = 0;
    if (end_y >= height) end_y = height - 1;

    std::vector<float> nodes;
    nodes.reserve(count);

    for (int y = start_y; y <= end_y; y++) {
        nodes.clear();
        float cur_y = (float)y + 0.5f;

        for (int i = 0; i < count; i++) {
            int j = (i + 1) % count;
            float y0 = pts[i].y;
            float y1 = pts[j].y;
            if ((y0 <= cur_y && y1 > cur_y) || (y1 <= cur_y && y0 > cur_y)) {
                float t = (cur_y - y0) / (y1 - y0);
                float x = pts[i].x + t * (pts[j].x - pts[i].x);
                nodes.push_back(x);
            }
        }

        std::sort(nodes.begin(), nodes.end());

        for (size_t i = 0; i + 1 < nodes.size(); i += 2) {
            int x_start = (int)std::ceil(nodes[i]);
            int x_end   = (int)std::floor(nodes[i + 1]);
            if (x_start < 0) x_start = 0;
            if (x_end >= width) x_end = width - 1;

            for (int x = x_start; x <= x_end; x++) {
                int idx = (y * width + x) * 4;
                switch (action) {
                    case ACT_SET_RED:     bitmap[idx + 0] = 255; break;
                    case ACT_CLEAR_RED:   bitmap[idx + 0] = 0;   break;
                    case ACT_SET_GREEN:   bitmap[idx + 1] = 255; break;
                    case ACT_SET_BLUE:    bitmap[idx + 2] = 255; break;
                    case ACT_SET_ALPHA:   bitmap[idx + 3] = 255; break;
                    case ACT_CLEAR_ALPHA: bitmap[idx + 3] = 0;   break;
                }
            }
        }
    }
}

struct snapjm_face_tracker {
    Ort::Env env;
    Ort::Session *sess_detector;
    Ort::Session *sess_mesh;
    Ort::Session *sess_selfie;
    Ort::MemoryInfo memory_info;

    // Buffer sizes
    static const int DET_W = 320;
    static const int DET_H = 240;
    static const int MESH_W = 192;
    static const int MESH_H = 192;
    static const int SEG_W = 256;
    static const int SEG_H = 256;
    static const int OUT_MASK_W = 320;
    static const int OUT_MASK_H = 240;

    // Tensors
    std::vector<float> det_input;     // [1, 3, 240, 320]
    std::vector<int64_t> det_shape;

    std::vector<float> mesh_input;    // [1, 192, 192, 3]
    std::vector<int64_t> mesh_shape;

    std::vector<float> seg_input;     // [1, 256, 256, 3]
    std::vector<int64_t> seg_shape;
    std::string seg_input_name;
    std::string seg_output_name;
    bool seg_is_multiclass;
    std::vector<uint8_t> seg_face_skin;         // SEG_W * SEG_H binary face skin mask (published)
    std::vector<uint8_t> seg_face_skin_staging; // SEG_W * SEG_H staging buffer for seg worker

    // Detection smoothing
    BoundingBox smooth_face_box;
    bool has_face_box;

    Point2D smoothed_landmarks[468];
    snapjm_face_mesh_vertex_t render_mesh_vertices[468];
    snapjm_face_mesh_vertex_t ready_mesh_vertices[468];
    uint64_t mesh_version;
    bool has_landmarks;

    // Output masks and multi-consumer versioning
    std::vector<uint8_t> render_mask;        // OUT_MASK_W * OUT_MASK_H * 4
    std::vector<uint8_t> ready_mask;
    uint64_t mask_version;

    std::vector<uint8_t> render_person_mask; // SEG_W * SEG_H * 4
    std::vector<uint8_t> ready_person_mask;
    uint64_t person_mask_version;

    bool face_detected;
    float ready_anchor_x;
    float ready_anchor_y;
    snapjm_face_lighting_t render_lighting;
    snapjm_face_lighting_t ready_lighting;

    // Threading
    HANDLE thread_handle;
    HANDLE event_stop;
    HANDLE event_work;
    CRITICAL_SECTION lock;
    CRITICAL_SECTION sync_track_lock;
    bool thread_running;

    // Ingestion buffer
    std::vector<uint8_t> frame_rgba;
    int frame_w;
    int frame_h;
    bool has_new_frame;
    int frame_counter;
    int sync_cycle_counter;
    int user_rotation;
    int active_rotation;
    float camera_aspect_ratio;

    snapjm_face_tracker()
        : env(ORT_LOGGING_LEVEL_ERROR, "SnapJM_AI"),
          sess_detector(nullptr),
          sess_mesh(nullptr),
          sess_selfie(nullptr),
          memory_info(Ort::MemoryInfo::CreateCpu(OrtDeviceAllocator, OrtMemTypeDefault)),
          has_face_box(false),
          has_landmarks(false),
          mask_version(0),
          person_mask_version(0),
          mesh_version(0),
          face_detected(false),
          ready_anchor_x(0.0f),
          ready_anchor_y(0.0f),
          thread_handle(NULL),
          event_stop(NULL),
          event_work(NULL),
          thread_running(false),
          frame_w(0),
          frame_h(0),
          has_new_frame(false),
          frame_counter(0),
          sync_cycle_counter(0),
          user_rotation(0),
          active_rotation(1),
          camera_aspect_ratio(16.0f / 9.0f),
          seg_is_multiclass(false)
    {
        InitializeCriticalSection(&lock);
        InitializeCriticalSection(&sync_track_lock);
        memset(&render_lighting, 0, sizeof(render_lighting));
        memset(&ready_lighting, 0, sizeof(ready_lighting));
        ready_lighting.face_r = 0.5f; ready_lighting.face_g = 0.5f; ready_lighting.face_b = 0.5f; ready_lighting.face_luma = 0.5f;
        ready_lighting.scene_r = 0.5f; ready_lighting.scene_g = 0.5f; ready_lighting.scene_b = 0.5f; ready_lighting.scene_luma = 0.5f;
        render_lighting = ready_lighting;

        det_shape = {1, 3, DET_H, DET_W};
        det_input.resize(1 * 3 * DET_H * DET_W, 0.0f);

        mesh_shape = {1, MESH_H, MESH_W, 3};
        mesh_input.resize(1 * MESH_H * MESH_W * 3, 0.0f);

        seg_shape = {1, SEG_H, SEG_W, 3};
        seg_input.resize(1 * SEG_H * SEG_W * 3, 0.0f);
        seg_face_skin.resize(SEG_W * SEG_H, 0);
        seg_face_skin_staging.resize(SEG_W * SEG_H, 0);

        render_mask.resize(OUT_MASK_W * OUT_MASK_H * 4, 0);
        ready_mask.resize(OUT_MASK_W * OUT_MASK_H * 4, 0);

        render_person_mask.resize(SEG_W * SEG_H * 4, 0);
        ready_person_mask.resize(SEG_W * SEG_H * 4, 0);

        smooth_face_box = {0, 0, 0, 0, 0};
        for (int i = 0; i < 468; i++) {
            smoothed_landmarks[i] = {0.0f, 0.0f};
            render_mesh_vertices[i] = {0.0f, 0.0f, 0.0f};
            ready_mesh_vertices[i] = {0.0f, 0.0f, 0.0f};
        }
    }

    ~snapjm_face_tracker()
    {
        if (thread_running) {
            thread_running = false;
            SetEvent(event_stop);
            if (thread_handle) {
                WaitForSingleObject(thread_handle, 800);
                CloseHandle(thread_handle);
            }
            CloseHandle(event_stop);
            CloseHandle(event_work);
        }
        delete sess_detector;
        delete sess_mesh;
        delete sess_selfie;
        DeleteCriticalSection(&sync_track_lock);
        DeleteCriticalSection(&lock);
    }
};

// Synchronous stage 1 + stage 3 face tracking helper (executes directly on the current camera frame)
static void execute_face_tracking_sync(snapjm_face_tracker *tracker,
                                       const uint8_t *rgba_data,
                                       int fw, int fh, int linesize)
{
    if (!tracker || !rgba_data || fw <= 0 || fh <= 0) return;

    EnterCriticalSection(&tracker->sync_track_lock);
    tracker->sync_cycle_counter++;
    int cycle = tracker->sync_cycle_counter;

    try {
        // Run UltraFace detector every 2nd frame (or immediately if no face box yet) for zero-drift bounding box scale
        bool need_detector = !tracker->has_face_box || !tracker->has_landmarks || ((cycle % 2) == 0);
        if (tracker->sess_detector && need_detector) {
            int rot_candidates[4];
            int num_candidates = 1;
            int user_rot = tracker->user_rotation;
            int act_rot  = tracker->active_rotation;
            rot_candidates[0] = (user_rot != 0) ? user_rot : 1;

            BoundingBox best_detected_box = {0, 0, 0, 0, 0.0f};
            int best_rot = 1;

            for (int c = 0; c < num_candidates; c++) {
                int try_rot = rot_candidates[c];

                float *dst_det = tracker->det_input.data();
                float *plane_r = dst_det;
                float *plane_g = dst_det + (snapjm_face_tracker::DET_H * snapjm_face_tracker::DET_W);
                float *plane_b = dst_det + (2 * snapjm_face_tracker::DET_H * snapjm_face_tracker::DET_W);

                for (int y = 0; y < snapjm_face_tracker::DET_H; y++) {
                    float ry = (float)y / (float)snapjm_face_tracker::DET_H;
                    for (int x = 0; x < snapjm_face_tracker::DET_W; x++) {
                        float rx = (float)x / (float)snapjm_face_tracker::DET_W;

                        float ox, oy;
                        rotated_to_original(rx, ry, try_rot, &ox, &oy);

                        int sx = (int)(ox * (float)fw);
                        int sy = (int)(oy * (float)fh);
                        if (sx < 0) sx = 0;
                        if (sx >= fw) sx = fw - 1;
                        if (sy < 0) sy = 0;
                        if (sy >= fh) sy = fh - 1;

                        const uint8_t *px = rgba_data + (sy * linesize) + (sx * 4);
                        int p_idx = y * snapjm_face_tracker::DET_W + x;
                        plane_r[p_idx] = ((float)px[0] - 127.0f) / 128.0f;
                        plane_g[p_idx] = ((float)px[1] - 127.0f) / 128.0f;
                        plane_b[p_idx] = ((float)px[2] - 127.0f) / 128.0f;
                    }
                }

                Ort::Value det_tensor = Ort::Value::CreateTensor<float>(
                    tracker->memory_info,
                    tracker->det_input.data(),
                    tracker->det_input.size(),
                    tracker->det_shape.data(),
                    tracker->det_shape.size()
                );

                const char *det_in_names[] = {"input"};
                const char *det_out_names[] = {"scores", "boxes"};

                auto det_outs = tracker->sess_detector->Run(
                    Ort::RunOptions{nullptr},
                    det_in_names, &det_tensor, 1,
                    det_out_names, 2
                );

                float *scores = det_outs[0].GetTensorMutableData<float>();
                float *boxes  = det_outs[1].GetTensorMutableData<float>();

                int best_i = -1;
                float cur_best_s = 0.50f;
                for (int i = 0; i < 4420; i++) {
                    float s = scores[i * 2 + 1];
                    if (s > cur_best_s) {
                        cur_best_s = s;
                        best_i = i;
                    }
                }

                if (best_i >= 0 && cur_best_s > best_detected_box.score) {
                    best_detected_box.x1 = std::max(0.0f, std::min(1.0f, boxes[best_i * 4 + 0]));
                    best_detected_box.y1 = std::max(0.0f, std::min(1.0f, boxes[best_i * 4 + 1]));
                    best_detected_box.x2 = std::max(0.0f, std::min(1.0f, boxes[best_i * 4 + 2]));
                    best_detected_box.y2 = std::max(0.0f, std::min(1.0f, boxes[best_i * 4 + 3]));
                    best_detected_box.score = cur_best_s;
                    best_rot = try_rot;

                    if ((try_rot == 1 || try_rot == act_rot) && cur_best_s > 0.65f) {
                        break;
                    }
                }
            }

            if (best_detected_box.score > 0.45f) {
                if (tracker->active_rotation == 0 || tracker->active_rotation != best_rot) {
                    tracker->active_rotation = best_rot;
                    tracker->has_face_box = false;
                }

                if (!tracker->has_face_box) {
                    tracker->smooth_face_box = best_detected_box;
                    tracker->has_face_box = true;
                } else {
                    float b_cx_diff = ((best_detected_box.x1 + best_detected_box.x2) - (tracker->smooth_face_box.x1 + tracker->smooth_face_box.x2)) * 0.5f;
                    float b_cy_diff = ((best_detected_box.y1 + best_detected_box.y2) - (tracker->smooth_face_box.y1 + tracker->smooth_face_box.y2)) * 0.5f;
                    float b_dist = std::sqrt(b_cx_diff * b_cx_diff + b_cy_diff * b_cy_diff);
                    float alpha;
                    if (b_dist < 0.004f) {
                        alpha = 0.20f;
                    } else if (b_dist < 0.015f) {
                        alpha = 0.20f + ((b_dist - 0.004f) / 0.011f) * 0.80f;
                    } else {
                        alpha = 1.0f;
                    }
                    tracker->smooth_face_box.x1 += alpha * (best_detected_box.x1 - tracker->smooth_face_box.x1);
                    tracker->smooth_face_box.y1 += alpha * (best_detected_box.y1 - tracker->smooth_face_box.y1);
                    tracker->smooth_face_box.x2 += alpha * (best_detected_box.x2 - tracker->smooth_face_box.x2);
                    tracker->smooth_face_box.y2 += alpha * (best_detected_box.y2 - tracker->smooth_face_box.y2);
                    tracker->smooth_face_box.score = best_detected_box.score;
                }
                tracker->face_detected = true;
            } else {
                if (tracker->has_face_box) {
                    tracker->smooth_face_box.score *= 0.75f;
                    if (tracker->smooth_face_box.score < 0.25f) {
                        tracker->has_face_box = false;
                        tracker->face_detected = false;
                    }
                }
            }
        }

        // =========================================================================
        // STAGE 3: FACEMESH 468 LANDMARKS (Synchronous 0ms Zero-Lag Tracking)
        // =========================================================================
        if (tracker->sess_mesh && tracker->has_face_box) {
            int active_rot = (tracker->active_rotation != 0) ? tracker->active_rotation : 1;

            float rx1 = tracker->smooth_face_box.x1;
            float ry1 = tracker->smooth_face_box.y1;
            float rx2 = tracker->smooth_face_box.x2;
            float ry2 = tracker->smooth_face_box.y2;

            float rcx = (rx1 + rx2) * 0.5f;
            float rcy = (ry1 + ry2) * 0.5f;
            float w_norm = rx2 - rx1;
            float h_norm = ry2 - ry1;

            float aspect = ((float)fw / (float)fh > 0.05f) ? ((float)fw / (float)fh) : (16.0f / 9.0f);
            float eff_aspect = (active_rot == 2 || active_rot == 4) ? (1.0f / aspect) : aspect;

            // Isotropic 1:1 square crop in physical camera space
            float sq_h = std::max(w_norm * eff_aspect, h_norm) * 1.30f;
            float sq_w = sq_h / eff_aspect;

            float *dst_mesh = tracker->mesh_input.data();
            for (int my = 0; my < snapjm_face_tracker::MESH_H; my++) {
                float m_ry = (rcy - sq_h * 0.5f) + ((float)my / (float)snapjm_face_tracker::MESH_H) * sq_h;
                for (int mx = 0; mx < snapjm_face_tracker::MESH_W; mx++) {
                    float m_rx = (rcx - sq_w * 0.5f) + ((float)mx / (float)snapjm_face_tracker::MESH_W) * sq_w;

                    float ox, oy;
                    rotated_to_original(m_rx, m_ry, active_rot, &ox, &oy);

                    float f_sx = ox * (float)(fw - 1);
                    float f_sy = oy * (float)(fh - 1);
                    if (f_sx < 0.0f) f_sx = 0.0f;
                    if (f_sx > (float)(fw - 1)) f_sx = (float)(fw - 1);
                    if (f_sy < 0.0f) f_sy = 0.0f;
                    if (f_sy > (float)(fh - 1)) f_sy = (float)(fh - 1);

                    int x0 = (int)f_sx;
                    int y0 = (int)f_sy;
                    int x1 = (x0 < fw - 1) ? x0 + 1 : x0;
                    int y1 = (y0 < fh - 1) ? y0 + 1 : y0;
                    float fx = f_sx - (float)x0;
                    float fy = f_sy - (float)y0;

                    const uint8_t *p00 = rgba_data + (y0 * linesize) + x0 * 4;
                    const uint8_t *p10 = rgba_data + (y0 * linesize) + x1 * 4;
                    const uint8_t *p01 = rgba_data + (y1 * linesize) + x0 * 4;
                    const uint8_t *p11 = rgba_data + (y1 * linesize) + x1 * 4;

                    int dst_idx = (my * snapjm_face_tracker::MESH_W + mx) * 3;
                    for (int c = 0; c < 3; c++) {
                        float top = (float)p00[c] * (1.0f - fx) + (float)p10[c] * fx;
                        float bot = (float)p01[c] * (1.0f - fx) + (float)p11[c] * fx;
                        dst_mesh[dst_idx + c] = (top * (1.0f - fy) + bot * fy) / 255.0f;
                    }
                }
            }

            Ort::Value mesh_tensor = Ort::Value::CreateTensor<float>(
                tracker->memory_info,
                tracker->mesh_input.data(),
                tracker->mesh_input.size(),
                tracker->mesh_shape.data(),
                tracker->mesh_shape.size()
            );

            const char *mesh_in_names[] = {"input_1"};
            const char *mesh_out_names[] = {"conv2d_21"};

            auto mesh_outs = tracker->sess_mesh->Run(
                Ort::RunOptions{nullptr},
                mesh_in_names, &mesh_tensor, 1,
                mesh_out_names, 1
            );

            float *raw_landmarks = mesh_outs[0].GetTensorMutableData<float>();

            for (int i = 0; i < 468; i++) {
                float lx = raw_landmarks[i * 3 + 0];
                float ly = raw_landmarks[i * 3 + 1];

                float m_rx = (rcx - sq_w * 0.5f) + (lx / 192.0f) * sq_w;
                float m_ry = (rcy - sq_h * 0.5f) + (ly / 192.0f) * sq_h;

                float ox, oy;
                rotated_to_original(m_rx, m_ry, active_rot, &ox, &oy);

                float lz = raw_landmarks[i * 3 + 2];
                float oz = (lz / 192.0f) * sq_h;

                float mask_x = ox * (float)snapjm_face_tracker::OUT_MASK_W;
                float mask_y = oy * (float)snapjm_face_tracker::OUT_MASK_H;

                if (!tracker->has_landmarks) {
                    tracker->smoothed_landmarks[i].x = mask_x;
                    tracker->smoothed_landmarks[i].y = mask_y;
                    tracker->render_mesh_vertices[i].x = ox;
                    tracker->render_mesh_vertices[i].y = oy;
                    tracker->render_mesh_vertices[i].z = oz;
                } else {
                    // Ultra-responsive zero-lag filter: tiny deadzone for sub-pixel sensor noise (<0.12 units in 320x240),
                    // 100% immediate tracking (a = 1.0f) for any intentional movement >= 0.25 units (~1.5 px in 1080p)
                    float dx = mask_x - tracker->smoothed_landmarks[i].x;
                    float dy = mask_y - tracker->smoothed_landmarks[i].y;
                    float dist = std::sqrt(dx * dx + dy * dy);
                    float a;
                    if (dist < 0.12f) {
                        a = 0.45f;
                    } else if (dist < 0.25f) {
                        a = 0.45f + ((dist - 0.12f) / 0.13f) * 0.55f;
                    } else {
                        a = 1.0f;
                    }

                    tracker->smoothed_landmarks[i].x += a * dx;
                    tracker->smoothed_landmarks[i].y += a * dy;
                    tracker->render_mesh_vertices[i].x += a * (ox - tracker->render_mesh_vertices[i].x);
                    tracker->render_mesh_vertices[i].y += a * (oy - tracker->render_mesh_vertices[i].y);
                    tracker->render_mesh_vertices[i].z += a * (oz - tracker->render_mesh_vertices[i].z);
                }
            }
            tracker->has_landmarks = true;

            // Clear render mask (RGBA = 0)
            std::fill(tracker->render_mask.begin(), tracker->render_mask.end(), 0);

            Point2D lip_outer_pts[32];
            for (int i = 0; i < NUM_LIP_OUTER; i++) lip_outer_pts[i] = tracker->smoothed_landmarks[LIP_OUTER[i]];

            Point2D lip_inner_pts[32];
            for (int i = 0; i < NUM_LIP_INNER; i++) lip_inner_pts[i] = tracker->smoothed_landmarks[LIP_INNER[i]];

            Point2D eye_l_pts[32];
            for (int i = 0; i < NUM_EYE_LEFT; i++) eye_l_pts[i] = tracker->smoothed_landmarks[EYE_LEFT[i]];

            Point2D eye_r_pts[32];
            for (int i = 0; i < NUM_EYE_RIGHT; i++) eye_r_pts[i] = tracker->smoothed_landmarks[EYE_RIGHT[i]];

            float mouth_gap = std::abs(tracker->smoothed_landmarks[14].y - tracker->smoothed_landmarks[13].y);
            float mouth_cx = 0, mouth_cy = 0;
            for (int i = 0; i < NUM_LIP_INNER; i++) {
                mouth_cx += tracker->smoothed_landmarks[LIP_INNER[i]].x;
                mouth_cy += tracker->smoothed_landmarks[LIP_INNER[i]].y;
            }
            mouth_cx /= (float)NUM_LIP_INNER;
            mouth_cy /= (float)NUM_LIP_INNER;

            Point2D lip_inner_precise[32];
            for (int i = 0; i < NUM_LIP_INNER; i++) {
                float dx = tracker->smoothed_landmarks[LIP_INNER[i]].x - mouth_cx;
                float dy = tracker->smoothed_landmarks[LIP_INNER[i]].y - mouth_cy;
                lip_inner_precise[i].x = mouth_cx + dx * 0.95f;
                lip_inner_precise[i].y = mouth_cy + dy * 0.95f;
            }

            // 1. Channel RED: Facial skin from 898 canonical MediaPipe 3D Mesh Triangles
            for (int t = 0; t < SNAPJM_NUM_FACEMESH_TRIANGLES; t++) {
                uint8_t tri_type = SNAPJM_FACEMESH_TRI_TYPE[t];
                if (tri_type == 0 || tri_type == 4) {
                    int i0 = SNAPJM_FACEMESH_TRIANGLES[t][0];
                    int i1 = SNAPJM_FACEMESH_TRIANGLES[t][1];
                    int i2 = SNAPJM_FACEMESH_TRIANGLES[t][2];
                    if (i0 < 468 && i1 < 468 && i2 < 468) {
                        Point2D tri[3] = {
                            tracker->smoothed_landmarks[i0],
                            tracker->smoothed_landmarks[i1],
                            tracker->smoothed_landmarks[i2]
                        };
                        rasterize_polygon(tracker->render_mask.data(),
                                          snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                                          tri, 3, ACT_SET_RED);
                    }
                }
            }

            // Transferir frente completa desde el segmentador multiclass MediaPipe (si está activo)
            if (tracker->seg_is_multiclass) {
                EnterCriticalSection(&tracker->lock);
                for (int my = 0; my < snapjm_face_tracker::OUT_MASK_H; my++) {
                    int sy = (my * snapjm_face_tracker::SEG_H) / snapjm_face_tracker::OUT_MASK_H;
                    if (sy >= snapjm_face_tracker::SEG_H) sy = snapjm_face_tracker::SEG_H - 1;

                    for (int mx = 0; mx < snapjm_face_tracker::OUT_MASK_W; mx++) {
                        int sx = (mx * snapjm_face_tracker::SEG_W) / snapjm_face_tracker::OUT_MASK_W;
                        if (sx >= snapjm_face_tracker::SEG_W) sx = snapjm_face_tracker::SEG_W - 1;

                        uint8_t s_val = tracker->seg_face_skin[sy * snapjm_face_tracker::SEG_W + sx];
                        if (s_val > 0) {
                            int m_idx = (my * snapjm_face_tracker::OUT_MASK_W + mx) * 4;
                            tracker->render_mask[m_idx + 0] = s_val;
                        }
                    }
                }
                LeaveCriticalSection(&tracker->lock);
            }

            static const int EYEBROW_LEFT[] = { 70, 63, 105, 66, 107, 55, 65, 52, 53, 46 };
            static const int NUM_EYEBROW_LEFT = sizeof(EYEBROW_LEFT) / sizeof(EYEBROW_LEFT[0]);

            static const int EYEBROW_RIGHT[] = { 336, 296, 334, 293, 300, 285, 295, 282, 283, 276 };
            static const int NUM_EYEBROW_RIGHT = sizeof(EYEBROW_RIGHT) / sizeof(EYEBROW_RIGHT[0]);

            Point2D eb_l_pts[16], eb_r_pts[16];
            for (int i = 0; i < NUM_EYEBROW_LEFT; i++) eb_l_pts[i] = tracker->smoothed_landmarks[EYEBROW_LEFT[i]];
            for (int i = 0; i < NUM_EYEBROW_RIGHT; i++) eb_r_pts[i] = tracker->smoothed_landmarks[EYEBROW_RIGHT[i]];

            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              eb_l_pts, NUM_EYEBROW_LEFT, ACT_CLEAR_RED);
            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              eb_r_pts, NUM_EYEBROW_RIGHT, ACT_CLEAR_RED);

            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              eye_l_pts, NUM_EYE_LEFT, ACT_CLEAR_RED);
            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              eye_r_pts, NUM_EYE_RIGHT, ACT_CLEAR_RED);
            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              lip_outer_pts, NUM_LIP_OUTER, ACT_CLEAR_RED);

            if (mouth_gap >= 2.2f) {
                rasterize_polygon(tracker->render_mask.data(),
                                  snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                                  lip_inner_precise, NUM_LIP_INNER, ACT_SET_GREEN);
            }

            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              eye_l_pts, NUM_EYE_LEFT, ACT_SET_BLUE);
            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              eye_r_pts, NUM_EYE_RIGHT, ACT_SET_BLUE);

            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              lip_outer_pts, NUM_LIP_OUTER, ACT_SET_ALPHA);
            rasterize_polygon(tracker->render_mask.data(),
                              snapjm_face_tracker::OUT_MASK_W, snapjm_face_tracker::OUT_MASK_H,
                              lip_inner_pts, NUM_LIP_INNER, ACT_CLEAR_ALPHA);

            // Compute Real-time Face & Scene Photometric Metrics
            double skin_r_acc = 0, skin_g_acc = 0, skin_b_acc = 0;
            int skin_px_cnt = 0;
            double scene_r_acc = 0, scene_g_acc = 0, scene_b_acc = 0;
            int scene_px_cnt = 0;

            int mw = snapjm_face_tracker::OUT_MASK_W;
            int mh = snapjm_face_tracker::OUT_MASK_H;
            const uint8_t *mask_bytes = tracker->render_mask.data();

            for (int my = 0; my < mh; my += 2) {
                int fy = (my * fh) / mh;
                if (fy >= fh) fy = fh - 1;
                const uint8_t *f_row = rgba_data + (fy * linesize);
                const uint8_t *m_row = mask_bytes + (my * mw * 4);

                for (int mx = 0; mx < mw; mx += 2) {
                    int fx = (mx * fw) / mw;
                    if (fx >= fw) fx = fw - 1;

                    const uint8_t *f_px = f_row + (fx * 4);
                    const uint8_t *m_px = m_row + (mx * 4);

                    float r = f_px[0] / 255.0f;
                    float g = f_px[1] / 255.0f;
                    float b = f_px[2] / 255.0f;

                    scene_r_acc += r;
                    scene_g_acc += g;
                    scene_b_acc += b;
                    scene_px_cnt++;

                    if (m_px[0] > 128) {
                        skin_r_acc += r;
                        skin_g_acc += g;
                        skin_b_acc += b;
                        skin_px_cnt++;
                    }
                }
            }

            if (scene_px_cnt > 0) {
                tracker->render_lighting.scene_r = (float)(scene_r_acc / scene_px_cnt);
                tracker->render_lighting.scene_g = (float)(scene_g_acc / scene_px_cnt);
                tracker->render_lighting.scene_b = (float)(scene_b_acc / scene_px_cnt);
                tracker->render_lighting.scene_luma = (float)(0.299 * tracker->render_lighting.scene_r + 0.587 * tracker->render_lighting.scene_g + 0.114 * tracker->render_lighting.scene_b);
            }

            if (skin_px_cnt > 20) {
                tracker->render_lighting.has_face = true;
                tracker->render_lighting.face_r = (float)(skin_r_acc / skin_px_cnt);
                tracker->render_lighting.face_g = (float)(skin_g_acc / skin_px_cnt);
                tracker->render_lighting.face_b = (float)(skin_b_acc / skin_px_cnt);
                tracker->render_lighting.face_luma = (float)(0.299 * tracker->render_lighting.face_r + 0.587 * tracker->render_lighting.face_g + 0.114 * tracker->render_lighting.face_b);
            } else {
                tracker->render_lighting.has_face = false;
                tracker->render_lighting.face_r = tracker->render_lighting.scene_r;
                tracker->render_lighting.face_g = tracker->render_lighting.scene_g;
                tracker->render_lighting.face_b = tracker->render_lighting.scene_b;
                tracker->render_lighting.face_luma = tracker->render_lighting.scene_luma;
            }

            EnterCriticalSection(&tracker->lock);
            tracker->ready_mask = tracker->render_mask;
            tracker->mask_version++;
            tracker->ready_anchor_x = tracker->render_mesh_vertices[1].x;
            tracker->ready_anchor_y = tracker->render_mesh_vertices[1].y;
            tracker->ready_lighting = tracker->render_lighting;
            memcpy(tracker->ready_mesh_vertices, tracker->render_mesh_vertices, sizeof(tracker->ready_mesh_vertices));
            tracker->mesh_version++;
            LeaveCriticalSection(&tracker->lock);
        } else if (!tracker->has_face_box) {
            tracker->has_landmarks = false;
            tracker->render_lighting.has_face = false;
            EnterCriticalSection(&tracker->lock);
            tracker->face_detected = false;
            tracker->ready_lighting.has_face = false;
            tracker->ready_anchor_x = 0.0f;
            tracker->ready_anchor_y = 0.0f;
            std::fill(tracker->ready_mask.begin(), tracker->ready_mask.end(), 0);
            tracker->mask_version++;
            LeaveCriticalSection(&tracker->lock);
        }
    } catch (...) {
    }

    LeaveCriticalSection(&tracker->sync_track_lock);
}

// Background worker thread dedicated ONLY to Stage 2 (Selfie Segmentation 256x256 on DirectML GPU)
static DWORD WINAPI face_tracker_worker_thread(LPVOID param)
{
    snapjm_face_tracker *tracker = (snapjm_face_tracker *)param;
    HANDLE handles[2] = {tracker->event_stop, tracker->event_work};

    while (tracker->thread_running) {
        DWORD res = WaitForMultipleObjects(2, handles, FALSE, 35);
        if (res == WAIT_OBJECT_0) break;

        EnterCriticalSection(&tracker->lock);
        if (!tracker->has_new_frame) {
            LeaveCriticalSection(&tracker->lock);
            continue;
        }
        std::vector<uint8_t> frame = tracker->frame_rgba;
        int fw = tracker->frame_w;
        int fh = tracker->frame_h;
        tracker->has_new_frame = false;
        tracker->frame_counter++;
        LeaveCriticalSection(&tracker->lock);

        if (fw <= 0 || fh <= 0 || frame.empty()) continue;

        try {
            if (tracker->sess_selfie) {
                float *dst_seg = tracker->seg_input.data();
                for (int y = 0; y < snapjm_face_tracker::SEG_H; y++) {
                    int sy = (y * fh) / snapjm_face_tracker::SEG_H;
                    if (sy >= fh) sy = fh - 1;
                    const uint8_t *row = frame.data() + (sy * fw * 4);

                    for (int x = 0; x < snapjm_face_tracker::SEG_W; x++) {
                        int sx = (x * fw) / snapjm_face_tracker::SEG_W;
                        if (sx >= fw) sx = fw - 1;
                        const uint8_t *px = row + (sx * 4);

                        int dst_idx = (y * snapjm_face_tracker::SEG_W + x) * 3;
                        dst_seg[dst_idx + 0] = (float)px[0] / 255.0f;
                        dst_seg[dst_idx + 1] = (float)px[1] / 255.0f;
                        dst_seg[dst_idx + 2] = (float)px[2] / 255.0f;
                    }
                }

                Ort::Value seg_tensor = Ort::Value::CreateTensor<float>(
                    tracker->memory_info,
                    tracker->seg_input.data(),
                    tracker->seg_input.size(),
                    tracker->seg_shape.data(),
                    tracker->seg_shape.size()
                );

                const char *seg_in_names[] = {tracker->seg_input_name.c_str()};
                const char *seg_out_names[] = {tracker->seg_output_name.c_str()};

                auto seg_outs = tracker->sess_selfie->Run(
                    Ort::RunOptions{nullptr},
                    seg_in_names, &seg_tensor, 1,
                    seg_out_names, 1
                );

                float *seg_data = seg_outs[0].GetTensorMutableData<float>();
                uint8_t *out_p = tracker->render_person_mask.data();

                if (tracker->seg_is_multiclass) {
                    for (int i = 0; i < snapjm_face_tracker::SEG_W * snapjm_face_tracker::SEG_H; i++) {
                        float *logits = seg_data + (i * 6);
                        int max_cls = 0;
                        float max_val = logits[0];
                        for (int c = 1; c < 6; c++) {
                            if (logits[c] > max_val) {
                                max_val = logits[c];
                                max_cls = c;
                            }
                        }

                        uint8_t person_val = (max_cls >= 1 && max_cls <= 4) ? 255 : 0;
                        out_p[i * 4 + 0] = person_val;
                        out_p[i * 4 + 1] = person_val;
                        out_p[i * 4 + 2] = person_val;
                        out_p[i * 4 + 3] = 255;

                        tracker->seg_face_skin_staging[i] = (max_cls == 3) ? 255 : ((max_cls == 2) ? 180 : 0);
                    }
                } else {
                    for (int i = 0; i < snapjm_face_tracker::SEG_W * snapjm_face_tracker::SEG_H; i++) {
                        float val = seg_data[i];
                        uint8_t u8 = (uint8_t)std::max(0.0f, std::min(255.0f, val * 255.0f));
                        out_p[i * 4 + 0] = u8;
                        out_p[i * 4 + 1] = u8;
                        out_p[i * 4 + 2] = u8;
                        out_p[i * 4 + 3] = 255;
                    }
                }

                EnterCriticalSection(&tracker->lock);
                tracker->ready_person_mask = tracker->render_person_mask;
                tracker->seg_face_skin = tracker->seg_face_skin_staging;
                tracker->person_mask_version++;
                LeaveCriticalSection(&tracker->lock);
            }
        } catch (...) {
        }
    }

    return 0;
}

// Global shared singleton state
static snapjm_face_tracker_t *s_shared_tracker = nullptr;
static int s_tracker_refcount = 0;
static CRITICAL_SECTION s_tracker_mutex;
static bool s_tracker_mutex_init = false;

static void init_tracker_mutex()
{
    if (!s_tracker_mutex_init) {
        InitializeCriticalSection(&s_tracker_mutex);
        s_tracker_mutex_init = true;
    }
}

snapjm_face_tracker_t *snapjm_face_tracker_acquire(const char *detector_model_path,
                                                   const char *mesh_model_path,
                                                   const char *selfie_model_path)
{
    return snapjm_face_tracker_acquire_ex(detector_model_path, mesh_model_path, selfie_model_path, "npu");
}

snapjm_face_tracker_t *snapjm_face_tracker_acquire_ex(const char *detector_model_path,
                                                      const char *mesh_model_path,
                                                      const char *selfie_model_path,
                                                      const char *ai_backend)
{
    init_tracker_mutex();
    EnterCriticalSection(&s_tracker_mutex);

    if (s_shared_tracker != nullptr) {
        s_tracker_refcount++;
        LeaveCriticalSection(&s_tracker_mutex);
        return s_shared_tracker;
    }

    if (!mesh_model_path) {
        LeaveCriticalSection(&s_tracker_mutex);
        return nullptr;
    }

    snapjm_face_tracker *tracker = new snapjm_face_tracker();
    try {
        // Dedicated low-latency CPU session options
        Ort::SessionOptions session_options_cpu;
        session_options_cpu.SetIntraOpNumThreads(4);
        session_options_cpu.SetGraphOptimizationLevel(GraphOptimizationLevel::ORT_ENABLE_ALL);

        // Dedicated DirectML GPU session options for heavy 256x256 Multiclass Selfie Segmentation
        Ort::SessionOptions session_options_dml;
        session_options_dml.SetIntraOpNumThreads(2);
        session_options_dml.SetGraphOptimizationLevel(GraphOptimizationLevel::ORT_ENABLE_ALL);
        bool dml_ok = false;
        try {
            OrtSessionOptionsAppendExecutionProvider_DML(session_options_dml, 0);
            blog(LOG_INFO, "[SnapJM Tracker] DirectML GPU Execution Provider activado (Device 0).");
            dml_ok = true;
        } catch (...) {
            blog(LOG_WARNING, "[SnapJM Tracker] DirectML no disponible, continuando en CPU.");
        }

        // Selección de backend para FaceMesh según ai_backend ("npu", "gpu", "cpu")
        Ort::SessionOptions session_options_npu;
        Ort::SessionOptions *p_session_options_mesh = &session_options_cpu;

        if (ai_backend && strcmp(ai_backend, "npu") == 0) {
            bool npu_ok = false;
            try {
                // Device 1 corresponde al acelerador de cómputo / NPU Intel AI Boost en DirectML
                session_options_npu.SetIntraOpNumThreads(2);
                session_options_npu.SetGraphOptimizationLevel(GraphOptimizationLevel::ORT_ENABLE_ALL);
                OrtSessionOptionsAppendExecutionProvider_DML(session_options_npu, 1);
                p_session_options_mesh = &session_options_npu;
                blog(LOG_INFO, "[SnapJM Tracker] Inferencia NPU activada (Intel AI Boost / DirectML Device 1).");
                npu_ok = true;
            } catch (...) {
                blog(LOG_WARNING, "[SnapJM Tracker] NPU Device 1 no disponible, usando aceleración GPU DirectML.");
            }
            if (!npu_ok && dml_ok) {
                p_session_options_mesh = &session_options_dml;
            }
        } else if (ai_backend && strcmp(ai_backend, "gpu") == 0) {
            if (dml_ok) {
                p_session_options_mesh = &session_options_dml;
                blog(LOG_INFO, "[SnapJM Tracker] Inferencia GPU activada para FaceMesh (Tensor Cores / DirectML).");
            }
        } else {
            blog(LOG_INFO, "[SnapJM Tracker] Inferencia CPU AVX2 multihilo activada para FaceMesh.");
        }

        // 1. Load UltraFace Detector (Synchronous Zero-Wait CPU AVX2)
        if (detector_model_path) {
            int wlen = MultiByteToWideChar(CP_UTF8, 0, detector_model_path, -1, NULL, 0);
            std::wstring wpath(wlen, 0);
            MultiByteToWideChar(CP_UTF8, 0, detector_model_path, -1, &wpath[0], wlen);
            tracker->sess_detector = new Ort::Session(tracker->env, wpath.c_str(), session_options_cpu);
        }

        // 2. Load FaceMesh (con backend NPU / GPU / CPU seleccionado con fallback automático a CPU)
        if (mesh_model_path) {
            int wlen = MultiByteToWideChar(CP_UTF8, 0, mesh_model_path, -1, NULL, 0);
            std::wstring wpath(wlen, 0);
            MultiByteToWideChar(CP_UTF8, 0, mesh_model_path, -1, &wpath[0], wlen);
            try {
                tracker->sess_mesh = new Ort::Session(tracker->env, wpath.c_str(), *p_session_options_mesh);
            } catch (...) {
                blog(LOG_WARNING, "[SnapJM Tracker] Error al crear sesión acelerada FaceMesh, fallback seguro a CPU.");
                tracker->sess_mesh = new Ort::Session(tracker->env, wpath.c_str(), session_options_cpu);
            }
        }

        // 3. Load Selfie Segmentation (Asynchronous Background GPU DirectML)
        if (selfie_model_path) {
            int wlen = MultiByteToWideChar(CP_UTF8, 0, selfie_model_path, -1, NULL, 0);
            std::wstring wpath(wlen, 0);
            MultiByteToWideChar(CP_UTF8, 0, selfie_model_path, -1, &wpath[0], wlen);
            try {
                tracker->sess_selfie = new Ort::Session(tracker->env, wpath.c_str(), session_options_dml);
            } catch (...) {
                tracker->sess_selfie = new Ort::Session(tracker->env, wpath.c_str(), session_options_cpu);
            }

            Ort::AllocatorWithDefaultOptions alloc;
            auto in_name = tracker->sess_selfie->GetInputNameAllocated(0, alloc);
            auto out_name = tracker->sess_selfie->GetOutputNameAllocated(0, alloc);
            tracker->seg_input_name = in_name.get();
            tracker->seg_output_name = out_name.get();

            auto out_type_info = tracker->sess_selfie->GetOutputTypeInfo(0);
            auto tensor_info = out_type_info.GetTensorTypeAndShapeInfo();
            auto shape = tensor_info.GetShape();
            tracker->seg_is_multiclass = (shape.size() >= 4 && shape[3] == 6);
            blog(LOG_INFO, "[SnapJM Tracker] Segmenter cargado: '%s' (Multiclass=%d, In='%s', Out='%s')",
                 selfie_model_path, tracker->seg_is_multiclass ? 1 : 0,
                 tracker->seg_input_name.c_str(), tracker->seg_output_name.c_str());
        }

        tracker->event_stop = CreateEvent(NULL, TRUE, FALSE, NULL);
        tracker->event_work = CreateEvent(NULL, FALSE, FALSE, NULL);
        tracker->thread_running = true;
        tracker->thread_handle = CreateThread(NULL, 0, face_tracker_worker_thread, tracker, 0, NULL);

        s_shared_tracker = tracker;
        s_tracker_refcount = 1;

        blog(LOG_INFO, "[SnapJM Face Tracker] Shared singleton tracker inicializado con exito.");
        LeaveCriticalSection(&s_tracker_mutex);
        return tracker;
    } catch (...) {
        delete tracker;
        LeaveCriticalSection(&s_tracker_mutex);
        return nullptr;
    }
}

void snapjm_face_tracker_release(snapjm_face_tracker_t *tracker)
{
    init_tracker_mutex();
    EnterCriticalSection(&s_tracker_mutex);

    if (tracker && tracker == s_shared_tracker) {
        s_tracker_refcount--;
        if (s_tracker_refcount <= 0) {
            delete s_shared_tracker;
            s_shared_tracker = nullptr;
            s_tracker_refcount = 0;
            blog(LOG_INFO, "[SnapJM Face Tracker] Shared singleton tracker liberado.");
        }
    } else if (tracker) {
        delete tracker;
    }

    LeaveCriticalSection(&s_tracker_mutex);
}

snapjm_face_tracker_t *snapjm_face_tracker_create(const char *detector_model_path,
                                                  const char *mesh_model_path,
                                                  const char *selfie_model_path)
{
    return snapjm_face_tracker_acquire(detector_model_path, mesh_model_path, selfie_model_path);
}

void snapjm_face_tracker_destroy(snapjm_face_tracker_t *tracker)
{
    snapjm_face_tracker_release(tracker);
}

bool snapjm_face_tracker_needs_frame(snapjm_face_tracker_t *tracker)
{
    if (!tracker) return false;
    EnterCriticalSection(&tracker->lock);
    bool needed = !tracker->has_new_frame;
    LeaveCriticalSection(&tracker->lock);
    return needed;
}

void snapjm_face_tracker_post_frame(snapjm_face_tracker_t *tracker,
                                    const uint8_t *rgba_data,
                                    int width, int height, int linesize)
{
    if (!tracker || !rgba_data || width <= 0 || height <= 0) return;

    EnterCriticalSection(&tracker->lock);
    if (tracker->frame_w != width || tracker->frame_h != height) {
        tracker->frame_w = width;
        tracker->frame_h = height;
        tracker->frame_rgba.resize(width * height * 4);
    }

    uint8_t *dst = tracker->frame_rgba.data();
    if (linesize == width * 4) {
        memcpy(dst, rgba_data, (size_t)width * height * 4);
    } else {
        for (int y = 0; y < height; y++) {
            memcpy(dst + (y * width * 4), rgba_data + (y * linesize), (size_t)width * 4);
        }
    }

    static bool s_first_post = false;
    if (!s_first_post) {
        s_first_post = true;
        blog(LOG_INFO, "[SnapJM Face Tracker] Ingestion de frames GPU activa: %dx%d (linesize=%d)", width, height, linesize);
    }

    tracker->has_new_frame = true;
    if (height > 0) {
        tracker->camera_aspect_ratio = (float)width / (float)height;
    }
    LeaveCriticalSection(&tracker->lock);

    SetEvent(tracker->event_work);
}

void snapjm_face_tracker_track_face_sync(snapjm_face_tracker_t *tracker,
                                         const uint8_t *rgba_data,
                                         int width, int height, int linesize)
{
    if (!tracker || !rgba_data || width <= 0 || height <= 0) return;

    // 1. Feed background Selfie Segmentation worker if it is ready for a new frame (non-blocking)
    if (snapjm_face_tracker_needs_frame(tracker)) {
        snapjm_face_tracker_post_frame(tracker, rgba_data, width, height, linesize);
    }

    // 2. Run Stage 1 + Stage 3 (468 FaceMesh + Feature Mask) synchronously on this exact frame (0.0ms lag)
    execute_face_tracking_sync(tracker, rgba_data, width, height, linesize);
}

bool snapjm_face_tracker_get_mask(snapjm_face_tracker_t *tracker,
                                  uint8_t *out_rgba_mask,
                                  int target_width, int target_height,
                                  uint64_t *inout_version)
{
    if (!tracker || !out_rgba_mask) return false;
    if (target_width != snapjm_face_tracker::OUT_MASK_W || target_height != snapjm_face_tracker::OUT_MASK_H)
        return false;

    EnterCriticalSection(&tracker->lock);
    if (inout_version && *inout_version == tracker->mask_version) {
        LeaveCriticalSection(&tracker->lock);
        return false;
    }
    memcpy(out_rgba_mask, tracker->ready_mask.data(),
           snapjm_face_tracker::OUT_MASK_W * snapjm_face_tracker::OUT_MASK_H * 4);
    if (inout_version) {
        *inout_version = tracker->mask_version;
    }
    LeaveCriticalSection(&tracker->lock);

    return true;
}

bool snapjm_face_tracker_get_person_mask(snapjm_face_tracker_t *tracker,
                                         uint8_t *out_person_mask,
                                         int target_width, int target_height,
                                         uint64_t *inout_version)
{
    if (!tracker || !out_person_mask) return false;
    if (target_width != snapjm_face_tracker::SEG_W || target_height != snapjm_face_tracker::SEG_H)
        return false;

    EnterCriticalSection(&tracker->lock);
    if (inout_version && *inout_version == tracker->person_mask_version) {
        LeaveCriticalSection(&tracker->lock);
        return false;
    }
    memcpy(out_person_mask, tracker->ready_person_mask.data(),
           snapjm_face_tracker::SEG_W * snapjm_face_tracker::SEG_H * 4);
    if (inout_version) {
        *inout_version = tracker->person_mask_version;
    }
    LeaveCriticalSection(&tracker->lock);

    return true;
}

bool snapjm_face_tracker_has_face(snapjm_face_tracker_t *tracker)
{
    if (!tracker) return false;
    EnterCriticalSection(&tracker->lock);
    bool detected = tracker->face_detected;
    LeaveCriticalSection(&tracker->lock);
    return detected;
}

void snapjm_face_tracker_set_rotation(snapjm_face_tracker_t *tracker, int rotation)
{
    if (!tracker) return;
    EnterCriticalSection(&tracker->lock);
    tracker->user_rotation = rotation;
    if (rotation != 0) {
        tracker->active_rotation = rotation;
    }
    LeaveCriticalSection(&tracker->lock);
}

void snapjm_face_tracker_set_aspect_ratio(snapjm_face_tracker_t *tracker, float aspect_ratio)
{
    if (!tracker || aspect_ratio <= 0.05f) return;
    EnterCriticalSection(&tracker->lock);
    tracker->camera_aspect_ratio = aspect_ratio;
    LeaveCriticalSection(&tracker->lock);
}

bool snapjm_face_tracker_get_mesh_landmarks(snapjm_face_tracker_t *tracker,
                                            snapjm_face_mesh_vertex_t *out_landmarks,
                                            int max_landmarks,
                                            uint64_t *inout_version)
{
    if (!tracker || !out_landmarks || max_landmarks < 468) return false;

    EnterCriticalSection(&tracker->lock);
    if (!tracker->face_detected || !tracker->has_landmarks) {
        LeaveCriticalSection(&tracker->lock);
        return false;
    }

    if (inout_version && *inout_version == tracker->mesh_version) {
        LeaveCriticalSection(&tracker->lock);
        return false;
    }

    memcpy(out_landmarks, tracker->ready_mesh_vertices, sizeof(tracker->ready_mesh_vertices));
    if (inout_version) {
        *inout_version = tracker->mesh_version;
    }
    LeaveCriticalSection(&tracker->lock);
    return true;
}

bool snapjm_face_tracker_get_face_lighting(snapjm_face_tracker_t *tracker,
                                           snapjm_face_lighting_t *out_lighting)
{
    if (!tracker || !out_lighting) return false;
    EnterCriticalSection(&tracker->lock);
    *out_lighting = tracker->ready_lighting;
    LeaveCriticalSection(&tracker->lock);
    return true;
}

bool snapjm_face_tracker_get_mask_anchor(snapjm_face_tracker_t *tracker, float *out_x, float *out_y)
{
    if (!tracker || !out_x || !out_y) return false;
    EnterCriticalSection(&tracker->lock);
    *out_x = tracker->ready_anchor_x;
    *out_y = tracker->ready_anchor_y;
    bool valid = tracker->face_detected && (tracker->ready_anchor_x > 0.05f);
    LeaveCriticalSection(&tracker->lock);
    return valid;
}



