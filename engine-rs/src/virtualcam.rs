use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, CreateMutexW, ReleaseMutex, SetEvent, WaitForSingleObject,
};

// Protocolo DirectShow UnityCapture de ultra-alta velocidad
const HEADER_SIZE: usize = 32;
const MAX_DATA_SIZE: usize = 3840 * 2160 * 4 * 2; // ~64 MB
const TOTAL_SHM_SIZE: usize = HEADER_SIZE + MAX_DATA_SIZE;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SharedMemHeader {
    pub max_size: u32,       // 0: DWORD maxSize
    pub width: i32,          // 4: int width
    pub height: i32,         // 8: int height
    pub stride: i32,         // 12: int stride (in pixels)
    pub format: i32,         // 16: int format (0 = FORMAT_UINT8)
    pub resize_mode: i32,    // 20: int resizeMode (1 = RESIZEMODE_LINEAR)
    pub mirror_mode: i32,    // 24: int mirrorMode (0 = MIRRORMODE_DISABLED)
    pub timeout: i32,        // 28: int timeout (0x7FFFFFFF ms)
}

pub struct VirtualCamManager {
    device_name: String,
    width: usize,
    height: usize,
    fps: u32,
    active: AtomicBool,
    frame_index: u32,
    file_mapping: HANDLE,
    shared_buffer: *mut u8,
    mutex: HANDLE,
    event_want: HANDLE,
    event_sent: HANDLE,
    event_stream_active: HANDLE,
}

unsafe impl Send for VirtualCamManager {}
unsafe impl Sync for VirtualCamManager {}

impl VirtualCamManager {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            device_name: "CamAr Studio Virtual Camera".to_string(),
            width,
            height,
            fps: 30,
            active: AtomicBool::new(false),
            frame_index: 0,
            file_mapping: 0 as HANDLE,
            shared_buffer: ptr::null_mut(),
            mutex: 0 as HANDLE,
            event_want: 0 as HANDLE,
            event_sent: 0 as HANDLE,
            event_stream_active: 0 as HANDLE,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize, fps: u32) {
        self.width = width;
        self.height = height;
        self.fps = fps.clamp(1, 30);
    }

    pub fn start(&mut self) -> bool {
        if self.active.load(Ordering::SeqCst) {
            return true;
        }

        // Nombres globales del filtro DirectShow de CamAr Studio
        let shm_name: Vec<u16> = "UnityCapture_Data\0".encode_utf16().collect();
        let mutex_name: Vec<u16> = "UnityCapture_Mutx\0".encode_utf16().collect();
        let event_want_name: Vec<u16> = "UnityCapture_Want\0".encode_utf16().collect();
        let event_sent_name: Vec<u16> = "UnityCapture_Sent\0".encode_utf16().collect();
        let stream_act_name: Vec<u16> = "CamAr_Streaming_Active_Event\0".encode_utf16().collect();

        unsafe {
            let mapping = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                ptr::null(),
                PAGE_READWRITE,
                0,
                TOTAL_SHM_SIZE as u32,
                shm_name.as_ptr(),
            );

            if mapping == 0 as HANDLE {
                crate::log_error!("VirtualCam-RS", "Error al crear memoria compartida DirectShow.");
                return false;
            }

            let view = MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, 0);
            if view.Value.is_null() {
                CloseHandle(mapping);
                crate::log_error!("VirtualCam-RS", "Error al mapear vista de archivo DirectShow.");
                return false;
            }

            let mutex = CreateMutexW(ptr::null(), 0, mutex_name.as_ptr());
            let event_want = CreateEventW(ptr::null(), 0, 0, event_want_name.as_ptr());
            let event_sent = CreateEventW(ptr::null(), 0, 0, event_sent_name.as_ptr());
            let event_stream = CreateEventW(ptr::null(), 1, 0, stream_act_name.as_ptr());

            self.file_mapping = mapping;
            self.shared_buffer = view.Value as *mut u8;
            self.mutex = mutex;
            self.event_want = event_want;
            self.event_sent = event_sent;
            self.event_stream_active = event_stream;

            if self.event_stream_active != 0 as HANDLE {
                SetEvent(self.event_stream_active);
            }
        }

        self.active.store(true, Ordering::SeqCst);
        crate::log_info!(
            "VirtualCam-RS",
            "Dispositivo '{}' iniciado y enlazado a DirectShow a {}x{} @ {} FPS",
            self.device_name, self.width, self.height, self.fps
        );
        true
    }

    pub fn stop(&mut self) {
        if !self.active.load(Ordering::SeqCst) {
            return;
        }

        self.active.store(false, Ordering::SeqCst);
        unsafe {
            if !self.shared_buffer.is_null() {
                use windows_sys::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS;
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.shared_buffer as *mut _,
                });
                self.shared_buffer = ptr::null_mut();
            }
            if self.file_mapping != 0 as HANDLE {
                CloseHandle(self.file_mapping);
                self.file_mapping = 0 as HANDLE;
            }
            if self.mutex != 0 as HANDLE {
                CloseHandle(self.mutex);
                self.mutex = 0 as HANDLE;
            }
            if self.event_want != 0 as HANDLE {
                CloseHandle(self.event_want);
                self.event_want = 0 as HANDLE;
            }
            if self.event_sent != 0 as HANDLE {
                CloseHandle(self.event_sent);
                self.event_sent = 0 as HANDLE;
            }
            if self.event_stream_active != 0 as HANDLE {
                CloseHandle(self.event_stream_active);
                self.event_stream_active = 0 as HANDLE;
            }
        }
        crate::log_info!("VirtualCam-RS", "Dispositivo '{}' detenido.", self.device_name);
    }

    pub fn publish_frame(&mut self, rgba_data: &[u8]) -> bool {
        if !self.active.load(Ordering::SeqCst) || self.shared_buffer.is_null() {
            return false;
        }

        let width = self.width;
        let height = self.height;
        let stride = width * 4;
        let pixel_bytes = stride * height;

        if rgba_data.len() < pixel_bytes || pixel_bytes > MAX_DATA_SIZE {
            return false;
        }

        unsafe {
            // Esperar mutex brevemente para evitar desgarros de frame (tearing)
            if self.mutex != 0 as HANDLE {
                WaitForSingleObject(self.mutex, 5);
            }

            // 1. Escribir cabecera estándar DirectShow
            let header = self.shared_buffer as *mut SharedMemHeader;
            (*header).max_size = MAX_DATA_SIZE as u32;
            (*header).width = width as i32;
            (*header).height = height as i32;
            (*header).stride = width as i32;
            (*header).format = 0;     // FORMAT_UINT8 (RGBA32)
            (*header).resize_mode = 1;// RESIZEMODE_LINEAR
            (*header).mirror_mode = 0;// MIRRORMODE_DISABLED
            (*header).timeout = 2147483647;

            // 2. Copia con volteo vertical (bottom-up scanline estándar DIB DirectShow)
            let dest_pixels = self.shared_buffer.add(HEADER_SIZE);
            for y in 0..height {
                let src_row = (height - 1 - y) * stride;
                let dst_row = y * stride;
                ptr::copy_nonoverlapping(
                    rgba_data.as_ptr().add(src_row),
                    dest_pixels.add(dst_row),
                    stride,
                );
            }

            // 3. Liberar mutex y señalizar cuadro enviado
            if self.mutex != 0 as HANDLE {
                ReleaseMutex(self.mutex);
            }

            if self.event_sent != 0 as HANDLE {
                SetEvent(self.event_sent);
            }

            if self.event_stream_active != 0 as HANDLE {
                SetEvent(self.event_stream_active);
            }
        }

        self.frame_index = self.frame_index.wrapping_add(1);
        true
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

impl Drop for VirtualCamManager {
    fn drop(&mut self) {
        self.stop();
    }
}
