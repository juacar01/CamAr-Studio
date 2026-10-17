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
            "Panic desconocido"
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
    }));

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
