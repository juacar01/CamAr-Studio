use crate::types::{FaceLandmarks, FaceLighting, Point2f, Rect2f, StudioConfig};
use std::ffi::{c_char, c_void, CString};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

extern "system" {
    fn LoadLibraryA(lpLibFileName: *const u8) -> *mut c_void;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const u8) -> *mut c_void;
    fn FreeLibrary(hModule: *mut c_void) -> i32;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FaceMeshVertex {
    pub x: f32, // Coordenada normalizada 0..1 en cuadro original
    pub y: f32, // Coordenada normalizada 0..1 en cuadro original
    pub z: f32, // Profundidad relativa
}

type FnAcquire = unsafe extern "C" fn(
    detector_model_path: *const c_char,
    mesh_model_path: *const c_char,
    selfie_model_path: *const c_char,
) -> *mut c_void;
type FnRelease = unsafe extern "C" fn(tracker: *mut c_void);
type FnNeedsFrame = unsafe extern "C" fn(tracker: *mut c_void) -> bool;
type FnPostFrame = unsafe extern "C" fn(
    tracker: *mut c_void,
    rgba_data: *const u8,
    width: i32,
    height: i32,
    linesize: i32,
);
type FnTrackFaceSync = unsafe extern "C" fn(
    tracker: *mut c_void,
    rgba_data: *const u8,
    width: i32,
    height: i32,
    linesize: i32,
);
type FnGetMask = unsafe extern "C" fn(
    tracker: *mut c_void,
    out_rgba_mask: *mut u8,
    target_width: i32,
    target_height: i32,
    inout_version: *mut u64,
) -> bool;
type FnGetPersonMask = unsafe extern "C" fn(
    tracker: *mut c_void,
    out_person_mask: *mut u8,
    target_width: i32,
    target_height: i32,
    inout_version: *mut u64,
) -> bool;
type FnHasFace = unsafe extern "C" fn(tracker: *mut c_void) -> bool;
type FnGetMaskAnchor = unsafe extern "C" fn(tracker: *mut c_void, out_x: *mut f32, out_y: *mut f32) -> bool;
type FnGetFaceLighting = unsafe extern "C" fn(tracker: *mut c_void, out_lighting: *mut FaceLighting) -> bool;
type FnGetMeshLandmarks = unsafe extern "C" fn(
    tracker: *mut c_void,
    out_landmarks: *mut FaceMeshVertex,
    max_landmarks: i32,
    inout_version: *mut u64,
) -> bool;

struct TrackerBindings {
    dll_handle: *mut c_void,
    acquire: FnAcquire,
    release: FnRelease,
    needs_frame: FnNeedsFrame,
    post_frame: FnPostFrame,
    track_face_sync: Option<FnTrackFaceSync>,
    get_mask: FnGetMask,
    get_person_mask: FnGetPersonMask,
    has_face: FnHasFace,
    _get_mask_anchor: FnGetMaskAnchor,
    get_face_lighting: FnGetFaceLighting,
    get_mesh_landmarks: FnGetMeshLandmarks,
}

impl TrackerBindings {
    fn load() -> Option<Self> {
        let dll_candidates = [
            "camar_tracker.dll",
            "ai_module/build/Release/camar_tracker.dll",
            "d:/proyectos/CamAr Studio/camar_tracker.dll",
            "d:/proyectos/CamAr Studio/ai_module/build/Release/camar_tracker.dll",
        ];

        let mut handle = std::ptr::null_mut();
        for candidate in &dll_candidates {
            let path_buf = if let Some(p) = find_file(candidate) {
                p
            } else {
                PathBuf::from(candidate)
            };
            if let Ok(c_str) = CString::new(path_buf.to_string_lossy().as_bytes()) {
                let h = unsafe { LoadLibraryA(c_str.as_ptr() as *const u8) };
                if !h.is_null() {
                    crate::log_info!("AiEngine", "Módulo IA cargado exitosamente desde: {:?}", path_buf);
                    handle = h;
                    break;
                }
            }
        }

        if handle.is_null() {
            crate::log_warn!(
                "AiEngine",
                "No se pudo cargar camar_tracker.dll. Operando en modo fallback."
            );
            return None;
        }

        unsafe {
            macro_rules! get_sym {
                ($sym:expr, $ty:ty) => {{
                    let fn_ptr = GetProcAddress(handle, concat!($sym, "\0").as_ptr());
                    if fn_ptr.is_null() {
                        crate::log_error!("AiEngine", "Símbolo C faltante en camar_tracker.dll: {}", $sym);
                        FreeLibrary(handle);
                        return None;
                    }
                    std::mem::transmute::<*mut c_void, $ty>(fn_ptr)
                }};
            }

            let sync_ptr = GetProcAddress(handle, "snapjm_face_tracker_track_face_sync\0".as_ptr());
            let track_face_sync = if !sync_ptr.is_null() {
                Some(std::mem::transmute::<*mut c_void, FnTrackFaceSync>(sync_ptr))
            } else {
                None
            };

            Some(Self {
                dll_handle: handle,
                acquire: get_sym!("snapjm_face_tracker_acquire", FnAcquire),
                release: get_sym!("snapjm_face_tracker_release", FnRelease),
                needs_frame: get_sym!("snapjm_face_tracker_needs_frame", FnNeedsFrame),
                post_frame: get_sym!("snapjm_face_tracker_post_frame", FnPostFrame),
                track_face_sync,
                get_mask: get_sym!("snapjm_face_tracker_get_mask", FnGetMask),
                get_person_mask: get_sym!("snapjm_face_tracker_get_person_mask", FnGetPersonMask),
                has_face: get_sym!("snapjm_face_tracker_has_face", FnHasFace),
                _get_mask_anchor: get_sym!("snapjm_face_tracker_get_mask_anchor", FnGetMaskAnchor),
                get_face_lighting: get_sym!("snapjm_face_tracker_get_face_lighting", FnGetFaceLighting),
                get_mesh_landmarks: get_sym!("snapjm_face_tracker_get_mesh_landmarks", FnGetMeshLandmarks),
            })
        }
    }
}

fn find_file(relative: &str) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join(relative);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    let cur = PathBuf::from(relative);
    if cur.exists() {
        return Some(cur);
    }
    let dev = PathBuf::from("d:/proyectos/CamAr Studio").join(relative);
    if dev.exists() {
        return Some(dev);
    }
    None
}

#[derive(Clone)]
pub struct SharedAiState {
    pub faces: Vec<FaceLandmarks>,
    pub segmentation_mask: Arc<Vec<u8>>,
    pub skin_mask: Arc<Vec<u8>>,
    pub feature_mask: Arc<Vec<u8>>,
    pub face_lighting: FaceLighting,
    pub ai_ms: f32,
}

impl Default for SharedAiState {
    fn default() -> Self {
        Self {
            faces: Vec::new(),
            segmentation_mask: Arc::new(Vec::new()),
            skin_mask: Arc::new(Vec::new()),
            feature_mask: Arc::new(Vec::new()),
            face_lighting: FaceLighting::default(),
            ai_ms: 0.0,
        }
    }
}

/// Motor de IA Híbrido de Latencia Cero:
/// - FaceMesh 468 Landmarks + Máscara Facial 320x240: Ejecución 1:1 síncrona sobre el cuadro actual (0.0 ms de retraso)
/// - Selfie Segmentation 256x256: Ejecución asíncrona en segundo plano en GPU DirectML
pub struct AiEngine {
    internal: InternalAiEngine,
}

unsafe impl Send for AiEngine {}
unsafe impl Sync for AiEngine {}

impl AiEngine {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            internal: InternalAiEngine::new(width, height),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.internal.resize(width, height);
    }

    /// Procesa el cuadro actual en sincronía 1:1 para los 468 landmarks faciales (0 frames de desfase)
    pub fn process_frame_sync(&mut self, frame: &[u8], config: &StudioConfig) -> SharedAiState {
        let start = Instant::now();
        let faces = self.internal.process_frame(frame, config);
        let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;

        SharedAiState {
            faces,
            segmentation_mask: Arc::clone(&self.internal.arc_mask),
            skin_mask: Arc::clone(&self.internal.arc_skin),
            feature_mask: Arc::clone(&self.internal.arc_feature),
            face_lighting: self.internal.last_lighting,
            ai_ms: elapsed_ms,
        }
    }
}

struct InternalAiEngine {
    width: usize,
    height: usize,
    tracker_ptr: *mut c_void,
    bindings: Option<TrackerBindings>,
    mesh_vertices_buf: Vec<FaceMeshVertex>,
    face_mask_320: Vec<u8>,
    compact_person_mask: Vec<u8>,
    person_mask_256: Vec<u8>,
    arc_mask: Arc<Vec<u8>>,
    arc_skin: Arc<Vec<u8>>,
    arc_feature: Arc<Vec<u8>>,
    last_mesh_version: u64,
    last_face_mask_version: u64,
    last_person_version: u64,
    prev_face: FaceLandmarks,
    face_lost_count: u32,
    last_lighting: FaceLighting,
}

impl InternalAiEngine {
    fn new(width: usize, height: usize) -> Self {
        let bindings = TrackerBindings::load();
        let mut tracker_ptr = std::ptr::null_mut();

        if let Some(ref b) = bindings {
            let det_path = find_file("models/version-RFB-320.onnx");
            let mesh_path = find_file("models/FaceMesh.onnx");
            let selfie_path = find_file("models/selfie_multiclass_256x256.onnx")
                .or_else(|| find_file("models/selfie_segmentation.onnx"));

            let c_det = det_path.as_ref().and_then(|p| CString::new(p.to_string_lossy().as_bytes()).ok());
            let c_mesh = mesh_path.as_ref().and_then(|p| CString::new(p.to_string_lossy().as_bytes()).ok());
            let c_selfie = selfie_path.as_ref().and_then(|p| CString::new(p.to_string_lossy().as_bytes()).ok());

            let p_det = c_det.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null());
            let p_mesh = c_mesh.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null());
            let p_selfie = c_selfie.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null());

            if !p_mesh.is_null() {
                tracker_ptr = unsafe { (b.acquire)(p_det, p_mesh, p_selfie) };
                if !tracker_ptr.is_null() {
                    crate::log_info!(
                        "AiEngine",
                        "Tracker MediaPipe FaceMesh 3D (Sync 0ms) & Selfie Segmentation DirectML GPU inicializado!"
                    );
                }
            }
        }

        let seg_buf = vec![255u8; 256 * 256];
        let feat_buf = vec![0u8; 320 * 240 * 4];

        Self {
            width,
            height,
            tracker_ptr,
            bindings,
            mesh_vertices_buf: vec![FaceMeshVertex::default(); 468],
            face_mask_320: feat_buf.clone(),
            compact_person_mask: seg_buf.clone(),
            person_mask_256: vec![0u8; 256 * 256 * 4],
            arc_mask: Arc::new(seg_buf),
            arc_skin: Arc::new(Vec::new()),
            arc_feature: Arc::new(feat_buf),
            last_mesh_version: 0,
            last_face_mask_version: 0,
            last_person_version: 0,
            prev_face: FaceLandmarks::default(),
            face_lost_count: 0,
            last_lighting: FaceLighting::default(),
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;

        // Forzar actualización inmediata en el siguiente frame
        self.last_face_mask_version = 0;
        self.last_person_version = 0;
    }

    fn smooth_landmarks(_current: &mut FaceLandmarks, _previous: &FaceLandmarks, _base_alpha: f32) {
        // El filtrado de sub-píxel contra ruido de sensor ya se ejecuta directamente en C++
        // con latencia 0.0 ms (a = 1.0f para cualquier movimiento >= 0.25 unidades).
        // No aplicar segundo filtro en Rust para garantizar acoplamiento 1:1 instantáneo.
    }

    fn process_frame(&mut self, rgba_data: &[u8], config: &StudioConfig) -> Vec<FaceLandmarks> {
        let mut faces = Vec::new();

        if let (Some(b), true) = (&self.bindings, !self.tracker_ptr.is_null()) {
            // 1. Ejecutar FaceMesh 468 en sincronía 1:1 con este cuadro exacto + alimentar segmentador de fondo
            if !rgba_data.is_empty() {
                if let Some(track_sync) = b.track_face_sync {
                    unsafe {
                        track_sync(
                            self.tracker_ptr,
                            rgba_data.as_ptr(),
                            self.width as i32,
                            self.height as i32,
                            (self.width * 4) as i32,
                        );
                    }
                } else {
                    let needs = unsafe { (b.needs_frame)(self.tracker_ptr) };
                    if needs {
                        unsafe {
                            (b.post_frame)(
                                self.tracker_ptr,
                                rgba_data.as_ptr(),
                                self.width as i32,
                                self.height as i32,
                                (self.width * 4) as i32,
                            );
                        }
                    }
                }
            }

            // 2. Obtener fotometría facial calibrada de la piel detectada
            let mut lighting = FaceLighting::default();
            let lighting_ok = unsafe { (b.get_face_lighting)(self.tracker_ptr, &mut lighting) };
            if lighting_ok && lighting.has_face {
                self.last_lighting = lighting;
            }

            // 3. Obtener landmarks 3D del FaceMesh real (468 vértices)
            let has_face = unsafe { (b.has_face)(self.tracker_ptr) };
            if has_face && config.face_detection {
                let _updated = unsafe {
                    (b.get_mesh_landmarks)(
                        self.tracker_ptr,
                        self.mesh_vertices_buf.as_mut_ptr(),
                        468,
                        &mut self.last_mesh_version,
                    )
                };

                let w_f32 = self.width as f32;
                let h_f32 = self.height as f32;

                let mut min_x = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_y = f32::NEG_INFINITY;

                for v in &self.mesh_vertices_buf {
                    let px = v.x * w_f32;
                    let py = v.y * h_f32;
                    if px < min_x { min_x = px; }
                    if px > max_x { max_x = px; }
                    if py < min_y { min_y = py; }
                    if py > max_y { max_y = py; }
                }

                // Margen del 5% al contorno de la cabeza
                let margin_x = (max_x - min_x) * 0.05;
                let margin_y = (max_y - min_y) * 0.05;
                let bb_x = (min_x - margin_x).max(0.0);
                let bb_y = (min_y - margin_y).max(0.0);
                let bb_w = ((max_x - min_x) + margin_x * 2.0).min(w_f32 - bb_x);
                let bb_h = ((max_y - min_y) + margin_y * 2.0).min(h_f32 - bb_y);

                // Puntos de referencia facial 3D completos (468 vértices de FaceMesh)
                let mut landmarks = Vec::with_capacity(self.mesh_vertices_buf.len());
                for v in &self.mesh_vertices_buf {
                    landmarks.push(Point2f {
                        x: v.x * w_f32,
                        y: v.y * h_f32,
                    });
                }

                let mut face = FaceLandmarks {
                    bounding_box: Rect2f {
                        x: bb_x,
                        y: bb_y,
                        width: bb_w,
                        height: bb_h,
                    },
                    confidence: 0.99,
                    landmarks,
                    detected: true,
                };

                Self::smooth_landmarks(&mut face, &self.prev_face, 0.40);
                self.prev_face = face.clone();
                self.face_lost_count = 0;
                faces.push(face);
            } else if self.prev_face.detected && self.face_lost_count < 10 {
                // Buffer de persistencia para evitar parpadeos si se pierde el tracking momentáneamente (hasta ~330ms)
                self.face_lost_count += 1;
                faces.push(self.prev_face.clone());
            } else if !has_face {
                self.prev_face.detected = false;
            }

            // 4. Segmentación Real de Persona (MediaPipe Selfie Segmentation)
            if config.person_segmentation {
                let seg_updated = unsafe {
                    (b.get_person_mask)(
                        self.tracker_ptr,
                        self.person_mask_256.as_mut_ptr(),
                        256,
                        256,
                        &mut self.last_person_version,
                    )
                };

                // Únicamente actualizar búfer compacto si el modelo generó una nueva versión de máscara
                if seg_updated {
                    let seg_256 = &self.person_mask_256;
                    for i in 0..256 * 256 {
                        self.compact_person_mask[i] = seg_256[i * 4];
                    }
                    self.arc_mask = Arc::new(self.compact_person_mask.clone());
                }
            } else if self.compact_person_mask.first().copied() != Some(255) {
                self.compact_person_mask.fill(255);
                self.arc_mask = Arc::new(self.compact_person_mask.clone());
            }

            // 5. Máscara de Rasgos Faciales y Piel de FaceMesh 3D (R: Piel, G: Dientes, B: Ojos, A: Labios)
            if config.face_detection || config.beauty_enabled {
                let mask_updated = unsafe {
                    (b.get_mask)(
                        self.tracker_ptr,
                        self.face_mask_320.as_mut_ptr(),
                        320,
                        240,
                        &mut self.last_face_mask_version,
                    )
                };

                // Zero-Allocation / Zero-Rayon: comparte directamente la máscara 320x240 en caché L2
                if mask_updated {
                    self.arc_feature = Arc::new(self.face_mask_320.clone());
                }
            } else if self.face_mask_320.first().copied() != Some(0) {
                self.face_mask_320.fill(0);
                self.arc_feature = Arc::new(self.face_mask_320.clone());
            }
        } else {
            // Modo Fallback si el módulo no está disponible
            if self.compact_person_mask.first().copied() != Some(255) {
                self.compact_person_mask.fill(255);
                self.face_mask_320.fill(0);
                self.arc_mask = Arc::new(self.compact_person_mask.clone());
                self.arc_feature = Arc::new(self.face_mask_320.clone());
            }
        }

        faces
    }
}

impl Drop for InternalAiEngine {
    fn drop(&mut self) {
        if let (Some(b), true) = (&self.bindings, !self.tracker_ptr.is_null()) {
            unsafe {
                (b.release)(self.tracker_ptr);
            }
            self.tracker_ptr = std::ptr::null_mut();
        }
        if let Some(b) = self.bindings.take() {
            unsafe {
                FreeLibrary(b.dll_handle);
            }
        }
    }
}
