//! Persistent stores and cosine similarity search.

use anyhow::{anyhow, ensure, Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use serde::Serialize;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use uuid::Uuid;

///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////
//	Store — local embedding storage and search
///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////

const databaseName: &str = "store.sqlite3";
const similarityMeasure: &str = "cosine";

/// The caller-declared identity and length of an embedding model's output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EmbeddingModel {
	/// Provider of the model, such as huggingface.
	pub provider: String,
	/// Full model identifier within the provider.
	pub model: String,
	/// Exact model revision, or an unknown revision.
	pub revision: Option<String>,
	/// Number of values in each embedding.
	pub dimensions: usize,
}

/// A precomputed embedding and its source reference.
#[derive(Debug)]
pub struct StoredEmbedding {
	/// Identifier unique within the store.
	pub identifier: String,
	/// Caller-supplied reference to the source item.
	pub source: String,
	/// Vector produced by the declared model.
	pub vector: Vec<f32>,
}

/// The metadata of a store and its current embedding count.
#[derive(Debug, Serialize)]
pub struct StoreDescription {
	/// Generated identifier of the store.
	pub identifier: Uuid,
	/// Folder containing the database.
	pub directory: PathBuf,
	/// Human-readable description.
	pub description: String,
	/// Model configuration shared by all embeddings in the store.
	#[serde(rename = "embedding_model")]
	pub embeddingModel: EmbeddingModel,
	/// Measure used to rank search results.
	#[serde(rename = "similarity_measure")]
	pub similarityMeasure: &'static str,
	/// Number of saved embeddings.
	#[serde(rename = "embedding_count")]
	pub embeddingCount: usize,
}

/// An embedding returned by a similarity search.
#[derive(Debug, Serialize)]
pub struct SearchMatch {
	/// Identifier of the saved embedding.
	pub identifier: String,
	/// Reference to the source item.
	pub source: String,
	/// Cosine similarity to the query vector.
	pub score: f64,
}

/// An open database for one local vector store.
pub struct Store {
	/// Database connection for reads and writes.
	connection: Connection,
	/// Generated identifier persisted with the database.
	identifier: Uuid,
	/// Folder containing the database.
	directory: PathBuf,
	/// Human-readable description.
	description: String,
	/// Model and vector length used in the store.
	embeddingModel: EmbeddingModel,
}

impl Store {
	/// Returns the default parent directory for the current user.
	pub fn defaultParentDirectory() -> Result<PathBuf> {
		let dataDirectory = dirs::data_local_dir().context("Could not locate the user's application data directory")?;
		Ok(dataDirectory.join("heidrun-localai-rust").join("vector-stores"))
	}

	/// Creates a store in the parent directory with the supplied description and model configuration.
	/// Returns the open store with its generated identifier.
	pub fn create(parentDirectory: &Path, description: &str, embeddingModel: EmbeddingModel) -> Result<Self> {
		ensure!(
			description.trim().is_empty() == false,
			"Store description must be non-empty"
		);
		Self::_validateModel(&embeddingModel)?;
		fs::create_dir_all(parentDirectory)
			.with_context(|| format!("Could not create parent directory {}", parentDirectory.display()))?;

		let (identifier, directory) = loop {
			let identifier = Uuid::new_v4();
			let directory = parentDirectory.join(identifier.to_string());
			match fs::create_dir(&directory) {
				Ok(()) => break (identifier, directory),
				Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
				Err(error) => return Err(error).with_context(|| format!("Could not create {}", directory.display())),
			}
		};

		let store = Self::_initialize(&directory, identifier, description, &embeddingModel);
		if store.is_err() == true {
			if let Err(cleanupError) = fs::remove_dir_all(&directory) {
				return store.with_context(|| format!("Could not remove {}: {cleanupError}", directory.display()));
			}
		}
		store
	}

	/// Opens the store with the given identifier in the parent directory, or returns an error.
	pub fn open(parentDirectory: &Path, identifier: Uuid) -> Result<Self> {
		let directory = parentDirectory.join(identifier.to_string());
		let connection =
			Connection::open_with_flags(directory.join(databaseName), OpenFlags::SQLITE_OPEN_READ_WRITE)
				.with_context(|| format!("Could not open store {identifier} in {}", parentDirectory.display()))?;
		let (savedIdentifier, description, savedSimilarityMeasure, embeddingModel) = connection
			.query_row(
				"SELECT identifier, description, provider, model, revision, dimensions, similarity_measure FROM store_metadata",
				[],
				|row| {
					Ok((
						row.get::<_, String>(0)?,
						row.get(1)?,
						row.get::<_, String>(6)?,
						EmbeddingModel {
							provider: row.get(2)?,
							model: row.get(3)?,
							revision: row.get(4)?,
							dimensions: row.get::<_, u32>(5)? as usize,
						},
					))
				},
			)
			.with_context(|| format!("Could not read metadata for store {identifier}"))?;
		ensure!(
			savedIdentifier == identifier.to_string(),
			"Store identifier does not match its folder"
		);
		ensure!(
			savedSimilarityMeasure == similarityMeasure,
			"Unsupported similarity measure: {savedSimilarityMeasure}"
		);
		Self::_validateModel(&embeddingModel)?;

		Ok(Self {
			connection,
			identifier,
			directory,
			description,
			embeddingModel,
		})
	}

	/// Returns descriptions for the stores in the parent directory, ordered by identifier.
	/// A missing parent directory returns an empty list.
	pub fn list(parentDirectory: &Path) -> Result<Vec<StoreDescription>> {
		let entries = match fs::read_dir(parentDirectory) {
			Ok(entries) => entries,
			Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
			Err(error) => return Err(error).with_context(|| format!("Could not list {}", parentDirectory.display())),
		};
		let mut storeDescriptions = Vec::new();
		for entry in entries {
			let entry = entry?;
			if entry.file_type()?.is_dir() == false {
				continue;
			}
			let Ok(identifier) = Uuid::parse_str(&entry.file_name().to_string_lossy()) else {
				continue;
			};
			if entry.file_name() == identifier.to_string().as_str()
				&& entry.path().join(databaseName).try_exists()? == true
			{
				storeDescriptions.push(Self::open(parentDirectory, identifier)?.describe()?);
			}
		}
		storeDescriptions.sort_by_key(|storeDescription| storeDescription.identifier);
		Ok(storeDescriptions)
	}

	/// Returns the store's saved metadata and the current number of embeddings.
	pub fn describe(&self) -> Result<StoreDescription> {
		let embeddingCount = self
			.connection
			.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get::<_, i64>(0))?;
		Ok(StoreDescription {
			identifier: self.identifier,
			directory: self.directory.clone(),
			description: self.description.clone(),
			embeddingModel: self.embeddingModel.clone(),
			similarityMeasure,
			embeddingCount: usize::try_from(embeddingCount)?,
		})
	}

	/// Stores all embeddings in one transaction using the declared model.
	/// Returns the number inserted or an error. On failure, the saved embeddings stay untouched.
	pub fn add(&mut self, embeddingModel: &EmbeddingModel, storedEmbeddings: &[StoredEmbedding]) -> Result<usize> {
		self._checkModel(embeddingModel)?;
		let transaction = self.connection.transaction()?;
		{
			let mut insertStatement =
				transaction.prepare("INSERT INTO embeddings (identifier, source, vector) VALUES (?1, ?2, ?3)")?;
			for storedEmbedding in storedEmbeddings {
				ensure!(
					storedEmbedding.identifier.trim().is_empty() == false,
					"Embedding identifier is empty"
				);
				ensure!(
					storedEmbedding.source.trim().is_empty() == false,
					"Embedding source is empty"
				);
				Self::_vectorNorm(&storedEmbedding.vector, embeddingModel.dimensions)
					.with_context(|| format!("Invalid embedding {}", storedEmbedding.identifier))?;
				let bytes: Vec<u8> = storedEmbedding
					.vector
					.iter()
					.flat_map(|value| value.to_le_bytes())
					.collect();
				insertStatement
					.execute(params![storedEmbedding.identifier, storedEmbedding.source, bytes])
					.with_context(|| format!("Could not insert embedding {}", storedEmbedding.identifier))?;
			}
		}
		transaction.commit()?;
		Ok(storedEmbeddings.len())
	}

	/// Scores a query vector with the declared model and returns up to limit matches.
	/// Higher scores come first. Equal scores are ordered by embedding identifier.
	pub fn search(&self, embeddingModel: &EmbeddingModel, query: &[f32], limit: usize) -> Result<Vec<SearchMatch>> {
		self._checkModel(embeddingModel)?;
		let queryNorm = Self::_vectorNorm(query, embeddingModel.dimensions)?;
		let expectedBytes = query.len() * size_of::<f32>();
		if limit == 0 {
			return Ok(Vec::new());
		}

		let mut statement = self
			.connection
			.prepare("SELECT identifier, source, vector FROM embeddings")?;
		let mut rows = statement.query([])?;
		let mut matches = Vec::new();
		while let Some(row) = rows.next()? {
			let identifier: String = row.get(0)?;
			let bytes = row.get_ref(2)?.as_blob()?;
			ensure!(
				bytes.len() == expectedBytes,
				"Stored embedding {identifier} has an invalid byte length"
			);
			let (dotProduct, squaredNorm) = bytes
				.as_chunks::<4>()
				.0
				.iter()
				.zip(query)
				.map(|(value, queryValue)| (f64::from(f32::from_le_bytes(*value)), f64::from(*queryValue)))
				.fold((0.0, 0.0), |(dotProduct, squaredNorm), (value, queryValue)| {
					(dotProduct + value * queryValue, squaredNorm + value * value)
				});
			ensure!(
				squaredNorm.is_finite() == true && squaredNorm > 0.0,
				"Stored embedding {identifier} has an invalid vector norm"
			);
			let score = (dotProduct / (squaredNorm.sqrt() * queryNorm)).clamp(-1.0, 1.0);
			matches.push(SearchMatch {
				identifier,
				source: row.get(1)?,
				score,
			});
		}

		matches.sort_unstable_by(|first, second| {
			second
				.score
				.total_cmp(&first.score)
				.then_with(|| first.identifier.cmp(&second.identifier))
		});
		matches.truncate(limit);
		Ok(matches)
	}

	///////////////////////////////////////////////////////////////////////////////
	///////////////////////////////////////////////////////////////////////////////
	//	Helpers
	///////////////////////////////////////////////////////////////////////////////
	///////////////////////////////////////////////////////////////////////////////

	/// Initializes the database in directory with the identifier, description, and model.
	/// Returns the open store after its schema and metadata have been committed.
	fn _initialize(
		directory: &Path,
		identifier: Uuid,
		description: &str,
		embeddingModel: &EmbeddingModel,
	) -> Result<Self> {
		let temporaryDatabase = directory.join("store.sqlite3.initializing");
		let mut connection = Connection::open(&temporaryDatabase)?;
		let transaction = connection.transaction()?;
		transaction.execute_batch(
			"CREATE TABLE store_metadata (
				singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
				identifier TEXT NOT NULL,
				description TEXT NOT NULL,
				provider TEXT NOT NULL,
				model TEXT NOT NULL,
				revision TEXT,
				dimensions INTEGER NOT NULL CHECK (dimensions > 0),
				similarity_measure TEXT NOT NULL
			) STRICT;
			CREATE TABLE embeddings (
				identifier TEXT PRIMARY KEY,
				source TEXT NOT NULL,
				vector BLOB NOT NULL
			) STRICT;",
		)?;
		transaction.execute(
			"INSERT INTO store_metadata VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
			params![
				identifier.to_string(),
				description,
				embeddingModel.provider,
				embeddingModel.model,
				embeddingModel.revision,
				i64::try_from(embeddingModel.dimensions)?,
				similarityMeasure,
			],
		)?;
		transaction.commit()?;
		connection.close().map_err(|(_, error)| error)?;
		let database = directory.join(databaseName);
		fs::rename(temporaryDatabase, &database)?;
		let connection = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
		Ok(Self {
			connection,
			identifier,
			directory: directory.to_path_buf(),
			description: description.to_owned(),
			embeddingModel: embeddingModel.clone(),
		})
	}

	/// Checks that the embedding model has meaningful identifiers and a positive vector length.
	fn _validateModel(embeddingModel: &EmbeddingModel) -> Result<()> {
		ensure!(
			embeddingModel.provider.trim().is_empty() == false,
			"Model provider must be non-empty"
		);
		ensure!(
			embeddingModel.model.trim().is_empty() == false,
			"Model identifier must be non-empty"
		);
		if let Some(revision) = &embeddingModel.revision {
			ensure!(
				revision.trim().is_empty() == false,
				"Model revision must be non-empty when supplied"
			);
		}
		ensure!(
			embeddingModel.dimensions > 0 && embeddingModel.dimensions <= i32::MAX as usize,
			"Embedding dimensions must be between 1 and {}",
			i32::MAX,
		);
		Ok(())
	}

	/// Checks the declared model identity and length against the stored configuration.
	fn _checkModel(&self, embeddingModel: &EmbeddingModel) -> Result<()> {
		let expected = &self.embeddingModel;
		ensure!(
			embeddingModel.provider == expected.provider
				&& embeddingModel.model == expected.model
				&& embeddingModel.revision == expected.revision,
			"Expected model {}/{} at revision {:?}, received {}/{} at revision {:?}",
			expected.provider,
			expected.model,
			expected.revision,
			embeddingModel.provider,
			embeddingModel.model,
			embeddingModel.revision,
		);
		ensure!(
			embeddingModel.dimensions == expected.dimensions,
			"Expected {} embedding values, received {}",
			expected.dimensions,
			embeddingModel.dimensions,
		);
		Ok(())
	}

	/// Returns the length of vector after checking its number of values and validity for cosine similarity.
	fn _vectorNorm(vector: &[f32], dimensions: usize) -> Result<f64> {
		ensure!(
			vector.len() == dimensions,
			"Expected {dimensions} embedding values, received {}",
			vector.len()
		);
		let squaredNorm: f64 = vector.iter().map(|value| f64::from(*value).powi(2)).sum();
		if squaredNorm.is_finite() == false || squaredNorm == 0.0 {
			return Err(anyhow!("An embedding requires finite values and a nonzero norm"));
		}
		Ok(squaredNorm.sqrt())
	}
}
