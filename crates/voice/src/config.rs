use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub stt: SttConfig,
    pub tts: TtsConfig,
    pub audio: AudioConfig,
    pub vad: VadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    pub provider: SttProvider,
    pub whisper: WhisperConfig,
    pub timeout_seconds: u64,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SttProvider {
    Whisper,
    OpenAI,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperConfig {
    pub model: String,
    pub api_key: Option<String>,
    pub api_url: String,
    pub local_model_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub provider: TtsProvider,
    pub elevenlabs: ElevenLabsConfig,
    pub openai: OpenAITtsConfig,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TtsProvider {
    ElevenLabs,
    OpenAI,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevenLabsConfig {
    pub api_key: String,
    pub api_url: String,
    pub default_voice_id: String,
    pub model_id: String,
    pub stability: f32,
    pub similarity_boost: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITtsConfig {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub voice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub buffer_size: usize,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    pub enabled: bool,
    pub energy_threshold: f32,
    pub silence_duration_ms: u64,
    pub min_speech_duration_ms: u64,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default to avoid requiring API keys
            stt: SttConfig::default(),
            tts: TtsConfig::default(),
            audio: AudioConfig::default(),
            vad: VadConfig::default(),
        }
    }
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            provider: SttProvider::Whisper,
            whisper: WhisperConfig::default(),
            timeout_seconds: 30,
            language: Some("en".to_string()),
        }
    }
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            model: "whisper-1".to_string(),
            api_key: None,
            api_url: "https://api.openai.com/v1/audio/transcriptions".to_string(),
            local_model_path: None,
        }
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            provider: TtsProvider::ElevenLabs,
            elevenlabs: ElevenLabsConfig::default(),
            openai: OpenAITtsConfig::default(),
            timeout_seconds: 30,
        }
    }
}

impl Default for ElevenLabsConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(), // Must be provided by user
            api_url: "https://api.elevenlabs.io/v1".to_string(),
            default_voice_id: "21m00Tcm4TlvDq8ikWAM".to_string(), // Rachel voice
            model_id: "eleven_monolingual_v1".to_string(),
            stability: 0.5,
            similarity_boost: 0.5,
        }
    }
}

impl Default for OpenAITtsConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_url: "https://api.openai.com/v1/audio/speech".to_string(),
            model: "tts-1".to_string(),
            voice: "alloy".to_string(),
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000, // Common for speech recognition
            channels: 1,       // Mono
            bits_per_sample: 16,
            buffer_size: 1024,
            input_device: None,  // Use default device
            output_device: None, // Use default device
        }
    }
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            energy_threshold: 0.01,
            silence_duration_ms: 1000,   // 1 second of silence to stop
            min_speech_duration_ms: 500, // Minimum 0.5 seconds of speech
        }
    }
}

impl VoiceConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_whisper_api_key(mut self, api_key: String) -> Self {
        self.stt.whisper.api_key = Some(api_key);
        self
    }

    pub fn with_elevenlabs_api_key(mut self, api_key: String) -> Self {
        self.tts.elevenlabs.api_key = api_key;
        self
    }

    pub fn with_openai_tts_api_key(mut self, api_key: String) -> Self {
        self.tts.openai.api_key = api_key;
        self
    }

    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.audio.sample_rate = sample_rate;
        self
    }

    pub fn with_voice_id(mut self, voice_id: String) -> Self {
        self.tts.elevenlabs.default_voice_id = voice_id;
        self
    }

    pub fn with_language(mut self, language: String) -> Self {
        self.stt.language = Some(language);
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        // Validate STT configuration
        match self.stt.provider {
            SttProvider::Whisper | SttProvider::OpenAI => {
                if self.stt.whisper.api_key.is_none() {
                    return Err("Whisper API key is required".to_string());
                }
            }
            SttProvider::Local => {
                if self.stt.whisper.local_model_path.is_none() {
                    return Err("Local Whisper model path is required".to_string());
                }
            }
        }

        // Validate TTS configuration
        match self.tts.provider {
            TtsProvider::ElevenLabs => {
                if self.tts.elevenlabs.api_key.is_empty() {
                    return Err("ElevenLabs API key is required".to_string());
                }
            }
            TtsProvider::OpenAI => {
                if self.tts.openai.api_key.is_empty() {
                    return Err("OpenAI TTS API key is required".to_string());
                }
            }
            TtsProvider::Local => {
                // Local TTS doesn't require API keys
            }
        }

        // Validate audio configuration
        if self.audio.sample_rate == 0 {
            return Err("Invalid sample rate".to_string());
        }

        if self.audio.channels == 0 {
            return Err("Invalid channel count".to_string());
        }

        if self.audio.bits_per_sample == 0 {
            return Err("Invalid bits per sample".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VoiceConfig::default();
        assert!(!config.enabled); // Should be disabled by default
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.audio.channels, 1);
    }

    #[test]
    fn test_config_builder() {
        let config = VoiceConfig::new()
            .enable()
            .with_whisper_api_key("test-key".to_string())
            .with_elevenlabs_api_key("test-key".to_string())
            .with_sample_rate(44100)
            .with_language("es".to_string());

        assert!(config.enabled);
        assert_eq!(config.stt.whisper.api_key, Some("test-key".to_string()));
        assert_eq!(config.tts.elevenlabs.api_key, "test-key".to_string());
        assert_eq!(config.audio.sample_rate, 44100);
        assert_eq!(config.stt.language, Some("es".to_string()));
    }

    #[test]
    fn test_config_validation() {
        // Valid disabled config
        let config = VoiceConfig::default();
        assert!(config.validate().is_ok());

        // Invalid enabled config (missing API keys)
        let config = VoiceConfig::default().enable();
        assert!(config.validate().is_err());

        // Valid enabled config
        let config = VoiceConfig::default()
            .enable()
            .with_whisper_api_key("test-key".to_string())
            .with_elevenlabs_api_key("test-key".to_string());
        assert!(config.validate().is_ok());
    }
}