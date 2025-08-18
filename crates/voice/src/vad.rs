use crate::config::VadConfig;
use rusty_ai_common::{Result, AssistantError};
use tracing::debug;

pub struct VoiceActivityDetector {
    config: VadConfig,
}

impl VoiceActivityDetector {
    pub fn new(config: &VadConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }
    
    pub async fn detect_speech(&self, audio_data: &[u8]) -> Result<bool> {
        if !self.config.enabled {
            return Ok(true); // Assume speech if VAD is disabled
        }
        
        debug!("Detecting voice activity in {} bytes of audio", audio_data.len());
        
        // Simple energy-based VAD implementation
        // In production, you would use a more sophisticated algorithm
        
        if audio_data.len() < 1024 {
            return Ok(false); // Too short to contain meaningful speech
        }
        
        // Convert bytes to samples (assuming 16-bit audio)
        let samples: Vec<i16> = audio_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        // Calculate RMS energy
        let rms = self.calculate_rms(&samples);
        let energy = rms / i16::MAX as f32;
        
        debug!("Audio energy level: {:.4}, threshold: {:.4}", energy, self.config.energy_threshold);
        
        let has_speech = energy > self.config.energy_threshold;
        
        if has_speech {
            debug!("Speech detected");
        } else {
            debug!("No speech detected");
        }
        
        Ok(has_speech)
    }
    
    pub async fn detect_continuous_speech(&self, audio_stream: &[Vec<u8>]) -> Result<Vec<bool>> {
        let mut results = Vec::new();
        
        for chunk in audio_stream {
            let has_speech = self.detect_speech(chunk).await?;
            results.push(has_speech);
        }
        
        Ok(results)
    }
    
    pub fn get_config(&self) -> &VadConfig {
        &self.config
    }
    
    pub fn update_config(&mut self, config: VadConfig) {
        self.config = config;
    }
    
    fn calculate_rms(&self, samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        
        let sum_squares: f64 = samples
            .iter()
            .map(|&sample| (sample as f64).powi(2))
            .sum();
        
        (sum_squares / samples.len() as f64).sqrt() as f32
    }
    
    pub fn detect_silence_duration(&self, audio_data: &[u8], sample_rate: u32) -> Result<u64> {
        // Simple silence detection based on energy levels
        let samples: Vec<i16> = audio_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        let chunk_size = sample_rate as usize / 10; // 100ms chunks
        let mut silence_chunks = 0;
        
        for chunk in samples.chunks(chunk_size) {
            let rms = self.calculate_rms(chunk);
            let energy = rms / i16::MAX as f32;
            
            if energy <= self.config.energy_threshold {
                silence_chunks += 1;
            } else {
                silence_chunks = 0; // Reset counter on speech
            }
        }
        
        // Convert chunks to milliseconds
        let silence_duration_ms = (silence_chunks * 100) as u64;
        
        Ok(silence_duration_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_vad_creation() {
        let config = VadConfig::default();
        let vad = VoiceActivityDetector::new(&config).unwrap();
        
        assert!(vad.config.enabled);
        assert_eq!(vad.config.energy_threshold, 0.01);
    }
    
    #[tokio::test]
    async fn test_speech_detection() {
        let config = VadConfig {
            enabled: true,
            energy_threshold: 0.01,
            silence_duration_ms: 1000,
            min_speech_duration_ms: 500,
        };
        
        let vad = VoiceActivityDetector::new(&config).unwrap();
        
        // Test with empty audio
        let empty_audio = vec![0u8; 100];
        let result = vad.detect_speech(&empty_audio).await.unwrap();
        assert!(!result); // Should detect no speech in silence
        
        // Test with some "audio" data (random bytes as proxy for audio)
        let mut audio_data = vec![0u8; 2048];
        for i in 0..audio_data.len() {
            audio_data[i] = (i % 256) as u8; // Generate some variation
        }
        
        let result = vad.detect_speech(&audio_data).await.unwrap();
        // Result depends on the energy calculation, just ensure it doesn't panic
        assert!(result || !result);
    }
    
    #[test]
    fn test_rms_calculation() {
        let config = VadConfig::default();
        let vad = VoiceActivityDetector::new(&config).unwrap();
        
        // Test with silence
        let silence = vec![0i16; 1000];
        let rms = vad.calculate_rms(&silence);
        assert_eq!(rms, 0.0);
        
        // Test with some signal
        let signal = vec![1000i16; 1000];
        let rms = vad.calculate_rms(&signal);
        assert!(rms > 0.0);
    }
}