use anyhow::Result;

pub struct VectorStore;

impl VectorStore {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn store(&self, _id: &str, _vector: Vec<f32>) -> Result<()> {
        Ok(())
    }
    
    pub async fn search(&self, _vector: Vec<f32>, _limit: usize) -> Result<Vec<String>> {
        Ok(vec![])
    }
}