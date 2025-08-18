use crate::config::{SttConfig, SttProvider};
use async_trait::async_trait;
use rusty_ai_common::{Result, AssistantError};
use std::sync::Arc;
use tracing::{debug, error, info};

#[async_trait]
pub trait SpeechToText: Send + Sync {
    async fn transcribe(&self, audio_data: &[u8]) -> Result<String>;
    async fn health_check(&self) -> Result<()>;
    async fn get_supported_languages(&self) -> Result<Vec<String>>;
}

pub struct WhisperStt {
    config: SttConfig,
    client: reqwest::Client,
}

impl WhisperStt {
    pub fn new(config: SttConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self { config, client })
    }
}

#[async_trait]
impl SpeechToText for WhisperStt {
    async fn transcribe(&self, audio_data: &[u8]) -> Result<String> {
        debug!("Transcribing audio with Whisper: {} bytes", audio_data.len());
        
        let api_key = self.config.whisper.api_key.as_ref()
            .ok_or_else(|| AssistantError::Configuration("Whisper API key not configured".to_string()))?;
        
        // Create multipart form
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(audio_data.to_vec())
                .file_name("audio.wav")
                .mime_str("audio/wav").unwrap())
            .text("model", self.config.whisper.model.clone());
        
        let form = if let Some(language) = &self.config.language {
            form.text("language", language.clone())
        } else {
            form
        };
        
        let response = self.client
            .post(&self.config.whisper.api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| AssistantError::VoiceProcessing(format!("STT request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AssistantError::VoiceProcessing(
                format!("STT API error: {}", error_text)
            ));
        }
        
        let result: serde_json::Value = response.json().await
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to parse STT response: {}", e)))?;
        
        let transcript = result["text"].as_str()
            .unwrap_or("")
            .to_string();
        
        info!("Transcription completed: '{}'", transcript);
        Ok(transcript)
    }
    
    async fn health_check(&self) -> Result<()> {
        debug!("Performing Whisper health check");
        
        // Simple health check - verify API key is present
        if self.config.whisper.api_key.is_none() {
            return Err(AssistantError::Configuration("Whisper API key not configured".to_string()));
        }
        
        // Could add actual API ping here
        Ok(())
    }
    
    async fn get_supported_languages(&self) -> Result<Vec<String>> {
        Ok(vec![
            "en".to_string(), "es".to_string(), "fr".to_string(),
            "de".to_string(), "it".to_string(), "pt".to_string(),
            "ru".to_string(), "ja".to_string(), "ko".to_string(),
            "zh".to_string(),
        ])
    }
}

pub async fn create_stt_service(config: &SttConfig) -> Result<Arc<dyn SpeechToText + Send + Sync>> {
    match config.provider {
        SttProvider::Whisper | SttProvider::OpenAI => {
            let stt = WhisperStt::new(config.clone())?;
            Ok(Arc::new(stt))
        }
        SttProvider::Local => {
            // TODO: Implement local Whisper model
            Err(AssistantError::VoiceProcessing(
                "Local Whisper not yet implemented".to_string()
            ))
        }
    }
}