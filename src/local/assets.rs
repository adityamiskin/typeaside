#[derive(Clone, Copy)]
pub(super) struct Asset {
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}
pub(super) const PARAKEET_V3: [Asset; 4] = [
    Asset {
        name: "decoder_joint-model.int8.onnx",
        url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce/decoder_joint-model.int8.onnx",
        sha256: "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70",
        size: 18202004,
    },
    Asset {
        name: "encoder-model.int8.onnx",
        url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce/encoder-model.int8.onnx",
        sha256: "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09",
        size: 652183999,
    },
    Asset {
        name: "nemo128.onnx",
        url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce/nemo128.onnx",
        sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f",
        size: 139764,
    },
    Asset {
        name: "vocab.txt",
        url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce/vocab.txt",
        sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
        size: 93939,
    },
];

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(super) const RUNTIME: Option<Asset> = Some(Asset {
    name: "onnxruntime-linux-x64-1.24.4.tgz",
    url: "https://github.com/microsoft/onnxruntime/releases/download/v1.24.4/onnxruntime-linux-x64-1.24.4.tgz",
    sha256: "3a211fbea252c1e66290658f1b735b772056149f28321e71c308942cdb54b747",
    size: 8155822,
});

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(super) const RUNTIME: Option<Asset> = Some(Asset {
    name: "onnxruntime-osx-arm64-1.24.4.tgz",
    url: "https://github.com/microsoft/onnxruntime/releases/download/v1.24.4/onnxruntime-osx-arm64-1.24.4.tgz",
    sha256: "93787795f47e1eee369182e43ed51b9e5da0878ab0346aecf4258979b8bba989",
    size: 30937282,
});

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub(super) const RUNTIME: Option<Asset> = Some(Asset {
    name: "onnxruntime-win-x64-1.24.4.zip",
    url: "https://github.com/microsoft/onnxruntime/releases/download/v1.24.4/onnxruntime-win-x64-1.24.4.zip",
    sha256: "d2319fddfb6ea4db99ccc4b60c85c517bcd855721f5daa6a06d40d7cb2ee2357",
    size: 74442783,
});

#[cfg(not(any(
    all(target_os = "linux", target_arch = "x86_64"),
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
)))]
pub(super) const RUNTIME: Option<Asset> = None;

pub(super) const COHERE: [Asset; 5] = [
    Asset {
        name: "encoder_model_q4.onnx",
        url: "https://huggingface.co/onnx-community/cohere-transcribe-03-2026-ONNX/resolve/31b1c6211c9000d76b077ddd23b74c9090badeba/onnx/encoder_model_q4.onnx",
        sha256: "de0f7e2c5f4c2e46e3704704a3cb41153ed45f5af07530b4b1d34f895c36db4b",
        size: 1400792,
    },
    Asset {
        name: "encoder_model_q4.onnx_data",
        url: "https://huggingface.co/onnx-community/cohere-transcribe-03-2026-ONNX/resolve/31b1c6211c9000d76b077ddd23b74c9090badeba/onnx/encoder_model_q4.onnx_data",
        sha256: "c5c668cc8c5951c789893ad25b06c654fc7cced7fb0989ad7ef0fb44a0554ee6",
        size: 2016067072,
    },
    Asset {
        name: "decoder_model_merged_q4.onnx",
        url: "https://huggingface.co/onnx-community/cohere-transcribe-03-2026-ONNX/resolve/31b1c6211c9000d76b077ddd23b74c9090badeba/onnx/decoder_model_merged_q4.onnx",
        sha256: "a565cdfa7ad3e12e1149e8cdcf519ade36f9e76c43038471c876c25893f8f8bf",
        size: 193099,
    },
    Asset {
        name: "decoder_model_merged_q4.onnx_data",
        url: "https://huggingface.co/onnx-community/cohere-transcribe-03-2026-ONNX/resolve/31b1c6211c9000d76b077ddd23b74c9090badeba/onnx/decoder_model_merged_q4.onnx_data",
        sha256: "9223643714186378c6aa0c95439d74298ed25d40b43b6737c55c016055ceb1ee",
        size: 108855296,
    },
    Asset {
        name: "tokenizer.json",
        url: "https://huggingface.co/onnx-community/cohere-transcribe-03-2026-ONNX/resolve/31b1c6211c9000d76b077ddd23b74c9090badeba/tokenizer.json",
        sha256: "e263c0ba13be0f0803705b002756908f84efc6e75a4c273231a01c4371908a2b",
        size: 1152395,
    },
];
pub(super) const PARAKEET_UNIFIED_EN: [Asset; 1] = [Asset {
    name: "parakeet-unified-en-0.6b-Q8_0.gguf",
    url: "https://huggingface.co/handy-computer/parakeet-unified-en-0.6b-gguf/resolve/7e948f21b7bdbac698d3318db9d350f1096f3b6c/parakeet-unified-en-0.6b-Q8_0.gguf",
    sha256: "4b50b6dd862bf6e346929aaf4f5eaacec003bfa3f56462d6c874b41ef2f38795",
    size: 731357568,
}];
pub(super) const WHISPER_TURBO: [Asset; 1] = [Asset {
    name: "whisper-large-v3-turbo-Q5_K_M.gguf",
    url: "https://huggingface.co/handy-computer/whisper-large-v3-turbo-gguf/resolve/18ada752005f5fddef499256f9a458d0b37a203a/whisper-large-v3-turbo-Q5_K_M.gguf",
    sha256: "06fc9923c09fb3ae878ea12c34d1be3ed18fad039ab14b4b045c7b6c40af56b2",
    size: 619628096,
}];

pub(super) const SILERO_VAD: Asset = Asset {
    name: "silero-vad-6.2.3.onnx",
    url: "https://raw.githubusercontent.com/snakers4/silero-vad/5cd7945676eb32225748052e2e6a0580e4686a08/src/silero_vad/data/silero_vad.onnx",
    sha256: "1a153a22f4509e292a94e67d6f9b85e8deb25b4988682b7e174c65279d8788e3",
    size: 2327524,
};
