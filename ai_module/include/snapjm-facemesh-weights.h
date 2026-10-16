#ifndef SNAPJM_FACEMESH_WEIGHTS_H
#define SNAPJM_FACEMESH_WEIGHTS_H

#include <stdint.h>
#include <stdbool.h>
#include <math.h>
#include "snapjm-face-tracker.h"
#include "snapjm-facemesh-topology.h"
#include "snapjm-facemesh-regions.h"

// 36 indices del contorno exterior / ovalo facial de MediaPipe FaceMesh
static const uint16_t SNAPJM_FACEMESH_OVAL_INDICES[36] = {
    10,  338, 297, 332, 284, 251, 389, 356, 454, 323, 361, 288,
    397, 365, 379, 378, 400, 377, 152, 148, 176, 149, 150, 136,
    172, 58,  132, 93,  234, 127, 162, 21,  54,  103, 67,  109
};

// 11 indices que componen el arco superior de la frente (de sien a sien pasando por el vertice 10)
static const uint16_t SNAPJM_FOREHEAD_HAIRLINE_ARC[11] = {
    21, 54, 103, 67, 109, 10, 338, 297, 332, 284, 251
};

static const uint16_t outer_lips[20] = {
    61, 185, 40, 39, 37, 0, 267, 269, 270, 409, 291, 375, 321, 405, 314, 17, 84, 181, 91, 146
};
static const uint16_t inner_lips[20] = {
    78, 191, 80, 81, 82, 13, 312, 311, 310, 415, 308, 324, 318, 402, 317, 14, 87, 178, 88, 95
};
static const uint16_t left_eye[16] = {
    33, 7, 163, 144, 145, 153, 154, 155, 133, 173, 157, 158, 159, 160, 161, 246
};
static const uint16_t right_eye[16] = {
    362, 382, 381, 380, 374, 373, 390, 249, 263, 466, 388, 387, 386, 385, 384, 398
};

static inline bool snapjm_is_forehead_hairline_index(uint16_t idx)
{
    for (int i = 0; i < 11; i++) {
        if (SNAPJM_FOREHEAD_HAIRLINE_ARC[i] == idx) return true;
    }
    return false;
}

// Proyecta virtualmente los vertices del arco frontal hacia la raiz del cabello
static inline void snapjm_compute_hairline_skirt(
    const snapjm_face_mesh_vertex_t *landmarks,
    snapjm_face_mesh_vertex_t skirt_pts[11],
    float extension_factor)
{
    if (!landmarks || !skirt_pts) return;

    // Centro del entrecejo / glabella (vertice 9) y vertice 10 (cima de la frente)
    float g_x = landmarks[9].x;
    float g_y = landmarks[9].y;
    float apex_x = landmarks[10].x;
    float apex_y = landmarks[10].y;

    float dx_forehead = apex_x - g_x;
    float dy_forehead = apex_y - g_y;
    float h_forehead = sqrtf(dx_forehead * dx_forehead + dy_forehead * dy_forehead);
    if (h_forehead < 0.001f) h_forehead = 0.08f;

    float ext_dist = h_forehead * extension_factor;

    for (int k = 0; k < 11; k++) {
        uint16_t idx = SNAPJM_FOREHEAD_HAIRLINE_ARC[k];
        float vx = landmarks[idx].x;
        float vy = landmarks[idx].y;

        float dir_x = vx - g_x;
        float dir_y = vy - g_y;
        float d_len = sqrtf(dir_x * dir_x + dir_y * dir_y);
        if (d_len > 0.0001f) {
            dir_x /= d_len;
            dir_y /= d_len;
        } else {
            dir_x = 0.0f;
            dir_y = -1.0f;
        }

        skirt_pts[k].x = vx + dir_x * ext_dist;
        skirt_pts[k].y = vy + dir_y * ext_dist;
        skirt_pts[k].z = landmarks[idx].z;
    }
}

static float   s_snapjm_vertex_skin_weights[SNAPJM_NUM_FACEMESH_VERTICES];
static float   s_snapjm_vertex_forehead_weights[SNAPJM_NUM_FACEMESH_VERTICES];
static float   s_snapjm_vertex_undereye_weights[SNAPJM_NUM_FACEMESH_VERTICES];
static float   s_snapjm_vertex_nasolabial_weights[SNAPJM_NUM_FACEMESH_VERTICES];
static uint8_t s_snapjm_tri_type[SNAPJM_NUM_FACEMESH_TRIANGLES];
static bool    s_snapjm_topology_init = false;

static inline void snapjm_init_topology_and_weights(void)
{
    if (s_snapjm_topology_init) return;

    // -------------------------------------------------------------
    // 1. Clasificacion directa de los 898 triangulos desde SNAPJM_FACEMESH_TRI_TYPE
    // (0=Piel, 1=Labios, 2=Boca/Dientes, 3=Ojos, 4=Bolsas bajo ojos)
    // -------------------------------------------------------------
    for (int t = 0; t < SNAPJM_NUM_FACEMESH_TRIANGLES; t++) {
        s_snapjm_tri_type[t] = SNAPJM_FACEMESH_TRI_TYPE[t];
    }

    // -------------------------------------------------------------
    // 2. Pesos progresivos de piel (Anclaje estricto de bordes y rasgos)
    // -------------------------------------------------------------
    bool is_protected_vert[SNAPJM_NUM_FACEMESH_VERTICES];
    for (int i = 0; i < SNAPJM_NUM_FACEMESH_VERTICES; i++) {
        s_snapjm_vertex_skin_weights[i] = 1.0f;
        is_protected_vert[i] = false;
    }

    // A. Todo el perimetro exterior del ovalo facial = 0.0f
    for (int k = 0; k < 36; k++) {
        uint16_t idx = SNAPJM_FACEMESH_OVAL_INDICES[k];
        if (idx < SNAPJM_NUM_FACEMESH_VERTICES) {
            s_snapjm_vertex_skin_weights[idx] = 0.0f;
            is_protected_vert[idx] = true;
        }
    }

    // B. Todos los vertices pertenecientes a Ojos, Labios y Cavidad Bucal = 0.0f (NUNCA SON PIEL)
    for (int t = 0; t < SNAPJM_NUM_FACEMESH_TRIANGLES; t++) {
        uint8_t type = SNAPJM_FACEMESH_TRI_TYPE[t];
        if (type == 1 || type == 2 || type == 3) {
            uint16_t i0 = SNAPJM_FACEMESH_TRIANGLES[t][0];
            uint16_t i1 = SNAPJM_FACEMESH_TRIANGLES[t][1];
            uint16_t i2 = SNAPJM_FACEMESH_TRIANGLES[t][2];
            s_snapjm_vertex_skin_weights[i0] = 0.0f;
            s_snapjm_vertex_skin_weights[i1] = 0.0f;
            s_snapjm_vertex_skin_weights[i2] = 0.0f;
            is_protected_vert[i0] = true;
            is_protected_vert[i1] = true;
            is_protected_vert[i2] = true;
        }
    }

    // C. Difusion armonica Laplaciana sobre la topologia 3D:
    // Genera un degradado continuo C1 organico de la piel hacia los bordes y rasgos protegidos
    float temp_diffuse[SNAPJM_NUM_FACEMESH_VERTICES];
    float sum_diffuse[SNAPJM_NUM_FACEMESH_VERTICES];
    int count_diffuse[SNAPJM_NUM_FACEMESH_VERTICES];

    for (int pass = 0; pass < 4; pass++) {
        memset(sum_diffuse, 0, sizeof(sum_diffuse));
        memset(count_diffuse, 0, sizeof(count_diffuse));

        for (int t = 0; t < SNAPJM_NUM_FACEMESH_TRIANGLES; t++) {
            uint16_t i0 = SNAPJM_FACEMESH_TRIANGLES[t][0];
            uint16_t i1 = SNAPJM_FACEMESH_TRIANGLES[t][1];
            uint16_t i2 = SNAPJM_FACEMESH_TRIANGLES[t][2];

            sum_diffuse[i0] += s_snapjm_vertex_skin_weights[i1] + s_snapjm_vertex_skin_weights[i2];
            count_diffuse[i0] += 2;

            sum_diffuse[i1] += s_snapjm_vertex_skin_weights[i0] + s_snapjm_vertex_skin_weights[i2];
            count_diffuse[i1] += 2;

            sum_diffuse[i2] += s_snapjm_vertex_skin_weights[i0] + s_snapjm_vertex_skin_weights[i1];
            count_diffuse[i2] += 2;
        }

        for (int i = 0; i < SNAPJM_NUM_FACEMESH_VERTICES; i++) {
            if (is_protected_vert[i]) {
                temp_diffuse[i] = 0.0f; // Anclado estricto en 0.0f para ojos, labios y borde
            } else if (count_diffuse[i] > 0) {
                float avg = sum_diffuse[i] / (float)count_diffuse[i];
                temp_diffuse[i] = 0.5f * s_snapjm_vertex_skin_weights[i] + 0.5f * avg;
            } else {
                temp_diffuse[i] = s_snapjm_vertex_skin_weights[i];
            }
        }
        memcpy(s_snapjm_vertex_skin_weights, temp_diffuse, sizeof(s_snapjm_vertex_skin_weights));
    }

    // -------------------------------------------------------------
    // 3. Zonas Antiarrugas y Ojeras Focalizadas
    // -------------------------------------------------------------
    static const uint16_t forehead_core[] = {
        10, 338, 297, 332, 284, 251, 389, 356, 109, 67, 103, 54, 21, 162, 127,
        9, 8, 168, 107, 66, 105, 63, 70, 336, 296, 334, 293, 300, 151, 68,
        69, 108, 104, 65, 55, 285, 295, 282, 337, 298, 299, 193, 417
    };
    static const uint16_t undereye_core[] = {
        111, 116, 117, 118, 119, 120, 121, 100, 126, 142, 205, 206, 207, 212, 214, 192, 138, 135, 230, 229, 228, 36, 123,
        340, 345, 346, 347, 348, 349, 350, 329, 355, 371, 425, 426, 427, 432, 434, 416, 367, 364, 450, 449, 448, 266, 352,
        130, 247, 30, 29, 27, 28, 56, 190, 243, 25, 110,
        359, 467, 260, 259, 257, 258, 286, 414, 463, 255, 339
    };
    static const uint16_t nasolabial_core[] = {
        92, 165, 167, 203, 206, 216, 187, 50, 101, 118, 119, 120, 47, 114, 217, 198, 209, 49, 202, 210, 211,
        322, 391, 393, 423, 426, 436, 411, 280, 330, 347, 348, 349, 277, 343, 437, 420, 429, 279, 422, 430, 431,
        58, 172, 136, 150, 149, 176, 148,
        288, 397, 365, 379, 378, 400, 377
    };

    for (int i = 0; i < SNAPJM_NUM_FACEMESH_VERTICES; i++) {
        s_snapjm_vertex_forehead_weights[i] = 0.0f;
        s_snapjm_vertex_undereye_weights[i] = 0.0f;
        s_snapjm_vertex_nasolabial_weights[i] = 0.0f;
    }

    for (size_t i = 0; i < sizeof(forehead_core) / sizeof(forehead_core[0]); i++) {
        s_snapjm_vertex_forehead_weights[forehead_core[i]] = 1.0f;
    }
    for (size_t i = 0; i < sizeof(undereye_core) / sizeof(undereye_core[0]); i++) {
        s_snapjm_vertex_undereye_weights[undereye_core[i]] = 1.0f;
    }
    for (size_t i = 0; i < sizeof(nasolabial_core) / sizeof(nasolabial_core[0]); i++) {
        s_snapjm_vertex_nasolabial_weights[nasolabial_core[i]] = 1.0f;
    }

    // Difusion suave de 1 anillo para transicion organica sin bordes duros
    for (int t = 0; t < SNAPJM_NUM_FACEMESH_TRIANGLES; t++) {
        uint16_t i0 = SNAPJM_FACEMESH_TRIANGLES[t][0];
        uint16_t i1 = SNAPJM_FACEMESH_TRIANGLES[t][1];
        uint16_t i2 = SNAPJM_FACEMESH_TRIANGLES[t][2];

        if (s_snapjm_vertex_forehead_weights[i0] == 1.0f || s_snapjm_vertex_forehead_weights[i1] == 1.0f || s_snapjm_vertex_forehead_weights[i2] == 1.0f) {
            if (s_snapjm_vertex_forehead_weights[i0] == 0.0f) s_snapjm_vertex_forehead_weights[i0] = 0.50f;
            if (s_snapjm_vertex_forehead_weights[i1] == 0.0f) s_snapjm_vertex_forehead_weights[i1] = 0.50f;
            if (s_snapjm_vertex_forehead_weights[i2] == 0.0f) s_snapjm_vertex_forehead_weights[i2] = 0.50f;
        }
        if (s_snapjm_vertex_undereye_weights[i0] == 1.0f || s_snapjm_vertex_undereye_weights[i1] == 1.0f || s_snapjm_vertex_undereye_weights[i2] == 1.0f) {
            if (s_snapjm_vertex_undereye_weights[i0] == 0.0f) s_snapjm_vertex_undereye_weights[i0] = 0.50f;
            if (s_snapjm_vertex_undereye_weights[i1] == 0.0f) s_snapjm_vertex_undereye_weights[i1] = 0.50f;
            if (s_snapjm_vertex_undereye_weights[i2] == 0.0f) s_snapjm_vertex_undereye_weights[i2] = 0.50f;
        }
        if (s_snapjm_vertex_nasolabial_weights[i0] == 1.0f || s_snapjm_vertex_nasolabial_weights[i1] == 1.0f || s_snapjm_vertex_nasolabial_weights[i2] == 1.0f) {
            if (s_snapjm_vertex_nasolabial_weights[i0] == 0.0f) s_snapjm_vertex_nasolabial_weights[i0] = 0.50f;
            if (s_snapjm_vertex_nasolabial_weights[i1] == 0.0f) s_snapjm_vertex_nasolabial_weights[i1] = 0.50f;
            if (s_snapjm_vertex_nasolabial_weights[i2] == 0.0f) s_snapjm_vertex_nasolabial_weights[i2] = 0.50f;
        }
    }

    // Exclusiones estrictas: Ojos y Labios internos nunca reciben peso
    for (int i = 0; i < 20; i++) {
        s_snapjm_vertex_forehead_weights[inner_lips[i]] = 0.0f;
        s_snapjm_vertex_undereye_weights[inner_lips[i]] = 0.0f;
        s_snapjm_vertex_nasolabial_weights[inner_lips[i]] = 0.0f;
        s_snapjm_vertex_forehead_weights[outer_lips[i]] = 0.0f;
        s_snapjm_vertex_undereye_weights[outer_lips[i]] = 0.0f;
    }
    for (int i = 0; i < 16; i++) {
        s_snapjm_vertex_forehead_weights[left_eye[i]] = 0.0f;
        s_snapjm_vertex_undereye_weights[left_eye[i]] = 0.0f;
        s_snapjm_vertex_nasolabial_weights[left_eye[i]] = 0.0f;
        s_snapjm_vertex_forehead_weights[right_eye[i]] = 0.0f;
        s_snapjm_vertex_undereye_weights[right_eye[i]] = 0.0f;
        s_snapjm_vertex_nasolabial_weights[right_eye[i]] = 0.0f;
    }

    s_snapjm_topology_init = true;
}

static inline float snapjm_get_vertex_skin_weight(uint16_t idx)
{
    if (!s_snapjm_topology_init) {
        snapjm_init_topology_and_weights();
    }
    if (idx >= SNAPJM_NUM_FACEMESH_VERTICES) return 1.0f;
    return s_snapjm_vertex_skin_weights[idx];
}

static inline float snapjm_get_vertex_forehead_weight(uint16_t idx)
{
    if (!s_snapjm_topology_init) {
        snapjm_init_topology_and_weights();
    }
    if (idx >= SNAPJM_NUM_FACEMESH_VERTICES) return 0.0f;
    return s_snapjm_vertex_forehead_weights[idx];
}

static inline float snapjm_get_vertex_undereye_weight(uint16_t idx)
{
    if (!s_snapjm_topology_init) {
        snapjm_init_topology_and_weights();
    }
    if (idx >= SNAPJM_NUM_FACEMESH_VERTICES) return 0.0f;
    return s_snapjm_vertex_undereye_weights[idx];
}

static inline float snapjm_get_vertex_nasolabial_weight(uint16_t idx)
{
    if (!s_snapjm_topology_init) {
        snapjm_init_topology_and_weights();
    }
    if (idx >= SNAPJM_NUM_FACEMESH_VERTICES) return 0.0f;
    return s_snapjm_vertex_nasolabial_weights[idx];
}

static inline uint8_t snapjm_get_triangle_type(int t)
{
    if (!s_snapjm_topology_init) {
        snapjm_init_topology_and_weights();
    }
    if (t < 0 || t >= SNAPJM_NUM_FACEMESH_TRIANGLES) return 0;
    return s_snapjm_tri_type[t];
}

#endif // SNAPJM_FACEMESH_WEIGHTS_H
