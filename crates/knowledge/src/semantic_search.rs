use anyhow::Result;

pub struct SemanticSearch;

impl SemanticSearch {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn search(&self, query: &str) -> Result<Vec<String>> {
        Ok(vec![format!("Result for: {}", query)])
    }
}