use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
           ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs},
    Client,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, error};

use crate::knowledge_service_simple::KnowledgeService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedInformation {
    pub category: InformationCategory,
    pub title: String,
    pub content: String,
    pub importance: ImportanceLevel,
    pub tags: Vec<String>,
    #[serde(skip_deserializing, default)]
    pub source_conversation_id: String,
    #[serde(skip_deserializing, default = "chrono::Utc::now")]
    pub extracted_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationCategory {
    Personal,       // Personal preferences, habits, routines
    Projects,       // Active projects, goals, tasks
    Knowledge,      // Facts, learnings, insights
    Relationships,  // People, contacts, social information
    Events,         // Important dates, memories, experiences
    Preferences,    // User preferences and settings
    Ideas,          // Creative ideas, thoughts, plans
    Other,          // Uncategorized information
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportanceLevel {
    Critical,   // Must remember (passwords, critical dates, etc.)
    High,       // Important personal information
    Medium,     // Useful context
    Low,        // Nice to have
}

pub struct MemoryService {
    openai_client: Client<OpenAIConfig>,
    knowledge_service: Arc<KnowledgeService>,
}

impl MemoryService {
    pub fn new(
        openai_api_key: Option<String>,
        knowledge_service: Arc<KnowledgeService>,
    ) -> Result<Self> {
        let config = if let Some(key) = openai_api_key {
            OpenAIConfig::new().with_api_key(key)
        } else {
            OpenAIConfig::new()
        };
        
        Ok(Self {
            openai_client: Client::with_config(config),
            knowledge_service,
        })
    }
    
    /// Extract important information from a conversation
    pub async fn extract_information(
        &self,
        conversation_id: &str,
        user_message: &str,
        assistant_response: &str,
    ) -> Result<Vec<ExtractedInformation>> {
        debug!("Extracting information from conversation {}", conversation_id);
        
        let extraction_prompt = format!(
            r#"Extract important information from this conversation to remember for future interactions.

User: {}
Assistant: {}

Look for: names, preferences, facts, goals, relationships, or anything important about the user.

Return JSON array. Each item needs: category, title, content, importance, tags.

Categories: personal, projects, knowledge, relationships, events, preferences, ideas, other
Importance: critical, high, medium, low

Example for "My name is John":
[{{"category":"personal","title":"User's Name","content":"User's name is John","importance":"high","tags":["name","identity"]}}]

Return [] if nothing to extract.
JSON:"#,
            user_message, assistant_response
        );
        
        let messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("You are an AI that extracts and categorizes important information from conversations for long-term memory storage.")
                    .build()?
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(extraction_prompt)
                    .build()?
            ),
        ];
        
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .messages(messages)
            .temperature(0.3) // Lower temperature for more consistent extraction
            .build()?;
        
        let response = self.openai_client
            .chat()
            .create(request)
            .await?;
        
        let mut content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "[]".to_string());
        
        // Strip markdown code fences if present
        if content.starts_with("```json") {
            content = content.strip_prefix("```json").unwrap_or(&content).to_string();
        } else if content.starts_with("```") {
            content = content.strip_prefix("```").unwrap_or(&content).to_string();
        }
        if content.ends_with("```") {
            content = content.strip_suffix("```").unwrap_or(&content).to_string();
        }
        content = content.trim().to_string();
        
        debug!("Raw extraction response (cleaned): {}", content);
        
        // Parse the JSON response - handle both array and object with items field
        let extracted: Vec<ExtractedInformation> = if content.trim().starts_with('[') {
            // Direct array response
            match serde_json::from_str(&content) {
                Ok(items) => items,
                Err(e) => {
                    error!("Failed to parse extraction array: {}. Content was: {}", e, content);
                    Vec::new()
                }
            }
        } else {
            // Try parsing as object with items field
            #[derive(Deserialize)]
            struct ExtractionResponse {
                items: Option<Vec<ExtractedInformation>>,
                extracted: Option<Vec<ExtractedInformation>>,
            }
            
            match serde_json::from_str::<ExtractionResponse>(&content) {
                Ok(resp) => resp.items.or(resp.extracted).unwrap_or_default(),
                Err(_) => {
                    // Try parsing as single object wrapped in array
                    match serde_json::from_str::<ExtractedInformation>(&content) {
                        Ok(single) => vec![single],
                        Err(e) => {
                            error!("Failed to parse extraction response: {}. Content was: {}", e, content);
                            Vec::new()
                        }
                    }
                }
            }
        };
        
        // Add metadata
        let extracted_with_metadata: Vec<ExtractedInformation> = extracted
            .into_iter()
            .map(|mut item| {
                item.source_conversation_id = conversation_id.to_string();
                item.extracted_at = chrono::Utc::now();
                item
            })
            .collect();
        
        info!("Extracted {} pieces of information from conversation", extracted_with_metadata.len());
        Ok(extracted_with_metadata)
    }
    
    /// Store extracted information in the knowledge base
    pub async fn store_extracted_information(
        &self,
        information: &ExtractedInformation,
    ) -> Result<()> {
        let category_str = format!("{:?}", information.category).to_lowercase();
        let importance_str = format!("{:?}", information.importance).to_lowercase();
        
        // Create a document for the knowledge base
        let title = format!("[{}] {}", category_str, information.title);
        
        let content = format!(
            "{}\n\nCategory: {}\nImportance: {}\nExtracted from: {}\nDate: {}",
            information.content,
            category_str,
            importance_str,
            information.source_conversation_id,
            information.extracted_at.to_rfc3339()
        );
        
        let mut tags = information.tags.clone();
        tags.push(category_str.clone());
        tags.push(importance_str);
        tags.push("extracted".to_string());
        
        // Store in Qdrant via knowledge service
        self.knowledge_service
            .store_document(
                title,
                content,
                format!("conversation_{}", information.source_conversation_id),
                tags,
            )
            .await?;
        
        info!("Stored extracted information: {}", information.title);
        Ok(())
    }
    
    /// Process a conversation and extract/store important information
    pub async fn process_conversation(
        &self,
        conversation_id: &str,
        user_message: &str,
        assistant_response: &str,
    ) -> Result<Vec<ExtractedInformation>> {
        // Extract information
        let extracted = self.extract_information(
            conversation_id,
            user_message,
            assistant_response
        ).await?;
        
        // Store each piece of extracted information
        for info in &extracted {
            if let Err(e) = self.store_extracted_information(info).await {
                error!("Failed to store extracted information: {}", e);
            }
        }
        
        Ok(extracted)
    }
    
    /// Get conversation summary for display
    pub async fn summarize_conversation(
        &self,
        messages: &[(String, String)], // (role, content) pairs
    ) -> Result<String> {
        let conversation_text = messages
            .iter()
            .map(|(role, content)| format!("{}: {}", role, content))
            .collect::<Vec<_>>()
            .join("\n");
        
        let summary_prompt = format!(
            "Summarize this conversation in 2-3 sentences:\n\n{}",
            conversation_text
        );
        
        let messages = vec![
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("You are a helpful AI that creates brief, informative summaries.")
                    .build()?
            ),
            ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(summary_prompt)
                    .build()?
            ),
        ];
        
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .messages(messages)
            .temperature(0.5)
            .max_tokens(100u32)
            .build()?;
        
        let response = self.openai_client
            .chat()
            .create(request)
            .await?;
        
        Ok(response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_else(|| "No summary available".to_string()))
    }
}