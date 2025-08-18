use crate::{auth::AuthenticatedUser, create_success_response, error::ApiResult};
use axum::{extract::State, routing::post, Json, Router};
use rusty_ai_core::AssistantCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct VoiceRequest {
    pub audio_data: String, // Base64 encoded audio
    pub format: String,     // "wav", "mp3", etc.
    pub session_id: Option<uuid::Uuid>,
}

#[derive(Serialize)]
pub struct VoiceResponse {
    pub transcript: String,
    pub response: String,
    pub audio_url: Option<String>,
    pub processing_time_ms: u64,
}

pub fn routes(core: Arc<AssistantCore>) -> Router {
    Router::new()
        .route("/process", post(process_voice))
        .route("/synthesize", post(synthesize_speech))
        .with_state(core)
}

async fn process_voice(
    State(_core): State<Arc<AssistantCore>>,
    _user: AuthenticatedUser,
    Json(_request): Json<VoiceRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // TODO: Implement voice processing pipeline
    // 1. Decode base64 audio data
    // 2. Process through STT (Whisper)
    // 3. Process transcript through intent classifier
    // 4. Generate response
    // 5. Synthesize response through TTS
    
    let response = VoiceResponse {
        transcript: "Voice processing not yet implemented".to_string(),
        response: "Voice processing is coming soon!".to_string(),
        audio_url: None,
        processing_time_ms: 0,
    };
    
    Ok(create_success_response(response))
}

async fn synthesize_speech(
    State(_core): State<Arc<AssistantCore>>,
    _user: AuthenticatedUser,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    // TODO: Implement text-to-speech
    // 1. Extract text from request
    // 2. Send to TTS service (ElevenLabs)
    // 3. Return audio URL or base64 data
    
    Ok(create_success_response(serde_json::json!({
        "message": "Speech synthesis not yet implemented",
        "audio_url": null
    })))
}