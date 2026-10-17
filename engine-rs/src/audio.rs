use crate::types::AudioDeviceInfo;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct AudioEngine {
    stream: Option<cpal::Stream>,
    peak_db: Arc<Mutex<f32>>,
    rms_db: Arc<Mutex<f32>>,
    volume: Arc<Mutex<f32>>,
    muted: Arc<AtomicBool>,
    _current_device_id: String,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            stream: None,
            peak_db: Arc::new(Mutex::new(-96.0)),
            rms_db: Arc::new(Mutex::new(-96.0)),
            volume: Arc::new(Mutex::new(1.0)),
            muted: Arc::new(AtomicBool::new(false)),
            _current_device_id: "default".to_string(),
        }
    }

    /// Enumera dinámicamente todos los micrófonos reales en Windows sin nada quemado
    pub fn enumerate_devices() -> Vec<AudioDeviceInfo> {
        let host = cpal::default_host();
        let mut list = Vec::new();

        let default_name = host
            .default_input_device()
            .and_then(|d| d.description().map(|desc| desc.name().to_string()).ok())
            .unwrap_or_default();

        if let Ok(devices) = host.input_devices() {
            for dev in devices {
                let name = dev
                    .description()
                    .map(|desc| desc.name().to_string())
                    .unwrap_or_else(|_| dev.to_string());

                let is_default = name == default_name;
                list.push(AudioDeviceInfo {
                    id: name.clone(),
                    name,
                    is_default,
                });
            }
        }

        if list.is_empty() {
            list.push(AudioDeviceInfo {
                id: "default".to_string(),
                name: "Micrófono del Sistema (WASAPI)".to_string(),
                is_default: true,
            });
        }

        list
    }

    pub fn start(&mut self, device_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.stop();

        let host = cpal::default_host();
        let device = if device_name == "default" || device_name.is_empty() {
            host.default_input_device()
        } else {
            host.input_devices()?
                .find(|d| {
                    d.description()
                        .map(|desc| desc.name() == device_name)
                        .unwrap_or_else(|_| d.to_string() == device_name)
                })
                .or_else(|| host.default_input_device())
        };

        let device = match device {
            Some(d) => d,
            None => {
                crate::log_warn!("Audio", "No se encontró ningún dispositivo de entrada de audio.");
                return Ok(());
            }
        };

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                crate::log_error!("Audio", "Error al obtener configuración de audio: {}", e);
                return Ok(());
            }
        };

        let peak_clone = Arc::clone(&self.peak_db);
        let rms_clone = Arc::clone(&self.rms_db);
        let vol_clone = Arc::clone(&self.volume);
        let muted_clone = Arc::clone(&self.muted);

        let err_fn = |err| {
            let err_str = format!("{}", err);
            // Evitar saturar la consola con avisos transitorios de jitter/underrun al conectar WASAPI
            if err_str.contains("underrun") || err_str.contains("overrun") {
                return;
            }
            crate::log_warn!("Audio", "Aviso en stream de audio: {}", err_str);
        };

        let stream_config: cpal::StreamConfig = config.clone().into();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                stream_config,
                move |data: &[f32], _: &_| {
                    Self::process_samples(data, &peak_clone, &rms_clone, &vol_clone, &muted_clone);
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                stream_config,
                move |data: &[i16], _: &_| {
                    let float_samples: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                    Self::process_samples(&float_samples, &peak_clone, &rms_clone, &vol_clone, &muted_clone);
                },
                err_fn,
                None,
            )?,
            _ => {
                crate::log_error!("Audio", "Formato de muestra no soportado directamente.");
                return Ok(());
            }
        };

        stream.play()?;
        self.stream = Some(stream);
        self._current_device_id = device_name.to_string();
        crate::log_info!("Audio", "Captura de audio iniciada en '{}'", device_name);
        Ok(())
    }

    fn process_samples(
        data: &[f32],
        peak_arc: &Arc<Mutex<f32>>,
        rms_arc: &Arc<Mutex<f32>>,
        vol_arc: &Arc<Mutex<f32>>,
        muted_arc: &Arc<AtomicBool>,
    ) {
        if muted_arc.load(Ordering::Relaxed) || data.is_empty() {
            if let Ok(mut p) = peak_arc.lock() { *p = -96.0; }
            if let Ok(mut r) = rms_arc.lock() { *r = -96.0; }
            return;
        }

        let vol = if let Ok(v) = vol_arc.lock() { *v } else { 1.0 };

        let mut max_abs = 0.0_f32;
        let mut sum_sq = 0.0_f32;

        for &sample in data {
            let s = (sample * vol).abs();
            if s > max_abs {
                max_abs = s;
            }
            sum_sq += s * s;
        }

        let rms = (sum_sq / data.len() as f32).sqrt();

        // Convertir a dBFS (mínimo -96 dB)
        let peak_db = if max_abs > 0.00001 { (20.0 * max_abs.log10()).max(-96.0) } else { -96.0 };
        let rms_db = if rms > 0.00001 { (20.0 * rms.log10()).max(-96.0) } else { -96.0 };

        if let Ok(mut p) = peak_arc.lock() {
            *p = *p * 0.7 + peak_db * 0.3;
        }
        if let Ok(mut r) = rms_arc.lock() {
            *r = *r * 0.7 + rms_db * 0.3;
        }
    }

    pub fn set_volume(&self, vol: f32) {
        if let Ok(mut v) = self.volume.lock() {
            *v = vol.clamp(0.0, 2.0);
        }
    }

    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    pub fn get_levels(&self) -> (f32, f32) {
        let p = if let Ok(v) = self.peak_db.lock() { *v } else { -96.0 };
        let r = if let Ok(v) = self.rms_db.lock() { *v } else { -96.0 };
        (p, r)
    }

    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop();
    }
}
