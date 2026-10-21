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

// Bark 3.0 is fine-tuned from Parakeet v3 and keeps its preprocessor, so it reuses v3's.
pub(super) const BARK_3: [Asset; 5] = [
    Asset {
        name: "encoder.int8.onnx",
        url: "https://huggingface.co/velddev/Underdog-Bark-3.0-sherpa-onnx/resolve/f438275bc431c58010eadb982dd0f8693ff4364f/encoder.int8.onnx",
        sha256: "d7e8f38d29aa486777eb7d505f2c7f35a0d5914760d622ed328e009cf18b9429",
        size: 630748293,
    },
    Asset {
        name: "decoder.int8.onnx",
        url: "https://huggingface.co/velddev/Underdog-Bark-3.0-sherpa-onnx/resolve/f438275bc431c58010eadb982dd0f8693ff4364f/decoder.int8.onnx",
        sha256: "89a081f0cf1500ec64e8a45f24f0bc10d1224aa334191e7f7e6f3226365b06b8",
        size: 47658545,
    },
    Asset {
        name: "joiner.int8.onnx",
        url: "https://huggingface.co/velddev/Underdog-Bark-3.0-sherpa-onnx/resolve/f438275bc431c58010eadb982dd0f8693ff4364f/joiner.int8.onnx",
        sha256: "b8fe34824c5d2ec077489e4b4f66907cb47a971e1d34fe96915e9e5427df3c89",
        size: 6123465,
    },
    Asset {
        name: "tokens.txt",
        url: "https://huggingface.co/velddev/Underdog-Bark-3.0-sherpa-onnx/resolve/f438275bc431c58010eadb982dd0f8693ff4364f/tokens.txt",
        sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
        size: 93939,
    },
    PARAKEET_V3[2],
];
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
pub(super) const WHISPER_TURBO: [Asset; 1] = [Asset {
    name: "ggml-large-v3-turbo-q5_0.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3-turbo-q5_0.bin",
    sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    size: 574041195,
}];
