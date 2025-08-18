//! Knowledge base module for document processing and semantic search

pub mod document_processor;
pub mod semantic_search;
pub mod vector_store;

pub use document_processor::DocumentProcessor;
pub use semantic_search::SemanticSearch;
pub use vector_store::VectorStore;