use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareOption {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "isRecommended")]
    pub is_recommended: bool,
    #[serde(rename = "deviceType")]
    pub device_type: String, // "CPU", "GPU", "NPU"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHardwareInfo {
    #[serde(rename = "cpuName")]
    pub cpu_name: String,
    #[serde(rename = "cpuCores")]
    pub cpu_cores: usize,
    #[serde(rename = "gpuName")]
    pub gpu_name: String,
    #[serde(rename = "hasDedicatedGpu")]
    pub has_dedicated_gpu: bool,
    #[serde(rename = "npuName")]
    pub npu_name: String,
    #[serde(rename = "hasNpu")]
    pub has_npu: bool,
    #[serde(rename = "videoBackends")]
    pub video_backends: Vec<HardwareOption>,
    #[serde(rename = "aiBackends")]
    pub ai_backends: Vec<HardwareOption>,
}

fn query_reg_value(key: &str, value_name: &str) -> Option<String> {
    let output = std::process::Command::new("reg")
        .args(["query", key, "/v", value_name])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if line.contains(value_name) {
            let val_idx = line.find("REG_SZ")?;
            let val = line[val_idx + 6..].trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn detect_npu_device() -> Option<String> {
    let output = std::process::Command::new("reg")
        .args(["query", "HKLM\\SYSTEM\\CurrentControlSet\\Enum\\PCI", "/s", "/f", "AI Boost"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    if text.contains("AI Boost") || text.contains("Intel(R) AI Boost") {
        return Some("Intel(R) AI Boost (NPU Compute Accelerator)".to_string());
    }
    None
}

impl SystemHardwareInfo {
    pub fn detect() -> Self {
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8);

        let cpu_name = query_reg_value("HKLM\\HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0", "ProcessorNameString")
            .unwrap_or_else(|| "Procesador x86_64".to_string());

        let gpu_name = query_reg_value(
            "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\\0000",
            "DriverDesc",
        ).or_else(|| {
            query_reg_value(
                "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Class\\{4d36e968-e325-11ce-bfc1-08002be10318}\\0001",
                "DriverDesc",
            )
        }).unwrap_or_else(|| "GPU Compatible con Direct3D 12".to_string());

        let npu_opt = detect_npu_device();
        let has_npu = npu_opt.is_some();
        let npu_name = npu_opt.unwrap_or_else(|| "No detectada".to_string());

        let has_dedicated_gpu = gpu_name.contains("NVIDIA") 
            || gpu_name.contains("GeForce") 
            || gpu_name.contains("Radeon") 
            || gpu_name.contains("RTX") 
            || gpu_name.contains("Arc");

        let video_backends = vec![
            HardwareOption {
                id: "cpu".to_string(),
                name: format!("CPU (SIMD AVX2 - {} Cores)", cpu_cores),
                description: "Procesamiento nativo en CPU usando hilos paralelos.".to_string(),
                is_recommended: !has_dedicated_gpu,
                device_type: "CPU".to_string(),
            },
            HardwareOption {
                id: "gpu_d3d12".to_string(),
                name: format!("GPU - {} (Direct3D 12 Compute)", gpu_name),
                description: "Aceleración gráfica por hardware. Máximos FPS, mínima latencia y cero carga en CPU. (Recomendado)".to_string(),
                is_recommended: has_dedicated_gpu,
                device_type: "GPU".to_string(),
            },
        ];

        let mut ai_backends = vec![
            HardwareOption {
                id: "cpu".to_string(),
                name: "CPU (Multihilo AVX2)".to_string(),
                description: "Inferencia ejecutada en CPU.".to_string(),
                is_recommended: !has_npu && !has_dedicated_gpu,
                device_type: "CPU".to_string(),
            },
        ];

        if has_npu {
            ai_backends.push(HardwareOption {
                id: "npu".to_string(),
                name: format!("NPU - {} (Acelerador Neural)", npu_name),
                description: "Inferencia dedicada en NPU. Cero impacto en GPU y CPU, ultra eficiente. (Recomendado)".to_string(),
                is_recommended: true,
                device_type: "NPU".to_string(),
            });
        }

        if has_dedicated_gpu {
            ai_backends.push(HardwareOption {
                id: "gpu".to_string(),
                name: format!("GPU - {} (Tensor Cores / DirectML)", gpu_name),
                description: "Máxima velocidad de cálculo con núcleos Tensor de la GPU dedicada.".to_string(),
                is_recommended: !has_npu,
                device_type: "GPU".to_string(),
            });
        }

        Self {
            cpu_name,
            cpu_cores,
            gpu_name,
            has_dedicated_gpu,
            npu_name,
            has_npu,
            video_backends,
            ai_backends,
        }
    }

    pub fn get_optimal_video_backend(&self) -> String {
        if self.has_dedicated_gpu {
            "gpu_d3d12".to_string()
        } else {
            "cpu".to_string()
        }
    }

    pub fn get_optimal_ai_backend(&self) -> String {
        if self.has_npu {
            "npu".to_string()
        } else if self.has_dedicated_gpu {
            "gpu".to_string()
        } else {
            "cpu".to_string()
        }
    }
}
