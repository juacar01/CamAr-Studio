#ifndef CAMAR_FACE_TRACKER_H
#define CAMAR_FACE_TRACKER_H

#include <stdbool.h>
#include <stdint.h>

#ifdef _WIN32
  #ifdef CAMAR_AI_EXPORTS
    #define CAMAR_AI_API __declspec(dllexport)
  #else
    #define CAMAR_AI_API __declspec(dllimport)
  #endif
#else
  #define CAMAR_AI_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct snapjm_face_tracker snapjm_face_tracker_t;

// Photometric lighting metrics extracted from detected facial skin vs background scene
typedef struct snapjm_face_lighting {
    bool has_face;
    float face_r;
    float face_g;
    float face_b;
    float face_luma;
    float scene_r;
    float scene_g;
    float scene_b;
    float scene_luma;
} snapjm_face_lighting_t;

// Normalized 3D Face Landmark in original raw frame coordinates [0..1]
typedef struct snapjm_face_mesh_vertex {
    float x; // Normalized 0..1 in original frame
    float y; // Normalized 0..1 in original frame
    float z; // Relative depth in original frame
} snapjm_face_mesh_vertex_t;

CAMAR_AI_API snapjm_face_tracker_t *snapjm_face_tracker_acquire(const char *detector_model_path,
                                                               const char *mesh_model_path,
                                                               const char *selfie_model_path);

CAMAR_AI_API void snapjm_face_tracker_release(snapjm_face_tracker_t *tracker);

CAMAR_AI_API snapjm_face_tracker_t *snapjm_face_tracker_create(const char *detector_model_path,
                                                               const char *mesh_model_path,
                                                               const char *selfie_model_path);
CAMAR_AI_API void snapjm_face_tracker_destroy(snapjm_face_tracker_t *tracker);

CAMAR_AI_API bool snapjm_face_tracker_needs_frame(snapjm_face_tracker_t *tracker);

CAMAR_AI_API void snapjm_face_tracker_post_frame(snapjm_face_tracker_t *tracker,
                                                 const uint8_t *rgba_data,
                                                 int width, int height, int linesize);

CAMAR_AI_API void snapjm_face_tracker_track_face_sync(snapjm_face_tracker_t *tracker,
                                                      const uint8_t *rgba_data,
                                                      int width, int height, int linesize);

CAMAR_AI_API bool snapjm_face_tracker_get_mask(snapjm_face_tracker_t *tracker,
                                               uint8_t *out_rgba_mask,
                                               int target_width, int target_height,
                                               uint64_t *inout_version);

CAMAR_AI_API bool snapjm_face_tracker_get_person_mask(snapjm_face_tracker_t *tracker,
                                                      uint8_t *out_person_mask,
                                                      int target_width, int target_height,
                                                      uint64_t *inout_version);

CAMAR_AI_API bool snapjm_face_tracker_has_face(snapjm_face_tracker_t *tracker);

CAMAR_AI_API bool snapjm_face_tracker_get_mask_anchor(snapjm_face_tracker_t *tracker, float *out_x, float *out_y);

CAMAR_AI_API bool snapjm_face_tracker_get_face_lighting(snapjm_face_tracker_t *tracker,
                                                        snapjm_face_lighting_t *out_lighting);

CAMAR_AI_API bool snapjm_face_tracker_get_mesh_landmarks(snapjm_face_tracker_t *tracker,
                                                         snapjm_face_mesh_vertex_t *out_landmarks,
                                                         int max_landmarks,
                                                         uint64_t *inout_version);

CAMAR_AI_API void snapjm_face_tracker_set_rotation(snapjm_face_tracker_t *tracker, int rotation);

CAMAR_AI_API void snapjm_face_tracker_set_aspect_ratio(snapjm_face_tracker_t *tracker, float aspect_ratio);

#ifdef __cplusplus
}
#endif

#endif // CAMAR_FACE_TRACKER_H
