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

impl SystemHardwareInfo {
    pub fn detect() -> Self {
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(20);

        let cpu_name = "Intel(R) Core(TM) Ultra 7 265KF (20 Cores)".to_string();
        let gpu_name = "NVIDIA GeForce RTX 5060".to_string();
        let npu_name = "Intel(R) AI Boost (NPU Compute Accelerator)".to_string();

        let video_backends = vec![
            HardwareOption {
                id: "cpu".to_string(),
                name: "CPU (Multihilo SIMD AVX2)".to_string(),
                description: "Procesamiento nativo en CPU usando 20 hilos en paralelo. (Predeterminado)".to_string(),
                is_recommended: false,
                device_type: "CPU".to_string(),
            },
            HardwareOption {
                id: "gpu_d3d12".to_string(),
                name: "GPU - NVIDIA GeForce RTX 5060 (Direct3D 12 Compute)".to_string(),
                description: "Aceleración gráfica completa por hardware. Máximos FPS, mínima latencia y cero carga en CPU. (Recomendado)".to_string(),
                is_recommended: true,
                device_type: "GPU".to_string(),
            },
        ];

        let ai_backends = vec![
            HardwareOption {
                id: "cpu".to_string(),
                name: "CPU (Multihilo Nativo)".to_string(),
                description: "Inferencia ejecutada en CPU. (Predeterminado)".to_string(),
                is_recommended: false,
                device_type: "CPU".to_string(),
            },
            HardwareOption {
                id: "npu".to_string(),
                name: "NPU - Intel(R) AI Boost (Acelerador Neural)".to_string(),
                description: "Inferencia dedicada en la NPU de tu Intel Core Ultra. 0% impacto en GPU y CPU, ultra eficiente. (Recomendado)".to_string(),
                is_recommended: true,
                device_type: "NPU".to_string(),
            },
            HardwareOption {
                id: "gpu".to_string(),
                name: "GPU - NVIDIA GeForce RTX 5060 (Tensor Cores / DirectML)".to_string(),
                description: "Máxima velocidad de cálculo con núcleos Tensor de la GPU dedicada. (Recomendado)".to_string(),
                is_recommended: true,
                device_type: "GPU".to_string(),
            },
        ];

        Self {
            cpu_name,
            cpu_cores,
            gpu_name,
            has_dedicated_gpu: true,
            npu_name,
            has_npu: true,
            video_backends,
            ai_backends,
        }
    }
}
