#[derive(Clone, Copy)]
pub(super) struct Asset {
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size: u64,
}
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

pub(super) const PARAKEET_V3: [Asset; 1] = [Asset {
    name: "parakeet-tdt-0.6b-v3-Q4_K_M.gguf",
    url: "https://huggingface.co/handy-computer/parakeet-tdt-0.6b-v3-gguf/resolve/d8d8c32cb2017c57e5307767343e97870f8a7f53/parakeet-tdt-0.6b-v3-Q4_K_M.gguf",
    sha256: "b68557be1e3c40207fd7c4bd9d63f1d3316b963f15325bfb0cc16a8bb0ffd181",
    size: 485425504,
}];
pub(super) const COHERE: [Asset; 1] = [Asset {
    name: "cohere-transcribe-03-2026-Q4_K_M.gguf",
    url: "https://huggingface.co/handy-computer/cohere-transcribe-03-2026-gguf/resolve/7353b6040bdc1be9e5a3823d99d5fe12afe88264/cohere-transcribe-03-2026-Q4_K_M.gguf",
    sha256: "0ea56826d8bd5d74b7143a4a04e022dc1bb75452cfae49d98b6acb0c1d16a1fb",
    size: 1558162944,
}];
pub(super) const SILERO_VAD: Asset = Asset {
    name: "silero-vad-v6.2-F32.gguf",
    url: "https://huggingface.co/handy-computer/silero-vad-v6.2-gguf/resolve/a9b2d61c20ae4ddf3644edb34969e8185a485448/silero-vad-v6.2-F32.gguf",
    sha256: "0d871a800c420fdd5d1b17a3aea4296d1c39608f0ff4a9295eb76457eb1c88e6",
    size: 1241056,
};
