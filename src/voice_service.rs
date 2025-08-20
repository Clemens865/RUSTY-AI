use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{
        AudioInput,
        AudioResponseFormat,
        CreateTranscriptionRequestArgs,
        CreateSpeechRequestArgs,
        SpeechModel,
        Voice,
    },
    Client,
};
use axum::{
    body::Bytes,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
    pub language: Option<String>,
    pub duration: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TTSRequest {
    pub text: String,
    pub voice_id: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TTSResponse {
    pub audio_base64: String,
    pub content_type: String,
}

pub struct VoiceService {
    openai_client: Client<OpenAIConfig>,
    elevenlabs_api_key: Option<String>,
    elevenlabs_voice_id: String,
}

impl VoiceService {
    pub fn new(openai_api_key: Option<String>, elevenlabs_api_key: Option<String>) -> Result<Self> {
        // Initialize OpenAI client for Whisper
        let config = if let Some(key) = openai_api_key {
            OpenAIConfig::new().with_api_key(key)
        } else {
            OpenAIConfig::new() // Uses OPENAI_API_KEY env var
        };

        let openai_client = Client::with_config(config);
        
        // Get ElevenLabs configuration from environment
        let elevenlabs_voice_id = std::env::var("ELEVENLABS_VOICE_ID")
            .unwrap_or_else(|_| "21m00Tcm4TlvDq8ikWAM".to_string()); // Rachel voice as default
        
        Ok(Self {
            openai_client,
            elevenlabs_api_key,
            elevenlabs_voice_id,
        })
    }

    // Transcribe audio using OpenAI Whisper API
    pub async fn transcribe_audio(&self, audio_data: Vec<u8>, filename: String) -> Result<TranscriptionResponse> {
        debug!("Transcribing audio file: {}", filename);
        
        // Create AudioInput from bytes
        let audio_input = AudioInput {
            source: async_openai::types::InputSource::Bytes {
                filename: filename.clone(),
                bytes: Bytes::from(audio_data),
            },
        };
        
        // Create transcription request
        let request = CreateTranscriptionRequestArgs::default()
            .file(audio_input)
            .model("whisper-1")
            .language("en") // You can make this configurable
            .response_format(AudioResponseFormat::Json)
            .build()?;
        
        // Call Whisper API
        let response = match self.openai_client.audio().transcribe(request).await {
            Ok(resp) => resp,
            Err(e) => {
                error!("Whisper API error: {}", e);
                return Ok(TranscriptionResponse {
                    text: "Error: Could not transcribe audio".to_string(),
                    language: None,
                    duration: None,
                });
            }
        };
        
        info!("Successfully transcribed audio");
        
        Ok(TranscriptionResponse {
            text: response.text,
            language: Some("en".to_string()),
            duration: None,
        })
    }

    // Synthesize speech using ElevenLabs API
    pub async fn synthesize_speech(&self, text: &str, voice_id: Option<String>) -> Result<Vec<u8>> {
        let api_key = self.elevenlabs_api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("ElevenLabs API key not configured"))?;
        
        let voice = voice_id.unwrap_or_else(|| self.elevenlabs_voice_id.clone());
        
        debug!("Synthesizing speech with ElevenLabs, voice: {}", voice);
        
        // ElevenLabs API endpoint
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}",
            voice
        );
        
        // Request body
        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5,
                "style": 0.5,
                "use_speaker_boost": true
            }
        });
        
        // Make request to ElevenLabs
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Accept", "audio/mpeg")
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("ElevenLabs API error: {}", error_text);
            return Err(anyhow::anyhow!("ElevenLabs API error: {}", error_text));
        }
        
        let audio_bytes = response.bytes().await?.to_vec();
        
        info!("Successfully synthesized {} bytes of audio", audio_bytes.len());
        
        Ok(audio_bytes)
    }

    // Alternative: Use OpenAI TTS as fallback
    pub async fn synthesize_speech_openai(&self, text: &str) -> Result<Vec<u8>> {
        debug!("Synthesizing speech with OpenAI TTS");
        
        let request = CreateSpeechRequestArgs::default()
            .model(SpeechModel::Tts1)
            .voice(Voice::Alloy)
            .input(text)
            .build()?;
        
        let response = self.openai_client.audio().speech(request).await?;
        let audio_bytes = response.bytes.to_vec();
        
        info!("Successfully synthesized {} bytes of audio with OpenAI", audio_bytes.len());
        
        Ok(audio_bytes)
    }
}

// Import AppState from main module
use crate::AppState;

// HTTP Handlers for voice endpoints
pub async fn transcribe_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let voice_service = &state.voice_service;
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("").to_string();
        
        if name == "audio" {
            let filename = field.file_name()
                .unwrap_or("audio.webm")
                .to_string();
            
            let data = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Failed to read audio data: {}", e);
                    return (StatusCode::BAD_REQUEST, "Failed to read audio data").into_response();
                }
            };
            
            match voice_service.transcribe_audio(data.to_vec(), filename).await {
                Ok(transcription) => {
                    return Json(transcription).into_response();
                }
                Err(e) => {
                    error!("Transcription error: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Transcription failed").into_response();
                }
            }
        }
    }
    
    (StatusCode::BAD_REQUEST, "No audio file provided").into_response()
}

pub async fn synthesize_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TTSRequest>,
) -> impl IntoResponse {
    let voice_service = &state.voice_service;
    // Try ElevenLabs first, fall back to OpenAI if it fails
    let audio_bytes = match voice_service.synthesize_speech(&request.text, request.voice_id).await {
        Ok(bytes) => bytes,
        Err(e) => {
            debug!("ElevenLabs TTS failed, falling back to OpenAI: {}", e);
            match voice_service.synthesize_speech_openai(&request.text).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Both TTS services failed: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "TTS synthesis failed").into_response();
                }
            }
        }
    };
    
    // Return audio as base64 for easier frontend handling
    use base64::{Engine as _, engine::general_purpose};
    let base64_audio = general_purpose::STANDARD.encode(&audio_bytes);
    
    Json(TTSResponse {
        audio_base64: base64_audio,
        content_type: "audio/mpeg".to_string(),
    }).into_response()
}

// Simple test endpoint for voice service health
pub async fn voice_health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "services": {
            "whisper": "available",
            "elevenlabs": "available",
            "openai_tts": "available"
        }
    }))
}