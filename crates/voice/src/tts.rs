use crate::config::{TtsConfig, TtsProvider};
use async_trait::async_trait;
use rusty_ai_common::{Result, AssistantError};
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info};

#[async_trait]
pub trait TextToSpeech: Send + Sync {
    async fn synthesize(&self, text: &str, voice_id: &str) -> Result<Vec<u8>>;
    async fn health_check(&self) -> Result<()>;
    async fn get_available_voices(&self) -> Result<Vec<String>>;
}

pub struct ElevenLabsTts {
    config: TtsConfig,
    client: reqwest::Client,
}

impl ElevenLabsTts {
    pub fn new(config: TtsConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self { config, client })
    }
}

#[async_trait]
impl TextToSpeech for ElevenLabsTts {
    async fn synthesize(&self, text: &str, voice_id: &str) -> Result<Vec<u8>> {
        debug!("Synthesizing speech with ElevenLabs: '{}'", text);
        
        if self.config.elevenlabs.api_key.is_empty() {
            return Err(AssistantError::Configuration("ElevenLabs API key not configured".to_string()));
        }
        
        let voice_id = if voice_id.is_empty() {
            &self.config.elevenlabs.default_voice_id
        } else {
            voice_id
        };
        
        let url = format!(
            "{}/text-to-speech/{}",
            self.config.elevenlabs.api_url,
            voice_id
        );
        
        let request_body = json!({
            "text": text,
            "model_id": self.config.elevenlabs.model_id,
            "voice_settings": {
                "stability": self.config.elevenlabs.stability,
                "similarity_boost": self.config.elevenlabs.similarity_boost
            }
        });
        
        let response = self.client
            .post(&url)
            .header("xi-api-key", &self.config.elevenlabs.api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AssistantError::VoiceProcessing(format!("TTS request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AssistantError::VoiceProcessing(
                format!("TTS API error: {}", error_text)
            ));
        }
        
        let audio_data = response.bytes().await
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to read TTS response: {}", e)))?;
        
        info!("Speech synthesis completed: {} bytes", audio_data.len());
        Ok(audio_data.to_vec())
    }
    
    async fn health_check(&self) -> Result<()> {
        debug!("Performing ElevenLabs health check");
        
        if self.config.elevenlabs.api_key.is_empty() {
            return Err(AssistantError::Configuration("ElevenLabs API key not configured".to_string()));
        }
        
        // Could add actual API ping here
        Ok(())
    }
    
    async fn get_available_voices(&self) -> Result<Vec<String>> {
        debug!("Getting available ElevenLabs voices");
        
        if self.config.elevenlabs.api_key.is_empty() {
            return Err(AssistantError::Configuration("ElevenLabs API key not configured".to_string()));
        }
        
        let url = format!("{}/voices", self.config.elevenlabs.api_url);
        
        let response = self.client
            .get(&url)
            .header("xi-api-key", &self.config.elevenlabs.api_key)
            .send()
            .await
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to get voices: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AssistantError::VoiceProcessing("Failed to retrieve voices".to_string()));
        }
        
        let result: serde_json::Value = response.json().await
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to parse voices response: {}", e)))?;
        
        let voices: Vec<String> = result["voices"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v["voice_id"].as_str().map(|s| s.to_string()))
            .collect();
        
        Ok(voices)
    }
}

pub struct OpenAITts {
    config: TtsConfig,
    client: reqwest::Client,
}

impl OpenAITts {
    pub fn new(config: TtsConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self { config, client })
    }
}

#[async_trait]
impl TextToSpeech for OpenAITts {
    async fn synthesize(&self, text: &str, _voice_id: &str) -> Result<Vec<u8>> {
        debug!("Synthesizing speech with OpenAI TTS: '{}'", text);
        
        if self.config.openai.api_key.is_empty() {
            return Err(AssistantError::Configuration("OpenAI API key not configured".to_string()));
        }
        
        let request_body = json!({
            "model": self.config.openai.model,
            "input": text,
            "voice": self.config.openai.voice
        });
        
        let response = self.client
            .post(&self.config.openai.api_url)
            .header("Authorization", format!("Bearer {}", self.config.openai.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AssistantError::VoiceProcessing(format!("TTS request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AssistantError::VoiceProcessing(
                format!("TTS API error: {}", error_text)
            ));
        }
        
        let audio_data = response.bytes().await
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to read TTS response: {}", e)))?;
        
        info!("OpenAI speech synthesis completed: {} bytes", audio_data.len());
        Ok(audio_data.to_vec())
    }
    
    async fn health_check(&self) -> Result<()> {
        if self.config.openai.api_key.is_empty() {
            return Err(AssistantError::Configuration("OpenAI API key not configured".to_string()));
        }
        Ok(())
    }
    
    async fn get_available_voices(&self) -> Result<Vec<String>> {
        Ok(vec![
            "alloy".to_string(),
            "echo".to_string(),
            "fable".to_string(),
            "onyx".to_string(),
            "nova".to_string(),
            "shimmer".to_string(),
        ])
    }
}

pub async fn create_tts_service(config: &TtsConfig) -> Result<Arc<dyn TextToSpeech + Send + Sync>> {
    match config.provider {
        TtsProvider::ElevenLabs => {
            let tts = ElevenLabsTts::new(config.clone())?;
            Ok(Arc::new(tts))
        }
        TtsProvider::OpenAI => {
            let tts = OpenAITts::new(config.clone())?;
            Ok(Arc::new(tts))
        }
        TtsProvider::Local => {
            Err(AssistantError::VoiceProcessing(
                "Local TTS not yet implemented".to_string()
            ))
        }
    }
}