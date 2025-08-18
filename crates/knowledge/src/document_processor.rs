use anyhow::Result;

pub struct DocumentProcessor;

impl DocumentProcessor {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn process(&self, _content: &[u8]) -> Result<String> {
        Ok("Document processed".to_string())
    }
}