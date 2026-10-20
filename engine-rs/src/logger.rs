use chrono::Local;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO ",
            LogLevel::Warn => "WARN ",
            LogLevel::Error => "ERROR",
        }
    }
}

static LOGGER: Mutex<Option<LoggerInstance>> = Mutex::new(None);

struct LoggerInstance {
    main_log_path: PathBuf,
    error_log_path: PathBuf,
}

pub fn init_logger() {
    let base_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // Guardar logs tanto en la raíz del proyecto como junto al ejecutable
    let logs_dir = if base_dir.join("../../../logs").exists() || base_dir.ends_with("release") || base_dir.ends_with("debug") {
        PathBuf::from("logs")
    } else {
        base_dir.join("logs")
    };

    let _ = create_dir_all(&logs_dir);

    let main_log_path = logs_dir.join("camar_studio.log");
    let error_log_path = logs_dir.join("error.log");

    // Instalar Panic Hook para trazabilidad total de fallos no controlados
    let err_path_clone = error_log_path.clone();
    let main_path_clone = main_log_path.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "Error de ejecución no controlado (Panic)"
        };
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "ubicación desconocida".to_string());

        let err_msg = format!(
            "[{}] [CRITICAL PANIC] Ubicación: {} | Detalle: {}\n",
            ts, location, payload
        );

        eprintln!("{}", err_msg);

        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&err_path_clone) {
            let _ = f.write_all(err_msg.as_bytes());
        }
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&main_path_clone) {
            let _ = f.write_all(err_msg.as_bytes());
        }

        let desc = format!(
            "Se produjo un fallo inesperado en el motor:\n{}\n\nUbicación técnica: {}\n\nConsulte el archivo 'logs/error.log' para más información.",
            payload, location
        );
        show_error_dialog("Ejecución de CamAr Studio", &desc);
    }));

    install_crash_handlers();

    let mut lock = LOGGER.lock().unwrap();
    *lock = Some(LoggerInstance {
        main_log_path,
        error_log_path,
    });
}

pub fn log(level: LogLevel, target: &str, message: &str) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] [{}] [{}] {}\n", now, level.as_str(), target, message);

    // Salida a consola estándar
    match level {
        LogLevel::Error => eprintln!("{}", line.trim_end()),
        LogLevel::Warn => eprintln!("{}", line.trim_end()),
        _ => println!("{}", line.trim_end()),
    }

    // Persistir en disco
    if let Ok(guard) = LOGGER.lock() {
        if let Some(inst) = guard.as_ref() {
            // Guardar en el log general
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&inst.main_log_path) {
                let _ = f.write_all(line.as_bytes());
            }

            // Si es un error, guardarlo además en error.log dedicado para trazabilidad inmediata
            if level == LogLevel::Error {
                if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&inst.error_log_path) {
                    let _ = f.write_all(line.as_bytes());
                }
            }
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ($target:expr, $($arg:tt)+) => {
        $crate::logger::log($crate::logger::LogLevel::Info, $target, &format!($($arg)+))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($target:expr, $($arg:tt)+) => {
        $crate::logger::log($crate::logger::LogLevel::Warn, $target, &format!($($arg)+))
    };
}

#[macro_export]
macro_rules! log_error {
    ($target:expr, $($arg:tt)+) => {
        $crate::logger::log($crate::logger::LogLevel::Error, $target, &format!($($arg)+))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($target:expr, $($arg:tt)+) => {
        $crate::logger::log($crate::logger::LogLevel::Debug, $target, &format!($($arg)+))
    };
}

#[link(name = "user32")]
extern "system" {
    fn MessageBoxW(
        hWnd: *mut std::ffi::c_void,
        lpText: *const u16,
        lpCaption: *const u16,
        uType: u32,
    ) -> i32;
}

/// Muestra una ventana de diálogo nativa de Windows con un mensaje claro y legible para el usuario.
pub fn show_error_dialog(modulo: &str, descripcion: &str) {
    let title: Vec<u16> = "CamAr Studio - Error\0".encode_utf16().collect();
    let text = format!("Error en {}:\n\n{}\0", modulo, descripcion);
    let msg: Vec<u16> = text.encode_utf16().collect();

    unsafe {
        // MB_OK (0x0) | MB_ICONERROR (0x10) | MB_TOPMOST (0x40000) | MB_SETFOREGROUND (0x10000)
        MessageBoxW(
            std::ptr::null_mut(),
            msg.as_ptr(),
            title.as_ptr(),
            0x00000010 | 0x00040000 | 0x00010000,
        );
    }
}

#[repr(C)]
struct ExceptionRecord {
    exception_code: u32,
    exception_flags: u32,
    exception_record: *mut ExceptionRecord,
    exception_address: *mut std::ffi::c_void,
    number_parameters: u32,
    exception_information: [usize; 15],
}

#[repr(C)]
struct ExceptionPointers {
    exception_record: *mut ExceptionRecord,
    context_record: *mut std::ffi::c_void,
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn unhandled_exception_handler(
    info: *mut std::ffi::c_void,
) -> i32 {
    let mut code_str = "0x00000000".to_string();
    let mut addr_str = "0x0".to_string();
    let mut reason = "Fallo nativo no identificado".to_string();

    if !info.is_null() {
        let ep = info as *const ExceptionPointers;
        if !(*ep).exception_record.is_null() {
            let rec = (*ep).exception_record;
            let code = (*rec).exception_code;
            let addr = (*rec).exception_address;
            code_str = format!("0x{:08X}", code);
            addr_str = format!("{:p}", addr);

            reason = match code {
                0xC0000005 => "Violación de Acceso a Memoria (STATUS_ACCESS_VIOLATION)".to_string(),
                0xC000001D => "Instrucción de CPU ilegal o no soportada (STATUS_ILLEGAL_INSTRUCTION)".to_string(),
                0xC00000FD => "Desbordamiento de Pila (STATUS_STACK_OVERFLOW)".to_string(),
                0xC0000008 => "Handle de sistema inválido (STATUS_INVALID_HANDLE)".to_string(),
                0xC0000094 => "División por cero en CPU (STATUS_INTEGER_DIVIDE_BY_ZERO)".to_string(),
                0xE06D7363 => "Excepción C++ no controlada en librería nativa (MSVC C++ Exception)".to_string(),
                _ => format!("Excepción del sistema (Código {})", code_str),
            };
        }
    }

    let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let err_msg = format!(
        "[{}] [CRITICAL NATIVE EXCEPTION] Código: {} | Causa: {} | Dirección: {}\n",
        ts, code_str, reason, addr_str
    );

    eprintln!("{}", err_msg);

    // Escribir directamente a logs/error.log y logs/camar_studio.log
    if let Ok(guard) = LOGGER.lock() {
        if let Some(inst) = guard.as_ref() {
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&inst.error_log_path) {
                let _ = f.write_all(err_msg.as_bytes());
            }
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&inst.main_log_path) {
                let _ = f.write_all(err_msg.as_bytes());
            }
        }
    }

    let desc = format!(
        "Se ha producido un error crítico a nivel de sistema:\n\n• Causa: {}\n• Código: {}\n• Dirección: {}\n\nEl reporte técnico completo fue registrado en 'logs/error.log'.",
        reason, code_str, addr_str
    );
    show_error_dialog("Excepción del Sistema (Nativo)", &desc);
    1 // EXCEPTION_EXECUTE_HANDLER
}

pub fn install_crash_handlers() {
    #[cfg(target_os = "windows")]
    unsafe {
        #[link(name = "kernel32")]
        extern "system" {
            fn SetUnhandledExceptionFilter(
                lpTopLevelExceptionFilter: unsafe extern "system" fn(*mut std::ffi::c_void) -> i32,
            ) -> *mut std::ffi::c_void;
        }
        SetUnhandledExceptionFilter(unhandled_exception_handler);
    }
}

