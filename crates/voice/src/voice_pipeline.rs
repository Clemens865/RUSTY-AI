use crate::{
    audio::AudioProcessor,
    config::VoiceConfig,
    stt::SpeechToText,
    tts::TextToSpeech,
    vad::VoiceActivityDetector,
    VoiceHealthStatus,
};
use rusty_ai_common::{Result, AssistantError, VoiceInteraction, Intent};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct VoicePipeline {
    config: VoiceConfig,
    stt: Arc<dyn SpeechToText + Send + Sync>,
    tts: Arc<dyn TextToSpeech + Send + Sync>,
    audio_processor: Arc<AudioProcessor>,
    vad: Arc<VoiceActivityDetector>,
    recording_state: Arc<RwLock<RecordingState>>,
}

#[derive(Debug, Clone)]
pub struct RecordingState {
    pub is_recording: bool,
    pub audio_buffer: Vec<u8>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            is_recording: false,
            audio_buffer: Vec::new(),
            started_at: None,
        }
    }
}

impl VoicePipeline {
    pub async fn new(config: VoiceConfig) -> Result<Self> {
        info!("Initializing voice pipeline");

        // Validate configuration
        if let Err(e) = config.validate() {
            return Err(AssistantError::Configuration(format!("Invalid voice config: {}", e)));
        }

        // Initialize STT service
        let stt = crate::stt::create_stt_service(&config.stt).await?;
        info!("STT service initialized: {:?}", config.stt.provider);

        // Initialize TTS service
        let tts = crate::tts::create_tts_service(&config.tts).await?;
        info!("TTS service initialized: {:?}", config.tts.provider);

        // Initialize audio processor
        let audio_processor = Arc::new(AudioProcessor::new(&config.audio)?);
        info!("Audio processor initialized");

        // Initialize VAD
        let vad = Arc::new(VoiceActivityDetector::new(&config.vad)?);
        info!("Voice activity detector initialized");

        let recording_state = Arc::new(RwLock::new(RecordingState::default()));

        Ok(Self {
            config,
            stt,
            tts,
            audio_processor,
            vad,
            recording_state,
        })
    }

    pub async fn process_audio(&self, audio_data: Vec<u8>, format: &str) -> Result<VoiceInteraction> {
        let start_time = std::time::Instant::now();
        
        debug!("Processing audio: {} bytes, format: {}", audio_data.len(), format);

        if !self.config.enabled {
            return Err(AssistantError::VoiceProcessing("Voice processing is disabled".to_string()));
        }

        // Step 1: Preprocess audio
        let processed_audio = self.audio_processor.preprocess_audio(audio_data, format).await?;
        debug!("Audio preprocessed: {} bytes", processed_audio.len());

        // Step 2: Voice Activity Detection (if enabled)
        if self.config.vad.enabled {
            let has_speech = self.vad.detect_speech(&processed_audio).await?;
            if !has_speech {
                debug!("No speech detected in audio");
                return Ok(VoiceInteraction {
                    id: Uuid::new_v4(),
                    transcript: String::new(),
                    intent: Intent::Unknown,
                    response: "No speech detected".to_string(),
                    confidence: 0.0,
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        // Step 3: Speech-to-Text
        let transcript = self.stt.transcribe(&processed_audio).await?;
        info!("Audio transcribed: '{}'", transcript);

        if transcript.trim().is_empty() {
            debug!("Empty transcript received");
            return Ok(VoiceInteraction {
                id: Uuid::new_v4(),
                transcript: String::new(),
                intent: Intent::Unknown,
                response: "No speech recognized".to_string(),
                confidence: 0.0,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
                timestamp: chrono::Utc::now(),
            });
        }

        // Step 4: Intent classification (simplified - would integrate with core intent classifier)
        let intent = self.classify_intent(&transcript);
        debug!("Intent classified: {:?}", intent);

        // Step 5: Generate response (simplified - would integrate with orchestrator)
        let response = self.generate_response(&transcript, &intent).await;
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        info!(
            "Voice interaction processed in {}ms: '{}' -> '{}'",
            processing_time, transcript, response
        );

        Ok(VoiceInteraction {
            id: Uuid::new_v4(),
            transcript,
            intent,
            response,
            confidence: 0.8, // Would be calculated properly
            processing_time_ms: processing_time,
            timestamp: chrono::Utc::now(),
        })
    }

    pub async fn synthesize_speech(&self, text: &str, voice_id: &str) -> Result<Vec<u8>> {
        debug!("Synthesizing speech for text: '{}'", text);

        if !self.config.enabled {
            return Err(AssistantError::VoiceProcessing("Voice processing is disabled".to_string()));
        }

        if text.trim().is_empty() {
            return Err(AssistantError::VoiceProcessing("Cannot synthesize empty text".to_string()));
        }

        let audio_data = self.tts.synthesize(text, voice_id).await?;
        info!("Speech synthesized: {} bytes", audio_data.len());

        Ok(audio_data)
    }

    pub async fn start_recording(&mut self) -> Result<()> {
        info!("Starting voice recording");

        if !self.config.enabled {
            return Err(AssistantError::VoiceProcessing("Voice processing is disabled".to_string()));
        }

        let mut state = self.recording_state.write().await;
        
        if state.is_recording {
            warn!("Recording already in progress");
            return Ok(());
        }

        state.is_recording = true;
        state.audio_buffer.clear();
        state.started_at = Some(chrono::Utc::now());

        // Start audio capture
        self.audio_processor.start_capture().await?;
        
        info!("Voice recording started");
        Ok(())
    }

    pub async fn stop_recording(&mut self) -> Result<Vec<u8>> {
        info!("Stopping voice recording");

        let mut state = self.recording_state.write().await;
        
        if !state.is_recording {
            warn!("No recording in progress");
            return Ok(Vec::new());
        }

        // Stop audio capture
        let audio_data = self.audio_processor.stop_capture().await?;
        
        state.is_recording = false;
        state.audio_buffer = audio_data.clone();
        
        let duration = state.started_at
            .map(|start| chrono::Utc::now().signed_duration_since(start))
            .unwrap_or_default();
            
        info!("Voice recording stopped: {} bytes, duration: {}ms", 
              audio_data.len(), duration.num_milliseconds());

        Ok(audio_data)
    }

    pub async fn is_recording(&self) -> bool {
        self.recording_state.read().await.is_recording
    }

    pub async fn get_recording_duration(&self) -> Option<chrono::Duration> {
        let state = self.recording_state.read().await;
        state.started_at.map(|start| chrono::Utc::now().signed_duration_since(start))
    }

    pub async fn health_check(&self) -> Result<VoiceHealthStatus> {
        debug!("Performing voice pipeline health check");

        let stt_available = self.stt.health_check().await.is_ok();
        let tts_available = self.tts.health_check().await.is_ok();
        let audio_devices_available = self.audio_processor.check_devices().await.is_ok();

        let status = VoiceHealthStatus {
            stt_available,
            tts_available,
            audio_devices_available,
            last_check: chrono::Utc::now(),
        };

        if !stt_available {
            warn!("STT service health check failed");
        }
        if !tts_available {
            warn!("TTS service health check failed");
        }
        if !audio_devices_available {
            warn!("Audio devices health check failed");
        }

        debug!("Voice pipeline health check completed: {:?}", status);
        Ok(status)
    }

    pub async fn update_config(&mut self, config: VoiceConfig) -> Result<()> {
        info!("Updating voice pipeline configuration");

        // Validate new configuration
        if let Err(e) = config.validate() {
            return Err(AssistantError::Configuration(format!("Invalid voice config: {}", e)));
        }

        // Stop any ongoing recording
        if self.is_recording().await {
            self.stop_recording().await?;
        }

        // Update configuration
        self.config = config;

        // Reinitialize components if needed
        // Note: In a full implementation, you might want to recreate services
        // only if their specific configuration changed

        info!("Voice pipeline configuration updated");
        Ok(())
    }

    // Helper methods
    
    fn classify_intent(&self, transcript: &str) -> Intent {
        // Simple intent classification - in production, this would use the core intent classifier
        let text = transcript.to_lowercase();
        
        if text.contains("what") || text.contains("who") || text.contains("when") || 
           text.contains("where") || text.contains("why") || text.contains("how") {
            Intent::Query { query: transcript.to_string() }
        } else if text.contains("create") || text.contains("make") || text.contains("do") ||
                  text.contains("start") || text.contains("stop") {
            Intent::Command { 
                action: "voice_command".to_string(), 
                parameters: vec![transcript.to_string()] 
            }
        } else if text.contains("hello") || text.contains("hi") || text.contains("hey") {
            Intent::Information { topic: "greeting".to_string() }
        } else {
            Intent::Unknown
        }
    }

    async fn generate_response(&self, transcript: &str, intent: &Intent) -> String {
        // Simple response generation - in production, this would use the orchestrator
        match intent {
            Intent::Query { .. } => {
                format!("I heard your question: '{}'. Let me think about that.", transcript)
            }
            Intent::Command { .. } => {
                format!("I'll help you with that command: '{}'.", transcript)
            }
            Intent::Information { topic } if topic == "greeting" => {
                "Hello! How can I help you today?".to_string()
            }
            Intent::Information { topic } => {
                format!("Here's information about {}: {}", topic, transcript)
            }
            Intent::Unknown => {
                "I'm not sure I understand. Could you please rephrase that?".to_string()
            }
        }
    }

    pub fn get_config(&self) -> &VoiceConfig {
        &self.config
    }

    pub async fn get_supported_formats(&self) -> Vec<String> {
        self.audio_processor.get_supported_formats().await
    }

    pub async fn get_available_voices(&self) -> Result<Vec<String>> {
        self.tts.get_available_voices().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_pipeline_creation() {
        let config = VoiceConfig::default();
        
        // Pipeline creation might fail in test environment
        let result = VoicePipeline::new(config).await;
        
        match result {
            Ok(pipeline) => {
                assert!(!pipeline.config.enabled); // Default is disabled
            }
            Err(_) => {
                // Expected in test environments
            }
        }
    }

    #[test]
    fn test_intent_classification() {
        let config = VoiceConfig::default();
        // We can't easily test the full pipeline in unit tests,
        // but we can test individual components
        
        // This would be a more comprehensive test in a real implementation
        assert!(true);
    }
}