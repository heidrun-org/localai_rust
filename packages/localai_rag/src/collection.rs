//! Saved collection lifecycle and document ranking.

use crate::documentChunks::{DocumentChunks, SourceChunk};
use anyhow::{ensure, Context, Result};
use chrono::Utc;
use localai_embeddings::{EmbeddingConfig, EmbeddingEngine};
use localai_vector_store::{EmbeddingModel, SearchMatch, Store, StoredEmbedding};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// A named collection and its saved source and model configuration.
pub struct Collection {
	/// Collection name shown in search responses.
	name: String,
	/// Directory with the configuration, refresh lock, and vector store.
	directory: PathBuf,
	/// Configuration written when the collection is created.
	config: CollectionConfig,
}

/// Permanent collection configuration.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct CollectionConfig {
	/// Canonical source directory.
	root: PathBuf,
	/// Model identifier used by Local AI Embeddings.
	model: String,
	/// Local AI Vector Store identifier.
	#[serde(rename = "store_identifier")]
	storeIdentifier: String,
}

/// A collection shown by the list command.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CollectionDescription {
	/// Name accepted by other commands.
	collection: String,
	/// Source directory.
	root: PathBuf,
	/// Model identifier.
	model: String,
	/// Time of the last successful sync, when present.
	#[serde(rename = "last_successful_refresh")]
	lastSuccessfulRefresh: Option<String>,
}

/// Details of a completed refresh.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RefreshResult {
	/// Refreshed collection name.
	collection: String,
	/// Timestamp published with the new index.
	#[serde(rename = "last_successful_refresh")]
	lastSuccessfulRefresh: String,
	/// Files that supplied new chunks.
	#[serde(rename = "indexed_documents")]
	indexedDocuments: usize,
	/// Files whose saved embeddings were reused.
	#[serde(rename = "reused_documents")]
	reusedDocuments: usize,
	/// Count of stale chunks removed.
	#[serde(rename = "removed_chunks")]
	removedChunks: usize,
}

/// Metadata stored alongside each embedding.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct ChunkSource {
	/// Stable identifier derived from the absolute source path.
	#[serde(rename = "document_id")]
	documentId: String,
	/// Absolute path to the file.
	#[serde(rename = "source_path")]
	sourcePath: PathBuf,
	/// SHA-256 digest of the indexed file bytes.
	#[serde(rename = "content_hash")]
	contentHash: String,
	/// Exact excerpt and its coordinates.
	#[serde(flatten)]
	sourceChunk: SourceChunk,
}

/// A collection search response.
#[derive(Serialize)]
pub struct SearchResult {
	/// Searched collection name.
	collection: String,
	/// Highest-scoring documents in descending score order.
	documents: Vec<DocumentMatch>,
}

/// A document and its best matching excerpts.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct DocumentMatch {
	/// Stable identifier derived from the absolute source path.
	#[serde(rename = "document_id")]
	documentId: String,
	/// Absolute path to the file.
	#[serde(rename = "source_path")]
	sourcePath: PathBuf,
	/// SHA-256 digest of the indexed file bytes.
	#[serde(rename = "content_hash")]
	contentHash: String,
	/// Score of the best matching excerpt.
	score: f64,
	/// Highest-scoring excerpts in descending score order.
	chunks: Vec<ChunkMatch>,
}

/// One matching excerpt and its similarity score.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ChunkMatch {
	/// Stable identifier for an unchanged source file.
	#[serde(rename = "chunk_id")]
	chunkId: String,
	/// Cosine similarity to the query embedding.
	score: f64,
	/// Exact excerpt and its coordinates.
	#[serde(flatten)]
	sourceChunk: SourceChunk,
}

impl Collection {
	/// Creates name in parent, recording root and model. Returns the saved collection description.
	pub fn create(parent: &Path, name: &str, root: &Path, model: String) -> Result<CollectionDescription> {
		let directory = Self::_directory(parent, name)?;
		ensure!(directory.exists() == false, "Collection already exists: {name}");
		let root = root
			.canonicalize()
			.with_context(|| format!("Could not open source directory {}", root.display()))?;
		ensure!(
			root.is_dir() == true,
			"Source path is not a directory: {}",
			root.display()
		);
		fs::create_dir_all(parent)?;
		let stagedDirectory = tempfile::Builder::new().prefix(".creating-").tempdir_in(parent)?;
		let embeddingEngine = Self::_engine(&model, None)?;
		let store = Store::create(
			&stagedDirectory.path().join("vectors"),
			name,
			Self::_embeddingModel(&embeddingEngine),
		)?;
		let config = CollectionConfig {
			root,
			model,
			storeIdentifier: Store::formatIdentifier(store.describe()?.identifier),
		};
		drop(store);
		serde_json::to_writer(File::create(stagedDirectory.path().join("collection.json"))?, &config)?;
		File::create(stagedDirectory.path().join("refresh.lock"))?;
		fs::rename(stagedDirectory.path(), &directory)
			.with_context(|| format!("Could not publish collection {name}"))?;
		Self::open(parent, name)?._describe()
	}

	/// Opens the saved configuration for name in parent.
	pub fn open(parent: &Path, name: &str) -> Result<Self> {
		let directory = Self::_directory(parent, name)?;
		let config = serde_json::from_reader(
			File::open(directory.join("collection.json"))
				.with_context(|| format!("Could not open collection {name}"))?,
		)?;
		Ok(Self {
			name: name.to_owned(),
			directory,
			config,
		})
	}

	/// Lists collection descriptions in parent in name order.
	pub fn list(parent: &Path) -> Result<Vec<CollectionDescription>> {
		let entries = match fs::read_dir(parent) {
			Ok(entries) => entries,
			Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
			Err(error) => return Err(error.into()),
		};
		let mut collections = Vec::new();
		for entry in entries {
			let entry = entry?;
			if entry.file_type()?.is_dir() == false || entry.path().join("collection.json").is_file() == false {
				continue;
			}
			if let Some(name) = entry.file_name().to_str().filter(|name| Self::_validName(name) == true) {
				collections.push(Self::open(parent, name)?._describe()?);
			}
		}
		collections.sort_unstable_by(|first, second| first.collection.cmp(&second.collection));
		Ok(collections)
	}

	/// Refreshes changed and deleted documents atomically and reports the work performed.
	pub fn sync(&self) -> Result<RefreshResult> {
		let refreshLock = File::options()
			.read(true)
			.write(true)
			.open(self.directory.join("refresh.lock"))?;
		refreshLock.lock()?;
		let mut store = self._openStore(false)?;
		let savedModel = store.describe()?.embeddingModel;
		let savedRevision = savedModel.revision.as_deref();
		let mut savedFiles = HashMap::<PathBuf, Vec<(String, String)>>::new();
		if savedRevision.is_some() == true {
			for (identifier, source) in store.sources()? {
				let source: ChunkSource = serde_json::from_str(&source)?;
				savedFiles
					.entry(source.sourcePath)
					.or_default()
					.push((identifier, source.contentHash));
			}
		}

		let mut removedIdentifiers = Vec::new();
		let mut storedEmbeddings = Vec::new();
		let mut reusedDocuments = 0;
		let mut indexedDocuments = 0;
		let mut embeddingEngine = if savedRevision.is_none() == true {
			Some(Self::_engine(&self.config.model, None)?)
		} else {
			None
		};
		for (sourcePath, isMarkdown) in Self::_sourceFiles(&self.config.root)? {
			let bytes = fs::read(&sourcePath).with_context(|| format!("Could not read {}", sourcePath.display()))?;
			let contentHash = format!("sha256:{:x}", Sha256::digest(&bytes));
			if let Some(savedChunks) = savedFiles.remove(&sourcePath) {
				if savedChunks.iter().all(|(_, savedHash)| savedHash == &contentHash) == true {
					reusedDocuments += 1;
					continue;
				}
				removedIdentifiers.extend(savedChunks.into_iter().map(|(identifier, _)| identifier));
			}
			let text =
				std::str::from_utf8(&bytes).with_context(|| format!("File is not UTF-8: {}", sourcePath.display()))?;
			if text.trim().is_empty() == true {
				continue;
			}
			if embeddingEngine.is_none() == true {
				embeddingEngine = Some(Self::_engine(&self.config.model, savedRevision)?);
			}
			let embeddingEngine = embeddingEngine.as_mut().expect("model is loaded");
			let newEmbeddings = Self::_embedFile(embeddingEngine, &sourcePath, &contentHash, text, isMarkdown)
				.with_context(|| format!("Could not index {}", sourcePath.display()))?;
			storedEmbeddings.extend(newEmbeddings);
			indexedDocuments += 1;
		}
		removedIdentifiers.extend(savedFiles.into_values().flatten().map(|(identifier, _)| identifier));
		let embeddingModel = match embeddingEngine.as_ref() {
			Some(embeddingEngine) => Self::_embeddingModel(embeddingEngine),
			None => savedModel.clone(),
		};
		let lastSuccessfulRefresh = Utc::now().to_rfc3339();
		let removedChunks = if savedRevision.is_none() == true {
			store.replaceUnpinnedEmbeddings(&embeddingModel, &storedEmbeddings, &lastSuccessfulRefresh)?
		} else {
			store.applyRefresh(
				&embeddingModel,
				&removedIdentifiers,
				&storedEmbeddings,
				&lastSuccessfulRefresh,
			)?;
			removedIdentifiers.len()
		};
		Ok(RefreshResult {
			collection: self.name.clone(),
			lastSuccessfulRefresh,
			indexedDocuments,
			reusedDocuments,
			removedChunks,
		})
	}

	/// Embeds query and returns up to documentLimit documents with at most chunkLimit matches each.
	pub fn search(&self, query: &str, documentLimit: usize, chunkLimit: usize) -> Result<SearchResult> {
		ensure!(query.trim().is_empty() == false, "Search query cannot be empty");
		let store = self._openStore(true)?;
		let description = store.describe()?;
		let documents = if documentLimit == 0 || chunkLimit == 0 || description.embeddingCount == 0 {
			Vec::new()
		} else {
			let revision = description
				.embeddingModel
				.revision
				.as_deref()
				.context("Collection has no pinned model revision. Run sync to rebuild the index.")?;
			let mut embeddingEngine = Self::_engine(&self.config.model, Some(revision))?;
			let embedding = embeddingEngine.embed(&[query.to_owned()])?.remove(0);
			let matches = store.search(&Self::_embeddingModel(&embeddingEngine), &embedding.vector, usize::MAX)?;
			Self::_rankDocuments(matches, documentLimit, chunkLimit)?
		};
		Ok(SearchResult {
			collection: self.name.clone(),
			documents,
		})
	}

	/// Validates name before resolving the collection directory inside parent.
	fn _directory(parent: &Path, name: &str) -> Result<PathBuf> {
		ensure!(Self::_validName(name) == true, "Collection name must start with an ASCII letter or digit and contain only letters, digits, hyphens, or underscores");
		Ok(parent.join(name))
	}

	/// Checks whether name is a single safe directory name.
	fn _validName(name: &str) -> bool {
		name.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
			&& name
				.bytes()
				.all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
	}

	/// Reads the saved metadata and latest refresh time.
	fn _describe(&self) -> Result<CollectionDescription> {
		Ok(CollectionDescription {
			collection: self.name.clone(),
			root: self.config.root.clone(),
			model: self.config.model.clone(),
			lastSuccessfulRefresh: self._openStore(true)?.lastRefresh()?,
		})
	}

	/// Opens the collection store with read-only access when isReadOnly is true.
	fn _openStore(&self, isReadOnly: bool) -> Result<Store> {
		let identifier = Store::parseIdentifier(&self.config.storeIdentifier)?;
		let vectorDirectory = self.directory.join("vectors");
		if isReadOnly == true {
			Store::openReadOnly(&vectorDirectory, identifier)
		} else {
			Store::open(&vectorDirectory, identifier)
		}
	}

	/// Loads model on the default backend, using the pinned revision when one was saved.
	fn _engine(model: &str, revision: Option<&str>) -> Result<EmbeddingEngine> {
		let config = EmbeddingConfig {
			model_id: model.to_owned(),
			..EmbeddingConfig::default()
		};
		match revision {
			Some(revision) => EmbeddingEngine::loadAtRevision(config, revision),
			None => EmbeddingEngine::load(config),
		}
	}

	/// Returns the model identity and vector length used by the store.
	fn _embeddingModel(embeddingEngine: &EmbeddingEngine) -> EmbeddingModel {
		EmbeddingModel {
			provider: "huggingface".to_owned(),
			model: embeddingEngine.model_id().to_owned(),
			revision: Some(embeddingEngine.modelRevision().to_owned()),
			dimensions: embeddingEngine.dimensions(),
		}
	}

	/// Enumerates Markdown and text files under root recursively, ignoring symbolic links.
	fn _sourceFiles(root: &Path) -> Result<Vec<(PathBuf, bool)>> {
		let mut sourceFiles = Vec::new();
		let mut directories = vec![root.to_path_buf()];
		while let Some(directory) = directories.pop() {
			for entry in fs::read_dir(&directory).with_context(|| format!("Could not list {}", directory.display()))? {
				let entry = entry?;
				let fileType = entry.file_type()?;
				let sourcePath = entry.path();
				if fileType.is_dir() == true {
					directories.push(sourcePath);
				} else if fileType.is_file() == true {
					let extension = sourcePath
						.extension()
						.and_then(|extension| extension.to_str())
						.unwrap_or_default()
						.to_ascii_lowercase();
					match extension.as_str() {
						"md" | "markdown" => sourceFiles.push((sourcePath, true)),
						"txt" => sourceFiles.push((sourcePath, false)),
						_ => {}
					}
				}
			}
		}
		sourceFiles.sort_unstable();
		Ok(sourceFiles)
	}

	/// Splits and embeds text, attaching sourcePath, contentHash, and source coordinates to each result.
	fn _embedFile(
		embeddingEngine: &mut EmbeddingEngine,
		sourcePath: &Path,
		contentHash: &str,
		text: &str,
		isMarkdown: bool,
	) -> Result<Vec<StoredEmbedding>> {
		let pathText = sourcePath.to_str().context("Source path is not UTF-8")?;
		let documentId = format!("doc-{:x}", Sha256::digest(pathText.as_bytes()));
		let documentChunks = DocumentChunks::build(text, embeddingEngine, isMarkdown)?;
		let mut storedEmbeddings = Vec::with_capacity(documentChunks.len());
		for batch in documentChunks.chunks(8) {
			let inputs = batch
				.iter()
				.map(|chunk| chunk.embeddingText.clone())
				.collect::<Vec<_>>();
			let embeddings = embeddingEngine.embed(&inputs)?;
			for (chunk, embedding) in batch.iter().zip(embeddings) {
				let sourceChunk = chunk.sourceChunk.clone();
				storedEmbeddings.push(StoredEmbedding {
					identifier: format!(
						"{documentId}-{contentHash}-{}-{}",
						sourceChunk.startByte, sourceChunk.endByte
					),
					source: serde_json::to_string(&ChunkSource {
						documentId: documentId.clone(),
						sourcePath: sourcePath.to_path_buf(),
						contentHash: contentHash.to_owned(),
						sourceChunk,
					})?,
					vector: embedding.vector,
				});
			}
		}
		Ok(storedEmbeddings)
	}

	/// Groups sorted chunk matches into documents and applies both result limits.
	fn _rankDocuments(
		matches: Vec<SearchMatch>,
		documentLimit: usize,
		chunkLimit: usize,
	) -> Result<Vec<DocumentMatch>> {
		let mut documentIndexes = HashMap::<String, usize>::new();
		let mut documents = Vec::<DocumentMatch>::new();
		for matchingChunk in matches {
			let source: ChunkSource = serde_json::from_str(&matchingChunk.source)?;
			let documentIndex = match documentIndexes.get(&source.documentId) {
				Some(index) => *index,
				None if documents.len() >= documentLimit => continue,
				None => {
					let index = documents.len();
					documentIndexes.insert(source.documentId.clone(), index);
					documents.push(DocumentMatch {
						documentId: source.documentId,
						sourcePath: source.sourcePath,
						contentHash: source.contentHash,
						score: matchingChunk.score,
						chunks: Vec::new(),
					});
					index
				}
			};
			let document = &mut documents[documentIndex];
			if document.chunks.len() < chunkLimit {
				document.chunks.push(ChunkMatch {
					chunkId: matchingChunk.identifier,
					score: matchingChunk.score,
					sourceChunk: source.sourceChunk,
				});
			}
		}
		Ok(documents)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Checks that five top chunks from one file do not prevent four other documents from appearing.
	#[test]
	fn ranksDocumentsAfterGroupingChunks() -> Result<()> {
		let mut matches = Vec::new();
		for index in 0_u32..10 {
			let documentId = format!("document-{}", index.saturating_sub(4));
			let source = ChunkSource {
				sourcePath: PathBuf::from(&documentId),
				documentId,
				contentHash: String::from("hash"),
				sourceChunk: SourceChunk {
					text: String::from("text"),
					startByte: 0,
					endByte: 4,
					startLine: 1,
					endLine: 1,
					markdown: None,
				},
			};
			matches.push(SearchMatch {
				identifier: index.to_string(),
				source: serde_json::to_string(&source)?,
				score: 1.0 - f64::from(index) / 10.0,
			});
		}
		let documents = Collection::_rankDocuments(matches, 5, 2)?;
		assert_eq!(documents.len(), 5);
		assert_eq!(documents[0].chunks.len(), 2);
		assert_eq!(documents[0].score, 1.0);
		assert_eq!(documents[4].documentId, "document-4");
		Ok(())
	}
}
