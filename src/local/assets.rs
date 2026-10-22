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
pub(super) const BARK_3_CLEAN: [Asset; 8] = [
    Asset {
        name: "encoder.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0-clean@6d34ed86/encoder.int8.onnx",
        sha256: "4f83513610515af37a98fd3b42525055f1c5581f001f669a00377fa120e7e8db",
        size: 901742,
    },
    Asset {
        name: "encoder.int8.data0",
        url: "https://typeaside.app/models/underdog-bark-3.0-clean@6d34ed86/encoder.int8.data0",
        sha256: "2c23bddc0e366273b3ddcb0eb52168a3875d04ded5a99c50e60bf03077411cca",
        size: 210210816,
    },
    Asset {
        name: "encoder.int8.data1",
        url: "https://typeaside.app/models/underdog-bark-3.0-clean@6d34ed86/encoder.int8.data1",
        sha256: "18aabb6b6d2b1f5099b6aef4f3f617417cef7595ac159aa0515811cd89849f2f",
        size: 211943424,
    },
    Asset {
        name: "encoder.int8.data2",
        url: "https://typeaside.app/models/underdog-bark-3.0-clean@6d34ed86/encoder.int8.data2",
        sha256: "1b9fcc237a7e0055d874f192293a97486a5b50fd5dfe6b22bbadb20206bb1322",
        size: 207749120,
    },
    Asset {
        name: "decoder.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0-clean@6d34ed86/decoder.int8.onnx",
        sha256: "8fb0c48ed66cbc43855912e2e1b75c50037e9d8d628eef11bcb5712b7eab7681",
        size: 47658545,
    },
    Asset {
        name: "joiner.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0-clean@6d34ed86/joiner.int8.onnx",
        sha256: "2d9a8a895f38a2e132c5c4edf9de4af638c121664329f7a86c473190700c6378",
        size: 6123465,
    },
    Asset {
        name: "tokens.txt",
        url: "https://typeaside.app/models/underdog-bark-3.0-clean@6d34ed86/tokens.txt",
        sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
        size: 93939,
    },
    PARAKEET_V3[2],
];
pub(super) const BARK_3: [Asset; 8] = [
    Asset {
        name: "encoder.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0@85db2ad1/encoder.int8.onnx",
        sha256: "96ba0e747bafb8f37974e0c576bb0fa5cb866664e81e83cabf2d65fb4b28ebc8",
        size: 901714,
    },
    Asset {
        name: "encoder.int8.data0",
        url: "https://typeaside.app/models/underdog-bark-3.0@85db2ad1/encoder.int8.data0",
        sha256: "0fa5d429e45e56efc8c4a4cfc1476944e46ef629fc9a0af59bb8333bac2b9e51",
        size: 210210816,
    },
    Asset {
        name: "encoder.int8.data1",
        url: "https://typeaside.app/models/underdog-bark-3.0@85db2ad1/encoder.int8.data1",
        sha256: "351794739e788d33fe709e3236daaddaa4f60bdae38043a24701b8b79732b02c",
        size: 211943424,
    },
    Asset {
        name: "encoder.int8.data2",
        url: "https://typeaside.app/models/underdog-bark-3.0@85db2ad1/encoder.int8.data2",
        sha256: "13d43f9169304442ece73840d587cf48f9ab9b1f6453b16e7793da63e54039e1",
        size: 207749120,
    },
    Asset {
        name: "decoder.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0@85db2ad1/decoder.int8.onnx",
        sha256: "89a081f0cf1500ec64e8a45f24f0bc10d1224aa334191e7f7e6f3226365b06b8",
        size: 47658545,
    },
    Asset {
        name: "joiner.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0@85db2ad1/joiner.int8.onnx",
        sha256: "b8fe34824c5d2ec077489e4b4f66907cb47a971e1d34fe96915e9e5427df3c89",
        size: 6123465,
    },
    Asset {
        name: "tokens.txt",
        url: "https://typeaside.app/models/underdog-bark-3.0@85db2ad1/tokens.txt",
        sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
        size: 93939,
    },
    PARAKEET_V3[2],
];
pub(super) const BARK_3_PLUS: [Asset; 8] = [
    Asset {
        name: "encoder.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0-plus@d4fab2f8/encoder.int8.onnx",
        sha256: "e3c250d7f58360ada8718537a4618773994abdd266a1986467f9bfe036f8f577",
        size: 901739,
    },
    Asset {
        name: "encoder.int8.data0",
        url: "https://typeaside.app/models/underdog-bark-3.0-plus@d4fab2f8/encoder.int8.data0",
        sha256: "c0cea250290045e0a3e8983afe953ae2449d42c68efd9f5e1d4b20a9be075326",
        size: 210210816,
    },
    Asset {
        name: "encoder.int8.data1",
        url: "https://typeaside.app/models/underdog-bark-3.0-plus@d4fab2f8/encoder.int8.data1",
        sha256: "aa7ba17e6a1dda11e667b7308633137354d1220759902a27abf3a2b6a493cb95",
        size: 211943424,
    },
    Asset {
        name: "encoder.int8.data2",
        url: "https://typeaside.app/models/underdog-bark-3.0-plus@d4fab2f8/encoder.int8.data2",
        sha256: "c144089c14a1dda28dec0959043b0ebe214b921b86d568f86fa13d14b6512ff2",
        size: 207749120,
    },
    Asset {
        name: "decoder.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0-plus@d4fab2f8/decoder.int8.onnx",
        sha256: "89a081f0cf1500ec64e8a45f24f0bc10d1224aa334191e7f7e6f3226365b06b8",
        size: 47658545,
    },
    Asset {
        name: "joiner.int8.onnx",
        url: "https://typeaside.app/models/underdog-bark-3.0-plus@d4fab2f8/joiner.int8.onnx",
        sha256: "b8fe34824c5d2ec077489e4b4f66907cb47a971e1d34fe96915e9e5427df3c89",
        size: 6123465,
    },
    Asset {
        name: "tokens.txt",
        url: "https://typeaside.app/models/underdog-bark-3.0-plus@d4fab2f8/tokens.txt",
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

pub(super) const SILERO_VAD: Asset = Asset {
    name: "silero-vad-6.2.3.onnx",
    url: "https://raw.githubusercontent.com/snakers4/silero-vad/5cd7945676eb32225748052e2e6a0580e4686a08/src/silero_vad/data/silero_vad.onnx",
    sha256: "1a153a22f4509e292a94e67d6f9b85e8deb25b4988682b7e174c65279d8788e3",
    size: 2327524,
};
