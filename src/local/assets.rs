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

pub(super) const MOONSHINE_BASE: [Asset; 3] = [
    Asset {
        name: "encoder_model_quantized.onnx",
        url: "https://huggingface.co/onnx-community/moonshine-base-ONNX/resolve/b1e9b6aae3c3c7298f10c3798393fdf38e8fbbad/onnx/encoder_model_quantized.onnx",
        sha256: "1dd9ab0a7f987113d30affcba5a068d11c8f90fa0223caa3e491ade431ad9751",
        size: 20513063,
    },
    Asset {
        name: "decoder_model_merged_quantized.onnx",
        url: "https://huggingface.co/onnx-community/moonshine-base-ONNX/resolve/b1e9b6aae3c3c7298f10c3798393fdf38e8fbbad/onnx/decoder_model_merged_quantized.onnx",
        sha256: "cc9f3cd6698a369c6008b41aa60aa3fb3322e7f03c9bdf19d8e6b7200afca4f3",
        size: 42498870,
    },
    Asset {
        name: "tokenizer.json",
        url: "https://huggingface.co/onnx-community/moonshine-base-ONNX/resolve/b1e9b6aae3c3c7298f10c3798393fdf38e8fbbad/tokenizer.json",
        sha256: "7b913404bdd039af4756783218af4440bc07fb7d6d8258d677e34f95b3ec416f",
        size: 3761754,
    },
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
pub(super) const WHISPER_TURBO: [Asset; 4] = [
    Asset {
        name: "encoder_model_quantized.onnx",
        url: "https://huggingface.co/onnx-community/whisper-large-v3-turbo-ONNX/resolve/f2d0148f21bb07751a7e6e9a4377e1ae7146607f/onnx/encoder_model_quantized.onnx",
        sha256: "d2f853dc3254fdc0079f55dd4433ea716ac98ec5574d3b475f288f2a77cebba9",
        size: 644822195,
    },
    Asset {
        name: "decoder_model_merged_quantized.onnx",
        url: "https://huggingface.co/onnx-community/whisper-large-v3-turbo-ONNX/resolve/f2d0148f21bb07751a7e6e9a4377e1ae7146607f/onnx/decoder_model_merged_quantized.onnx",
        sha256: "fc6cd2499a13239dc650d0e633e6f49f775c254614a1ddaaec551581493445a5",
        size: 438133085,
    },
    Asset {
        name: "tokenizer.json",
        url: "https://huggingface.co/onnx-community/whisper-large-v3-turbo-ONNX/resolve/f2d0148f21bb07751a7e6e9a4377e1ae7146607f/tokenizer.json",
        sha256: "b3c8202bbf06d8ee4232c5984baa563784ac4737e2e7fdc42fa180200d3cfcdb",
        size: 2480645,
    },
    Asset {
        name: "generation_config.json",
        url: "https://huggingface.co/onnx-community/whisper-large-v3-turbo-ONNX/resolve/f2d0148f21bb07751a7e6e9a4377e1ae7146607f/generation_config.json",
        sha256: "8568ddf83476167b4607014956b87142763fa6923e71259a606734beb39bdfc8",
        size: 3797,
    },
];
