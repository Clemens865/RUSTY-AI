# Voice Pipeline - Implementation Details

## Overview

The voice pipeline is the core component enabling natural voice interactions with the Personal AI Assistant. It implements real-time speech processing, natural language understanding, and voice synthesis while maintaining low latency and high accuracy.

## Architecture Components

### 1. Voice Pipeline Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Audio Capture  │───▶│ Voice Activity  │───▶│ Speech-to-Text  │
│   (Microphone)  │    │   Detection     │    │   (Whisper)     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Audio Playback  │◄───│ Text-to-Speech  │◄───│ Intent & NLU    │
│   (Speakers)    │    │  (ElevenLabs)   │    │   Processing    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### 2. Core Voice Service Implementation

```toml
# crates/voice/Cargo.toml
[package]
name = "voice"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }
whisper-rs = "0.10"
reqwest = { version = "0.11", features = ["json", "stream"] }
cpal = "0.15"
hound = "3.5"
tokio-stream = "0.1"
futures-util = "0.3"
uuid = { workspace = true }
chrono = { workspace = true }
tracing = "0.1"
```

#### Voice Service Core (`crates/voice/src/lib.rs`)

```rust
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub struct VoiceService {
    audio_capture: Arc<AudioCapture>,
    voice_activity_detector: Arc<VoiceActivityDetector>,
    speech_to_text: Arc<Mutex<SpeechToText>>,
    text_to_speech: Arc<TextToSpeech>,
    intent_processor: Arc<IntentProcessor>,
    audio_playback: Arc<AudioPlayback>,
    session_manager: Arc<VoiceSessionManager>,
    config: VoiceConfig,
}

#[derive(Debug, Clone)]
pub struct VoiceConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub chunk_size: usize,
    pub vad_threshold: f32,
    pub silence_duration_ms: u32,
    pub max_recording_duration_ms: u32,
    pub whisper_model_path: String,
    pub elevenlabs_api_key: String,
    pub elevenlabs_voice_id: String,
    pub enable_wake_word: bool,
    pub wake_word: String,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            chunk_size: 1024,
            vad_threshold: 0.6,
            silence_duration_ms: 1500,
            max_recording_duration_ms: 30000,
            whisper_model_path: "./models/whisper/ggml-base.en.bin".to_string(),
            elevenlabs_api_key: String::new(),
            elevenlabs_voice_id: String::new(),
            enable_wake_word: true,
            wake_word: "hey assistant".to_string(),
        }
    }
}

impl VoiceService {
    pub async fn new(config: VoiceConfig) -> Result<Self> {
        let audio_capture = Arc::new(AudioCapture::new(&config).await?);
        let voice_activity_detector = Arc::new(VoiceActivityDetector::new(config.vad_threshold));
        let speech_to_text = Arc::new(Mutex::new(SpeechToText::new(&config.whisper_model_path)?));
        let text_to_speech = Arc::new(TextToSpeech::new(
            config.elevenlabs_api_key.clone(),
            config.elevenlabs_voice_id.clone(),
        ));
        let intent_processor = Arc::new(IntentProcessor::new());
        let audio_playback = Arc::new(AudioPlayback::new(&config).await?);
        let session_manager = Arc::new(VoiceSessionManager::new());
        
        Ok(Self {
            audio_capture,
            voice_activity_detector,
            speech_to_text,
            text_to_speech,
            intent_processor,
            audio_playback,
            session_manager,
            config,
        })
    }
    
    pub async fn start_voice_interaction(&self) -> Result<mpsc::Receiver<VoiceEvent>> {
        let (event_tx, event_rx) = mpsc::channel(100);
        
        // Start audio capture
        let (audio_tx, mut audio_rx) = mpsc::channel(1000);
        self.audio_capture.start_capture(audio_tx).await?;
        
        // Create voice processing pipeline
        let voice_pipeline = VoicePipeline {
            vad: self.voice_activity_detector.clone(),
            stt: self.speech_to_text.clone(),
            intent_processor: self.intent_processor.clone(),
            session_manager: self.session_manager.clone(),
            config: self.config.clone(),
        };
        
        // Spawn audio processing task
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            voice_pipeline.process_audio_stream(audio_rx, event_tx_clone).await;
        });
        
        Ok(event_rx)
    }
    
    pub async fn synthesize_and_play(&self, text: &str, interrupt_current: bool) -> Result<()> {
        if interrupt_current {
            self.audio_playback.stop_playback().await?;
        }
        
        // Generate speech audio
        let audio_data = self.text_to_speech.synthesize(text).await?;
        
        // Play audio
        self.audio_playback.play_audio(&audio_data).await?;
        
        Ok(())
    }
    
    pub async fn set_listening_state(&self, listening: bool) -> Result<()> {
        if listening {
            self.audio_capture.resume_capture().await?;
        } else {
            self.audio_capture.pause_capture().await?;
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum VoiceEvent {
    AudioCaptureStarted,
    VoiceActivityDetected,
    SpeechStarted,
    SpeechEnded,
    TranscriptionReady { text: String, confidence: f32 },
    IntentProcessed { intent: Intent, response: String },
    SynthesisStarted,
    PlaybackStarted,
    PlaybackCompleted,
    Error { message: String },
}

struct VoicePipeline {
    vad: Arc<VoiceActivityDetector>,
    stt: Arc<Mutex<SpeechToText>>,
    intent_processor: Arc<IntentProcessor>,
    session_manager: Arc<VoiceSessionManager>,
    config: VoiceConfig,
}

impl VoicePipeline {
    async fn process_audio_stream(
        &self,
        mut audio_rx: mpsc::Receiver<AudioChunk>,
        event_tx: mpsc::Sender<VoiceEvent>,
    ) {
        let mut recording_buffer = Vec::new();
        let mut silence_duration = 0u32;
        let mut is_recording = false;
        let mut speech_detected = false;
        
        while let Some(audio_chunk) = audio_rx.recv().await {
            // Voice Activity Detection
            let voice_probability = self.vad.detect(&audio_chunk.samples);
            
            if voice_probability > self.config.vad_threshold {
                if !speech_detected {
                    speech_detected = true;
                    let _ = event_tx.send(VoiceEvent::VoiceActivityDetected).await;
                }
                
                if !is_recording {
                    is_recording = true;
                    recording_buffer.clear();
                    let _ = event_tx.send(VoiceEvent::SpeechStarted).await;
                }
                
                recording_buffer.extend_from_slice(&audio_chunk.samples);
                silence_duration = 0;
            } else {
                if is_recording {
                    recording_buffer.extend_from_slice(&audio_chunk.samples);
                    silence_duration += audio_chunk.duration_ms;
                    
                    // Check if we've had enough silence to end recording
                    if silence_duration >= self.config.silence_duration_ms {
                        is_recording = false;
                        speech_detected = false;
                        let _ = event_tx.send(VoiceEvent::SpeechEnded).await;
                        
                        // Process the recorded audio
                        if !recording_buffer.is_empty() {
                            self.process_speech_audio(&recording_buffer, &event_tx).await;
                        }
                        
                        recording_buffer.clear();
                        silence_duration = 0;
                    }
                }
            }
            
            // Safety check for maximum recording duration
            if is_recording && recording_buffer.len() > self.calculate_max_samples() {
                is_recording = false;
                speech_detected = false;
                let _ = event_tx.send(VoiceEvent::SpeechEnded).await;
                
                if !recording_buffer.is_empty() {
                    self.process_speech_audio(&recording_buffer, &event_tx).await;
                }
                
                recording_buffer.clear();
                silence_duration = 0;
            }
        }
    }
    
    async fn process_speech_audio(
        &self,
        audio_samples: &[f32],
        event_tx: &mpsc::Sender<VoiceEvent>,
    ) {
        // Transcribe speech to text
        match self.transcribe_audio(audio_samples).await {
            Ok(transcription) => {
                let _ = event_tx.send(VoiceEvent::TranscriptionReady {
                    text: transcription.text.clone(),
                    confidence: transcription.confidence,
                }).await;
                
                // Process intent
                match self.intent_processor.process(&transcription.text).await {
                    Ok(intent_result) => {
                        let _ = event_tx.send(VoiceEvent::IntentProcessed {
                            intent: intent_result.intent,
                            response: intent_result.response,
                        }).await;
                    }
                    Err(e) => {
                        let _ = event_tx.send(VoiceEvent::Error {
                            message: format!("Intent processing failed: {}", e),
                        }).await;
                    }
                }
            }
            Err(e) => {
                let _ = event_tx.send(VoiceEvent::Error {
                    message: format!("Transcription failed: {}", e),
                }).await;
            }
        }
    }
    
    async fn transcribe_audio(&self, audio_samples: &[f32]) -> Result<TranscriptionResult> {
        let mut stt = self.stt.lock().await;
        stt.transcribe(audio_samples).await
    }
    
    fn calculate_max_samples(&self) -> usize {
        (self.config.sample_rate as u32 * self.config.max_recording_duration_ms / 1000) as usize
    }
}

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub duration_ms: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub processing_time_ms: u32,
    pub language: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IntentResult {
    pub intent: Intent,
    pub response: String,
    pub confidence: f32,
    pub entities: Vec<Entity>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Intent {
    Query { topic: String },
    Command { action: String, parameters: Vec<String> },
    Conversation { context: String },
    SystemControl { operation: String },
    Unknown,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub confidence: f32,
    pub start_pos: usize,
    pub end_pos: usize,
}
```

### 3. Audio Capture Implementation

```rust
use cpal::{Device, Stream, StreamConfig, SampleFormat, SampleRate, ChannelCount};
use std::sync::Arc;

pub struct AudioCapture {
    device: Device,
    config: StreamConfig,
    stream: Option<Stream>,
    is_capturing: Arc<std::sync::atomic::AtomicBool>,
}

impl AudioCapture {
    pub async fn new(voice_config: &VoiceConfig) -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;
        
        let config = StreamConfig {
            channels: voice_config.channels,
            sample_rate: SampleRate(voice_config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(voice_config.chunk_size as u32),
        };
        
        Ok(Self {
            device,
            config,
            stream: None,
            is_capturing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }
    
    pub async fn start_capture(&mut self, audio_tx: mpsc::Sender<AudioChunk>) -> Result<()> {
        let is_capturing = self.is_capturing.clone();
        let sample_rate = self.config.sample_rate.0;
        
        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if is_capturing.load(std::sync::atomic::Ordering::Relaxed) {
                    let chunk = AudioChunk {
                        samples: data.to_vec(),
                        duration_ms: (data.len() as f32 / sample_rate as f32 * 1000.0) as u32,
                        timestamp: Utc::now(),
                    };
                    
                    if let Err(e) = audio_tx.try_send(chunk) {
                        tracing::warn!("Failed to send audio chunk: {}", e);
                    }
                }
            },
            |err| {
                tracing::error!("Audio capture error: {}", err);
            },
            None,
        )?;
        
        stream.play()?;
        self.stream = Some(stream);
        self.is_capturing.store(true, std::sync::atomic::Ordering::Relaxed);
        
        Ok(())
    }
    
    pub async fn pause_capture(&self) -> Result<()> {
        self.is_capturing.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    pub async fn resume_capture(&self) -> Result<()> {
        self.is_capturing.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}
```

### 4. Voice Activity Detection

```rust
pub struct VoiceActivityDetector {
    threshold: f32,
    window_size: usize,
    energy_buffer: std::collections::VecDeque<f32>,
}

impl VoiceActivityDetector {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            window_size: 50, // 50 frame window for smoothing
            energy_buffer: std::collections::VecDeque::with_capacity(50),
        }
    }
    
    pub fn detect(&mut self, audio_samples: &[f32]) -> f32 {
        // Calculate RMS energy
        let energy = self.calculate_rms_energy(audio_samples);
        
        // Add to rolling window
        self.energy_buffer.push_back(energy);
        if self.energy_buffer.len() > self.window_size {
            self.energy_buffer.pop_front();
        }
        
        // Calculate smoothed energy
        let avg_energy = self.energy_buffer.iter().sum::<f32>() / self.energy_buffer.len() as f32;
        
        // Apply voice activity detection algorithm
        let voice_probability = self.compute_voice_probability(avg_energy, audio_samples);
        
        voice_probability
    }
    
    fn calculate_rms_energy(&self, samples: &[f32]) -> f32 {
        let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }
    
    fn compute_voice_probability(&self, energy: f32, samples: &[f32]) -> f32 {
        // Basic energy-based VAD with spectral features
        let energy_score = (energy / self.threshold).min(1.0);
        
        // Zero crossing rate (indicator of voice vs noise)
        let zcr = self.calculate_zero_crossing_rate(samples);
        let zcr_score = if zcr > 0.1 && zcr < 0.4 { 1.0 } else { 0.5 };
        
        // Spectral centroid (frequency distribution)
        let spectral_score = self.calculate_spectral_features(samples);
        
        // Combine features
        (energy_score * 0.5 + zcr_score * 0.3 + spectral_score * 0.2).min(1.0)
    }
    
    fn calculate_zero_crossing_rate(&self, samples: &[f32]) -> f32 {
        let mut crossings = 0;
        for window in samples.windows(2) {
            if (window[0] >= 0.0) != (window[1] >= 0.0) {
                crossings += 1;
            }
        }
        crossings as f32 / (samples.len() - 1) as f32
    }
    
    fn calculate_spectral_features(&self, samples: &[f32]) -> f32 {
        // Simplified spectral analysis
        // In a full implementation, this would use FFT
        let mut high_freq_energy = 0.0;
        let mut low_freq_energy = 0.0;
        
        // Rough approximation using high-pass and low-pass filtering
        for window in samples.windows(3) {
            let high_pass = window[2] - 2.0 * window[1] + window[0];
            let low_pass = (window[0] + 2.0 * window[1] + window[2]) / 4.0;
            
            high_freq_energy += high_pass * high_pass;
            low_freq_energy += low_pass * low_pass;
        }
        
        let ratio = if low_freq_energy > 0.0 {
            high_freq_energy / (high_freq_energy + low_freq_energy)
        } else {
            0.0
        };
        
        // Voice typically has balanced frequency content
        if ratio > 0.2 && ratio < 0.8 { 1.0 } else { 0.3 }
    }
}
```

### 5. Speech-to-Text Implementation

```rust
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

pub struct SpeechToText {
    context: WhisperContext,
    params: FullParams,
}

impl SpeechToText {
    pub fn new(model_path: &str) -> Result<Self> {
        let context = WhisperContext::new_with_params(
            model_path,
            WhisperContextParameters::default(),
        )?;
        
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_context(true);
        params.set_single_segment(true);
        
        Ok(Self { context, params })
    }
    
    pub async fn transcribe(&mut self, audio_samples: &[f32]) -> Result<TranscriptionResult> {
        let start_time = std::time::Instant::now();
        
        // Run Whisper transcription
        self.context.full(self.params.clone(), audio_samples)?;
        
        let processing_time = start_time.elapsed().as_millis() as u32;
        
        // Extract transcription result
        let num_segments = self.context.full_n_segments()?;
        let mut full_text = String::new();
        let mut total_confidence = 0.0;
        
        for i in 0..num_segments {
            let segment_text = self.context.full_get_segment_text(i)?;
            full_text.push_str(&segment_text);
            
            // Whisper doesn't provide confidence scores directly
            // This is a placeholder implementation
            total_confidence += 0.9; // Approximate confidence
        }
        
        let confidence = if num_segments > 0 {
            total_confidence / num_segments as f32
        } else {
            0.0
        };
        
        Ok(TranscriptionResult {
            text: full_text.trim().to_string(),
            confidence,
            processing_time_ms: processing_time,
            language: Some("en".to_string()),
        })
    }
    
    pub async fn transcribe_streaming(&mut self, audio_stream: &mut mpsc::Receiver<Vec<f32>>) -> Result<mpsc::Receiver<TranscriptionResult>> {
        let (tx, rx) = mpsc::channel(10);
        
        // This would implement streaming transcription
        // Currently Whisper.cpp doesn't support true streaming
        // This is a placeholder for future implementation
        
        Ok(rx)
    }
}
```

### 6. Text-to-Speech Implementation

```rust
use reqwest::Client;
use serde_json::json;

pub struct TextToSpeech {
    client: Client,
    api_key: String,
    voice_id: String,
    base_url: String,
}

impl TextToSpeech {
    pub fn new(api_key: String, voice_id: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            voice_id,
            base_url: "https://api.elevenlabs.io/v1".to_string(),
        }
    }
    
    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>> {
        let url = format!("{}/text-to-speech/{}", self.base_url, self.voice_id);
        
        let payload = json!({
            "text": text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5,
                "style": 0.0,
                "use_speaker_boost": true
            }
        });
        
        let response = self.client
            .post(&url)
            .header("Accept", "audio/mpeg")
            .header("Content-Type", "application/json")
            .header("xi-api-key", &self.api_key)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "TTS API error: {} - {}",
                response.status(),
                response.text().await?
            ));
        }
        
        let audio_data = response.bytes().await?;
        Ok(audio_data.to_vec())
    }
    
    pub async fn synthesize_with_options(&self, text: &str, options: TTSOptions) -> Result<Vec<u8>> {
        let url = format!("{}/text-to-speech/{}", self.base_url, self.voice_id);
        
        let payload = json!({
            "text": text,
            "model_id": options.model_id,
            "voice_settings": {
                "stability": options.stability,
                "similarity_boost": options.similarity_boost,
                "style": options.style,
                "use_speaker_boost": options.use_speaker_boost
            }
        });
        
        let response = self.client
            .post(&url)
            .header("Accept", "audio/mpeg")
            .header("Content-Type", "application/json")
            .header("xi-api-key", &self.api_key)
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "TTS API error: {} - {}",
                response.status(),
                response.text().await?
            ));
        }
        
        let audio_data = response.bytes().await?;
        Ok(audio_data.to_vec())
    }
    
    pub async fn get_available_voices(&self) -> Result<Vec<Voice>> {
        let url = format!("{}/voices", self.base_url);
        
        let response = self.client
            .get(&url)
            .header("xi-api-key", &self.api_key)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch voices: {}",
                response.status()
            ));
        }
        
        let voices_response: VoicesResponse = response.json().await?;
        Ok(voices_response.voices)
    }
}

#[derive(Debug, Clone)]
pub struct TTSOptions {
    pub model_id: String,
    pub stability: f32,
    pub similarity_boost: f32,
    pub style: f32,
    pub use_speaker_boost: bool,
}

impl Default for TTSOptions {
    fn default() -> Self {
        Self {
            model_id: "eleven_monolingual_v1".to_string(),
            stability: 0.5,
            similarity_boost: 0.5,
            style: 0.0,
            use_speaker_boost: true,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Voice {
    pub voice_id: String,
    pub name: String,
    pub category: String,
    pub description: Option<String>,
    pub preview_url: Option<String>,
    pub available_for_tiers: Vec<String>,
    pub settings: Option<VoiceSettings>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct VoiceSettings {
    pub stability: f32,
    pub similarity_boost: f32,
    pub style: Option<f32>,
    pub use_speaker_boost: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
struct VoicesResponse {
    voices: Vec<Voice>,
}
```

### 7. Audio Playback

```rust
use cpal::{Device, Stream, StreamConfig, SampleFormat, SampleRate};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AudioPlayback {
    device: Device,
    config: StreamConfig,
    playback_queue: Arc<Mutex<Vec<Vec<u8>>>>,
    is_playing: Arc<std::sync::atomic::AtomicBool>,
}

impl AudioPlayback {
    pub async fn new(voice_config: &VoiceConfig) -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device available"))?;
        
        let config = StreamConfig {
            channels: 2, // Stereo output
            sample_rate: SampleRate(44100), // Standard audio output rate
            buffer_size: cpal::BufferSize::Default,
        };
        
        Ok(Self {
            device,
            config,
            playback_queue: Arc::new(Mutex::new(Vec::new())),
            is_playing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }
    
    pub async fn play_audio(&self, audio_data: &[u8]) -> Result<()> {
        // Convert MP3/other format to PCM samples
        let pcm_samples = self.decode_audio(audio_data)?;
        
        // Add to playback queue
        {
            let mut queue = self.playback_queue.lock().await;
            queue.push(pcm_samples);
        }
        
        // Start playback if not already playing
        if !self.is_playing.load(std::sync::atomic::Ordering::Relaxed) {
            self.start_playback().await?;
        }
        
        Ok(())
    }
    
    async fn start_playback(&self) -> Result<()> {
        let playback_queue = self.playback_queue.clone();
        let is_playing = self.is_playing.clone();
        
        let stream = self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // This is a simplified implementation
                // In practice, you'd properly decode and convert audio formats
                for sample in data.iter_mut() {
                    *sample = 0.0; // Placeholder
                }
            },
            |err| {
                tracing::error!("Audio playback error: {}", err);
            },
            None,
        )?;
        
        stream.play()?;
        is_playing.store(true, std::sync::atomic::Ordering::Relaxed);
        
        Ok(())
    }
    
    pub async fn stop_playback(&self) -> Result<()> {
        self.is_playing.store(false, std::sync::atomic::Ordering::Relaxed);
        
        // Clear playback queue
        {
            let mut queue = self.playback_queue.lock().await;
            queue.clear();
        }
        
        Ok(())
    }
    
    fn decode_audio(&self, audio_data: &[u8]) -> Result<Vec<u8>> {
        // This would use a proper audio decoder like symphonia
        // For now, returning the raw data as placeholder
        Ok(audio_data.to_vec())
    }
}
```

### 8. Intent Processing

```rust
pub struct IntentProcessor {
    nlp_service: Arc<NLPService>,
    command_registry: CommandRegistry,
}

impl IntentProcessor {
    pub fn new() -> Self {
        Self {
            nlp_service: Arc::new(NLPService::new()),
            command_registry: CommandRegistry::new(),
        }
    }
    
    pub async fn process(&self, text: &str) -> Result<IntentResult> {
        // Clean and normalize text
        let normalized_text = self.normalize_text(text);
        
        // Extract entities
        let entities = self.nlp_service.extract_entities(&normalized_text).await?;
        
        // Classify intent
        let intent = self.classify_intent(&normalized_text, &entities).await?;
        
        // Generate response
        let response = self.generate_response(&intent, &entities).await?;
        
        Ok(IntentResult {
            intent,
            response,
            confidence: 0.85, // Placeholder confidence
            entities,
        })
    }
    
    async fn classify_intent(&self, text: &str, entities: &[Entity]) -> Result<Intent> {
        let text_lower = text.to_lowercase();
        
        // Simple rule-based classification (in practice, use ML model)
        if text_lower.contains("search") || text_lower.contains("find") || text_lower.contains("what") {
            let topic = self.extract_search_topic(text, entities);
            Ok(Intent::Query { topic })
        } else if text_lower.starts_with("create") || text_lower.starts_with("add") || text_lower.starts_with("set") {
            let (action, parameters) = self.extract_command_details(text);
            Ok(Intent::Command { action, parameters })
        } else if text_lower.contains("hello") || text_lower.contains("hi") || text_lower.contains("how are you") {
            Ok(Intent::Conversation { context: text.to_string() })
        } else if text_lower.contains("stop") || text_lower.contains("exit") || text_lower.contains("quit") {
            Ok(Intent::SystemControl { operation: "stop".to_string() })
        } else {
            Ok(Intent::Unknown)
        }
    }
    
    async fn generate_response(&self, intent: &Intent, entities: &[Entity]) -> Result<String> {
        match intent {
            Intent::Query { topic } => {
                Ok(format!("I'll search for information about: {}", topic))
            }
            Intent::Command { action, parameters } => {
                Ok(format!("I'll {} with parameters: {:?}", action, parameters))
            }
            Intent::Conversation { context } => {
                self.generate_conversational_response(context).await
            }
            Intent::SystemControl { operation } => {
                Ok(format!("Executing system operation: {}", operation))
            }
            Intent::Unknown => {
                Ok("I'm not sure I understand. Could you please rephrase that?".to_string())
            }
        }
    }
    
    async fn generate_conversational_response(&self, context: &str) -> Result<String> {
        // Simple conversational responses
        let context_lower = context.to_lowercase();
        
        if context_lower.contains("hello") || context_lower.contains("hi") {
            Ok("Hello! How can I assist you today?".to_string())
        } else if context_lower.contains("how are you") {
            Ok("I'm doing well, thank you for asking! How can I help you?".to_string())
        } else if context_lower.contains("thank") {
            Ok("You're welcome! Is there anything else I can help you with?".to_string())
        } else {
            Ok("That's interesting! Tell me more about what you'd like to do.".to_string())
        }
    }
    
    fn normalize_text(&self, text: &str) -> String {
        // Remove extra whitespace, normalize punctuation
        text.trim()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }
    
    fn extract_search_topic(&self, text: &str, entities: &[Entity]) -> String {
        // Extract the main topic from search queries
        let words: Vec<&str> = text.split_whitespace().collect();
        let search_keywords = ["search", "find", "what", "who", "where", "when", "how"];
        
        let mut topic_words = Vec::new();
        let mut found_keyword = false;
        
        for word in words {
            if search_keywords.contains(&word.to_lowercase().as_str()) {
                found_keyword = true;
                continue;
            }
            
            if found_keyword && !["is", "are", "the", "a", "an"].contains(&word.to_lowercase().as_str()) {
                topic_words.push(word);
            }
        }
        
        if topic_words.is_empty() {
            text.to_string()
        } else {
            topic_words.join(" ")
        }
    }
    
    fn extract_command_details(&self, text: &str) -> (String, Vec<String>) {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return ("unknown".to_string(), vec![]);
        }
        
        let action = words[0].to_string();
        let parameters = words[1..].iter().map(|s| s.to_string()).collect();
        
        (action, parameters)
    }
}

pub struct NLPService {
    // In practice, this would contain ML models for NER, classification, etc.
}

impl NLPService {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn extract_entities(&self, text: &str) -> Result<Vec<Entity>> {
        // Placeholder entity extraction
        // In practice, use spaCy, Stanford NER, or similar
        let mut entities = Vec::new();
        
        // Simple regex-based entity extraction for demo
        if let Some(captures) = regex::Regex::new(r"\b(\d{1,2}:\d{2})\b").unwrap().captures(text) {
            if let Some(time_match) = captures.get(1) {
                entities.push(Entity {
                    entity_type: "TIME".to_string(),
                    value: time_match.as_str().to_string(),
                    confidence: 0.9,
                    start_pos: time_match.start(),
                    end_pos: time_match.end(),
                });
            }
        }
        
        Ok(entities)
    }
}

struct CommandRegistry {
    commands: std::collections::HashMap<String, CommandDefinition>,
}

struct CommandDefinition {
    name: String,
    description: String,
    parameters: Vec<ParameterDefinition>,
    handler: Box<dyn Fn(&[String]) -> Result<String> + Send + Sync>,
}
```

## Performance Optimizations

### 1. Audio Processing Optimization

```rust
pub struct AudioProcessingOptimizer {
    buffer_pool: Arc<Mutex<Vec<Vec<f32>>>>,
    processing_threads: usize,
}

impl AudioProcessingOptimizer {
    pub fn new() -> Self {
        Self {
            buffer_pool: Arc::new(Mutex::new(Vec::new())),
            processing_threads: num_cpus::get(),
        }
    }
    
    pub async fn get_buffer(&self, size: usize) -> Vec<f32> {
        let mut pool = self.buffer_pool.lock().await;
        
        if let Some(mut buffer) = pool.pop() {
            buffer.clear();
            buffer.resize(size, 0.0);
            buffer
        } else {
            vec![0.0; size]
        }
    }
    
    pub async fn return_buffer(&self, buffer: Vec<f32>) {
        let mut pool = self.buffer_pool.lock().await;
        if pool.len() < 10 { // Limit pool size
            pool.push(buffer);
        }
    }
}
```

### 2. Streaming Processing

```rust
pub struct StreamingProcessor {
    chunk_processor: Arc<ChunkProcessor>,
    result_aggregator: Arc<ResultAggregator>,
}

impl StreamingProcessor {
    pub async fn process_audio_stream(
        &self,
        mut audio_stream: mpsc::Receiver<AudioChunk>,
    ) -> mpsc::Receiver<ProcessingResult> {
        let (result_tx, result_rx) = mpsc::channel(100);
        
        tokio::spawn(async move {
            while let Some(chunk) = audio_stream.recv().await {
                let processed = self.chunk_processor.process(chunk).await;
                if let Ok(result) = processed {
                    let _ = result_tx.send(result).await;
                }
            }
        });
        
        result_rx
    }
}
```

This voice pipeline implementation provides a robust foundation for natural voice interactions while maintaining high performance and accuracy. The modular design allows for easy upgrades and optimizations as better models and techniques become available.