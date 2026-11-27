//! Command line interface for Local AI Vector Store.

#![allow(non_snake_case)]

#[path = "outputFormat.rs"]
mod outputFormat;

use outputFormat::OutputFormat;
use anyhow::{ensure, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use localai_vector_store::{EmbeddingModel, Store, StoredEmbedding};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use uuid::Uuid;

///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////
//	Command Line Arguments
///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////

/// Store and search precomputed embeddings on your computer.
#[derive(Parser)]
#[command(version)]
struct Arguments {
	/// Select text, Markdown, or JSON output.
	#[arg(short = 'f', long, value_enum, global = true, value_name = "FORMAT")]
	format: Option<OutputFormat>,
	/// Parent folder for stores. Defaults to your application data directory.
	#[arg(long = "parent-dir", global = true)]
	parentDirectory: Option<PathBuf>,
	/// Store operation to perform.
	#[command(subcommand)]
	command: Command,
}

/// Operations available from the command line.
#[derive(Subcommand)]
enum Command {
	/// Create a store and print its identifier and metadata.
	Create {
		/// Human-readable description of the collection.
		#[arg(long)]
		description: String,
		/// Full model identifier within the provider.
		#[arg(long)]
		model: String,
		/// Number of values in each embedding.
		#[arg(long)]
		dimensions: usize,
		/// Model provider and optional revision.
		#[command(flatten)]
		modelIdentity: ModelIdentity,
	},
	/// List the stores in the selected parent directory.
	List,
	/// Show the metadata and embedding count for one store.
	Describe {
		/// The generated store identifier.
		#[arg(long, value_parser = Store::parseIdentifier)]
		store: Uuid,
	},
	/// Give an existing store a readable folder name.
	RenameFolder {
		/// The generated store identifier.
		#[arg(long, value_parser = Store::parseIdentifier)]
		store: Uuid,
	},
	/// Save embeddings from a localai_embeddings output file.
	Add {
		/// The generated store identifier.
		#[arg(long, value_parser = Store::parseIdentifier)]
		store: Uuid,
		/// JSON file produced by localai_embeddings.
		#[arg(long)]
		input: PathBuf,
		/// Which vectors to save from each source document.
		#[arg(long, value_enum, default_value = "document")]
		vectors: VectorSelection,
		/// Optional qualifier for source names, such as a project name.
		#[arg(long = "source-prefix")]
		sourcePrefix: Option<String>,
		/// Model provider and optional revision.
		#[command(flatten)]
		modelIdentity: ModelIdentity,
	},
	/// Rank saved embeddings against the document vector in a query file.
	Search {
		/// The generated store identifier.
		#[arg(long, value_parser = Store::parseIdentifier)]
		store: Uuid,
		/// localai_embeddings JSON file with one document.
		#[arg(long)]
		query: PathBuf,
		/// Maximum number of matches to return.
		#[arg(long, default_value_t = 10)]
		limit: usize,
		/// Model provider and optional revision.
		#[command(flatten)]
		modelIdentity: ModelIdentity,
	},
}

/// Model identity that is supplied separately from the embedding output file.
#[derive(Args)]
struct ModelIdentity {
	/// Provider of the model, such as huggingface.
	#[arg(long = "model-provider")]
	provider: String,
	/// Exact model revision. Omit when the revision is unknown.
	#[arg(long = "model-revision")]
	revision: Option<String>,
}

/// Embeddings to save from each document.
#[derive(Clone, Copy, ValueEnum)]
enum VectorSelection {
	/// Save the whole-document vector.
	Document,
	/// Save each chunk vector.
	Chunk,
	/// Save whole-document and chunk vectors.
	Both,
}

///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////
//	Embedding Input
///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////

/// Fields read from the localai_embeddings output.
#[derive(Deserialize)]
struct EmbeddingOutput {
	/// Full model identifier within the provider.
	model: String,
	/// Number of values in each vector.
	dimensions: usize,
	/// Output for each input source.
	documents: Vec<DocumentOutput>,
}

/// Source reference and vectors for one input document.
#[derive(Deserialize)]
struct DocumentOutput {
	/// Source name reported by localai_embeddings.
	source: String,
	/// Whole-document vector.
	#[serde(rename = "global_vector")]
	globalVector: Vec<f32>,
	/// Chunk vectors.
	chunks: Vec<ChunkOutput>,
}

/// Chunk identification and its vector within the source document.
#[derive(Deserialize)]
struct ChunkOutput {
	/// Position of the chunk within the source document.
	index: usize,
	/// Vector for this chunk.
	vector: Vec<f32>,
}

/// Result of adding embeddings to a store.
#[derive(Serialize)]
struct AddResult {
	/// Identifier of the store that received the embeddings.
	store: String,
	/// Number of embeddings added.
	inserted: usize,
}

impl EmbeddingOutput {
	/// Reads and returns the embedding output from path.
	fn read(path: &Path) -> Result<Self> {
		let file = File::open(path).with_context(|| format!("Could not open {}", path.display()))?;
		serde_json::from_reader(BufReader::new(file))
			.with_context(|| format!("Could not read embedding output from {}", path.display()))
	}

	/// Returns the declared model using the file's model identifier and vector length.
	fn embeddingModel(&self, modelIdentity: ModelIdentity) -> EmbeddingModel {
		EmbeddingModel {
			provider: modelIdentity.provider,
			model: self.model.clone(),
			revision: modelIdentity.revision,
			dimensions: self.dimensions,
		}
	}

	/// Returns the selected vectors with stable identifiers derived from their source and chunk position.
	/// If supplied, sourcePrefix scopes the source name before deriving identifiers.
	fn intoStoredEmbeddings(
		self,
		vectors: VectorSelection,
		sourcePrefix: Option<&str>,
	) -> Result<Vec<StoredEmbedding>> {
		ensure!(
			self.documents.is_empty() == false,
			"Input must contain at least one document"
		);
		let mut storedEmbeddings = Vec::new();
		for document in self.documents {
			let source = match sourcePrefix {
				Some(sourcePrefix) => format!("{sourcePrefix}/{}", document.source),
				None => document.source,
			};
			if matches!(vectors, VectorSelection::Document | VectorSelection::Both) == true {
				storedEmbeddings.push(StoredEmbedding {
					identifier: serde_json::to_string(&(&source, Option::<usize>::None))?,
					source: source.clone(),
					vector: document.globalVector,
				});
			}
			if matches!(vectors, VectorSelection::Chunk | VectorSelection::Both) == true {
				for chunk in document.chunks {
					storedEmbeddings.push(StoredEmbedding {
						identifier: serde_json::to_string(&(&source, Some(chunk.index)))?,
						source: source.clone(),
						vector: chunk.vector,
					});
				}
			}
		}
		ensure!(
			storedEmbeddings.is_empty() == false,
			"Input contains no vectors of the selected kind"
		);
		Ok(storedEmbeddings)
	}
}

///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////
//	Command Execution
///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////

impl Command {
	/// Runs this operation in parentDirectory and writes its formatted result to standard output.
	fn run(self, parentDirectory: &Path, outputFormat: OutputFormat) -> Result<()> {
		match self {
			Self::Create {
				description,
				model,
				dimensions,
				modelIdentity,
			} => {
				let embeddingModel = EmbeddingModel {
					provider: modelIdentity.provider,
					model,
					revision: modelIdentity.revision,
					dimensions,
				};
				outputFormat.write(&Store::create(parentDirectory, &description, embeddingModel)?.describe()?, None)
			}
			Self::List => outputFormat.write(&Store::list(parentDirectory)?, None),
			Self::Describe { store } => outputFormat.write(&Store::open(parentDirectory, store)?.describe()?, None),
			Self::RenameFolder { store } => outputFormat.write(&Store::renameFolder(parentDirectory, store)?, None),
			Self::Add {
				store,
				input,
				vectors,
				sourcePrefix,
				modelIdentity,
			} => {
				let embeddingOutput = EmbeddingOutput::read(&input)?;
				let embeddingModel = embeddingOutput.embeddingModel(modelIdentity);
				let storedEmbeddings = embeddingOutput.intoStoredEmbeddings(vectors, sourcePrefix.as_deref())?;
				let inserted = Store::open(parentDirectory, store)?.add(&embeddingModel, &storedEmbeddings)?;
				outputFormat.write(
					&AddResult {
						store: Store::formatIdentifier(store),
						inserted,
					},
					None,
				)
			}
			Self::Search {
				store,
				query,
				limit,
				modelIdentity,
			} => {
				let embeddingOutput = EmbeddingOutput::read(&query)?;
				let embeddingModel = embeddingOutput.embeddingModel(modelIdentity);
				let [document] = embeddingOutput.documents.as_slice() else {
					anyhow::bail!("Query input must contain exactly one document");
				};
				let matches =
					Store::open(parentDirectory, store)?.search(&embeddingModel, &document.globalVector, limit)?;
				outputFormat.write(&matches, None)
			}
		}
	}
}

/// Parses arguments and returns any error from the selected operation.
fn main() -> Result<()> {
	let arguments = Arguments::parse();
	let parentDirectory = match arguments.parentDirectory {
		Some(parentDirectory) => parentDirectory,
		None => Store::defaultParentDirectory()?,
	};
	arguments.command.run(&parentDirectory, arguments.format.unwrap_or(OutputFormat::Json))
}
