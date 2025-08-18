use crate::config::AudioConfig;
use rusty_ai_common::{Result, AssistantError};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct AudioProcessor {
    config: AudioConfig,
    recording_buffer: Arc<Mutex<Vec<u8>>>,
    is_capturing: Arc<Mutex<bool>>,
}

impl AudioProcessor {
    pub fn new(config: &AudioConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            recording_buffer: Arc::new(Mutex::new(Vec::new())),
            is_capturing: Arc::new(Mutex::new(false)),
        })
    }
    
    pub async fn preprocess_audio(&self, audio_data: Vec<u8>, format: &str) -> Result<Vec<u8>> {
        debug!("Preprocessing audio: {} bytes, format: {}", audio_data.len(), format);
        
        match format.to_lowercase().as_str() {
            "wav" => self.process_wav(audio_data).await,
            "mp3" => self.process_mp3(audio_data).await,
            "raw" | "pcm" => self.process_raw(audio_data).await,
            "base64" => self.process_base64(audio_data).await,
            _ => {
                warn!("Unsupported audio format: {}, treating as raw", format);
                self.process_raw(audio_data).await
            }
        }
    }
    
    async fn process_wav(&self, audio_data: Vec<u8>) -> Result<Vec<u8>> {
        debug!("Processing WAV audio");
        
        // Simple WAV processing - skip header and extract audio data
        if audio_data.len() < 44 {
            return Err(AssistantError::VoiceProcessing("Invalid WAV file: too short".to_string()));
        }
        
        // Check WAV header
        if &audio_data[0..4] != b"RIFF" || &audio_data[8..12] != b"WAVE" {
            return Err(AssistantError::VoiceProcessing("Invalid WAV file format".to_string()));
        }
        
        // Extract audio data (skip standard 44-byte header)
        let audio_samples = audio_data[44..].to_vec();
        
        // Apply basic preprocessing
        self.normalize_audio(audio_samples).await
    }
    
    async fn process_mp3(&self, _audio_data: Vec<u8>) -> Result<Vec<u8>> {
        // MP3 decoding would require a specialized library like minimp3
        // For now, return an error
        Err(AssistantError::VoiceProcessing(
            "MP3 processing not yet implemented. Please use WAV format.".to_string()
        ))
    }
    
    async fn process_raw(&self, audio_data: Vec<u8>) -> Result<Vec<u8>> {
        debug!("Processing raw PCM audio");
        self.normalize_audio(audio_data).await
    }
    
    async fn process_base64(&self, audio_data: Vec<u8>) -> Result<Vec<u8>> {
        debug!("Processing base64 encoded audio");
        
        let base64_string = String::from_utf8(audio_data)
            .map_err(|e| AssistantError::VoiceProcessing(format!("Invalid base64 string: {}", e)))?;
        
        let decoded = base64::decode(base64_string)
            .map_err(|e| AssistantError::VoiceProcessing(format!("Failed to decode base64: {}", e)))?;
        
        // Assume the decoded data is WAV and process accordingly
        self.process_wav(decoded).await
    }
    
    async fn normalize_audio(&self, mut audio_data: Vec<u8>) -> Result<Vec<u8>> {
        debug!("Normalizing audio: {} bytes", audio_data.len());
        
        // Ensure even number of bytes for 16-bit samples
        if audio_data.len() % 2 != 0 {
            audio_data.push(0);
        }
        
        // Convert to 16-bit samples
        let mut samples: Vec<i16> = audio_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        // Apply gain normalization if needed
        if let Some(max_sample) = samples.iter().map(|&s| s.abs()).max() {
            if max_sample > 0 {
                let gain = (i16::MAX as f32 * 0.8) / max_sample as f32;
                if gain > 1.0 {
                    for sample in &mut samples {
                        *sample = (*sample as f32 * gain) as i16;
                    }
                }
            }
        }
        
        // Convert back to bytes
        let normalized_data: Vec<u8> = samples
            .iter()
            .flat_map(|&sample| sample.to_le_bytes())
            .collect();
        
        debug!("Audio normalization completed");
        Ok(normalized_data)
    }
    
    pub async fn start_capture(&self) -> Result<()> {
        info!("Starting audio capture");
        
        let mut is_capturing = self.is_capturing.lock().await;
        if *is_capturing {
            return Err(AssistantError::VoiceProcessing("Audio capture already in progress".to_string()));
        }
        
        *is_capturing = true;
        
        // Clear the buffer
        let mut buffer = self.recording_buffer.lock().await;
        buffer.clear();
        
        // In a real implementation, this would start the actual audio capture
        // using cpal or similar audio library
        info!("Audio capture started (mock implementation)");
        
        Ok(())
    }
    
    pub async fn stop_capture(&self) -> Result<Vec<u8>> {
        info!("Stopping audio capture");
        
        let mut is_capturing = self.is_capturing.lock().await;
        if !*is_capturing {
            return Err(AssistantError::VoiceProcessing("No audio capture in progress".to_string()));
        }
        
        *is_capturing = false;
        
        // Get the recorded audio
        let buffer = self.recording_buffer.lock().await;
        let audio_data = buffer.clone();
        
        info!("Audio capture stopped: {} bytes recorded", audio_data.len());
        Ok(audio_data)
    }
    
    pub async fn is_capturing(&self) -> bool {
        *self.is_capturing.lock().await
    }
    
    pub async fn check_devices(&self) -> Result<()> {
        debug!("Checking audio devices");
        
        // In a real implementation, this would check for available audio devices
        // using cpal or similar
        
        // Mock implementation - assume devices are available
        Ok(())
    }
    
    pub async fn get_supported_formats(&self) -> Vec<String> {
        vec![
            "wav".to_string(),
            "raw".to_string(),
            "pcm".to_string(),
            "base64".to_string(),
            // "mp3".to_string(), // Commented out until implemented
        ]
    }
    
    pub fn get_config(&self) -> &AudioConfig {
        &self.config
    }
    
    pub async fn convert_sample_rate(&self, audio_data: Vec<u8>, from_rate: u32, to_rate: u32) -> Result<Vec<u8>> {
        if from_rate == to_rate {
            return Ok(audio_data);
        }
        
        debug!("Converting sample rate from {} to {} Hz", from_rate, to_rate);
        
        // Simple linear interpolation for sample rate conversion
        // In production, you'd use a proper resampling library
        
        let samples: Vec<i16> = audio_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        let ratio = to_rate as f64 / from_rate as f64;
        let new_length = (samples.len() as f64 * ratio) as usize;
        let mut resampled = Vec::with_capacity(new_length);
        
        for i in 0..new_length {
            let original_index = i as f64 / ratio;
            let index = original_index as usize;
            
            if index < samples.len() {
                resampled.push(samples[index]);
            }
        }
        
        let resampled_data: Vec<u8> = resampled
            .iter()
            .flat_map(|&sample| sample.to_le_bytes())
            .collect();
        
        debug!("Sample rate conversion completed: {} -> {} samples", samples.len(), resampled.len());
        Ok(resampled_data)
    }
    
    pub async fn apply_noise_reduction(&self, audio_data: Vec<u8>) -> Result<Vec<u8>> {
        debug!("Applying noise reduction to {} bytes", audio_data.len());
        
        // Simple noise gate implementation
        let samples: Vec<i16> = audio_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        let noise_threshold = 100i16; // Threshold below which we consider it noise
        
        let denoised: Vec<i16> = samples
            .iter()
            .map(|&sample| {
                if sample.abs() < noise_threshold {
                    0 // Remove low-level noise
                } else {
                    sample
                }
            })
            .collect();
        
        let denoised_data: Vec<u8> = denoised
            .iter()
            .flat_map(|&sample| sample.to_le_bytes())
            .collect();
        
        debug!("Noise reduction completed");
        Ok(denoised_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_audio_processor_creation() {
        let config = AudioConfig::default();
        let processor = AudioProcessor::new(&config).unwrap();
        
        assert_eq!(processor.config.sample_rate, 16000);
        assert_eq!(processor.config.channels, 1);
    }
    
    #[tokio::test]
    async fn test_raw_audio_processing() {
        let config = AudioConfig::default();
        let processor = AudioProcessor::new(&config).unwrap();
        
        let test_data = vec![0u8; 1024];
        let result = processor.process_raw(test_data.clone()).await.unwrap();
        
        assert_eq!(result.len(), test_data.len());
    }
    
    #[tokio::test]
    async fn test_audio_capture() {
        let config = AudioConfig::default();
        let processor = AudioProcessor::new(&config).unwrap();
        
        assert!(!processor.is_capturing().await);
        
        processor.start_capture().await.unwrap();
        assert!(processor.is_capturing().await);
        
        let audio_data = processor.stop_capture().await.unwrap();
        assert!(!processor.is_capturing().await);
        
        // In mock implementation, we expect empty buffer
        assert_eq!(audio_data.len(), 0);
    }
    
    #[tokio::test]
    async fn test_supported_formats() {
        let config = AudioConfig::default();
        let processor = AudioProcessor::new(&config).unwrap();
        
        let formats = processor.get_supported_formats().await;
        assert!(formats.contains(&"wav".to_string()));
        assert!(formats.contains(&"raw".to_string()));
    }
}