//! Local SQLite storage and exact similarity search for precomputed embeddings.

#![allow(non_snake_case, non_upper_case_globals)]

mod store;

pub use store::{EmbeddingModel, SearchMatch, Store, StoreDescription, StoredEmbedding};
