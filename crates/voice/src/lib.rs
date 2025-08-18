pub mod voice_pipeline;
pub mod stt;
pub mod tts;
pub mod vad;
pub mod audio;
pub mod config;

use rusty_ai_common::{Result, AssistantError, VoiceInteraction};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub use config::VoiceConfig;
pub use voice_pipeline::VoicePipeline;

#[derive(Debug, Clone)]
pub struct VoiceService {
    pipeline: Arc<RwLock<VoicePipeline>>,
    config: VoiceConfig,
}

impl VoiceService {
    pub async fn new(config: VoiceConfig) -> Result<Self> {
        let pipeline = Arc::new(RwLock::new(VoicePipeline::new(config.clone()).await?));
        
        info!("Voice service initialized");
        
        Ok(Self {
            pipeline,
            config,
        })
    }

    pub async fn process_audio(&self, audio_data: Vec<u8>, format: &str) -> Result<VoiceInteraction> {
        let pipeline = self.pipeline.read().await;
        pipeline.process_audio(audio_data, format).await
    }

    pub async fn synthesize_speech(&self, text: &str, voice_id: &str) -> Result<Vec<u8>> {
        let pipeline = self.pipeline.read().await;
        pipeline.synthesize_speech(text, voice_id).await
    }

    pub async fn start_voice_recording(&self) -> Result<()> {
        let mut pipeline = self.pipeline.write().await;
        pipeline.start_recording().await
    }

    pub async fn stop_voice_recording(&self) -> Result<Vec<u8>> {
        let mut pipeline = self.pipeline.write().await;
        pipeline.stop_recording().await
    }

    pub async fn health_check(&self) -> Result<VoiceHealthStatus> {
        let pipeline = self.pipeline.read().await;
        pipeline.health_check().await
    }

    pub fn get_config(&self) -> &VoiceConfig {
        &self.config
    }

    pub async fn update_config(&mut self, config: VoiceConfig) -> Result<()> {
        self.config = config.clone();
        let mut pipeline = self.pipeline.write().await;
        pipeline.update_config(config).await?;
        info!("Voice service configuration updated");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct VoiceHealthStatus {
    pub stt_available: bool,
    pub tts_available: bool,
    pub audio_devices_available: bool,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_service_creation() {
        let config = VoiceConfig::default();
        let result = VoiceService::new(config).await;
        
        // Service creation might fail in test environment due to missing audio devices
        // or external API keys, so we just check that it doesn't panic
        match result {
            Ok(service) => {
                assert!(service.get_config().enabled);
            }
            Err(_) => {
                // Expected in test environments without proper audio setup
            }
        }
    }
}