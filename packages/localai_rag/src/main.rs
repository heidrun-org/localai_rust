//! Command line for local document collections.

#![allow(non_snake_case)]

mod collection;
#[path = "documentChunks.rs"]
mod documentChunks;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use collection::Collection;
use localai_embeddings::DEFAULT_MODEL_ID;
use serde::Serialize;
use std::io::{self, Write};
use std::path::PathBuf;

/// Global options and the operation to run.
#[derive(Parser)]
#[command(about = "Index and search named collections of local documents")]
struct CommandLine {
	/// Collection storage directory, separate from document roots.
	#[arg(long = "data-directory", global = true, env = "LOCALAI_RAG_HOME")]
	dataDirectory: Option<PathBuf>,
	/// Requested collection operation.
	#[command(subcommand)]
	command: Command,
}

/// Supported collection operations.
#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Command {
	/// Register a source directory and an embedding model.
	Create {
		/// Collection name.
		#[arg(long)]
		collection: String,
		/// Source directory containing Markdown and text files.
		#[arg(long)]
		root: PathBuf,
		/// Hugging Face model identifier supported by Local AI Embeddings.
		#[arg(long, default_value = DEFAULT_MODEL_ID)]
		model: String,
	},
	/// Refresh the saved index after creating, editing, or removing files.
	Sync {
		/// Collection name.
		#[arg(long)]
		collection: String,
	},
	/// List saved collections and when they were last refreshed.
	List,
	/// Search a saved index and return matching source excerpts in JSON.
	Search {
		/// Collection name.
		#[arg(long)]
		collection: String,
		/// Query in ordinary language.
		#[arg(long)]
		query: String,
		/// Maximum documents.
		#[arg(long, default_value_t = 5)]
		limit: usize,
		/// Maximum matching chunks returned for each document.
		#[arg(long = "chunks-per-document", default_value_t = 2)]
		chunksPerDocument: usize,
	},
}

impl CommandLine {
	/// Runs the requested operation and emits its result as JSON.
	fn run(self) -> Result<()> {
		let parent = match self.dataDirectory {
			Some(directory) => directory,
			None => dirs::data_local_dir()
				.context("Could not locate the user's application data directory")?
				.join("heidrun-localai-rust")
				.join("rag-collections"),
		};
		match self.command {
			Command::Create {
				collection,
				root,
				model,
			} => Self::_writeJson(Collection::create(&parent, &collection, &root, model)?),
			Command::Sync { collection } => Self::_writeJson(Collection::open(&parent, &collection)?.sync()?),
			Command::List => Self::_writeJson(Collection::list(&parent)?),
			Command::Search {
				collection,
				query,
				limit,
				chunksPerDocument,
			} => Self::_writeJson(Collection::open(&parent, &collection)?.search(&query, limit, chunksPerDocument)?),
		}
	}

	/// Writes value as one JSON response and finishes the output line.
	fn _writeJson(value: impl Serialize) -> Result<()> {
		let mut output = io::stdout().lock();
		serde_json::to_writer(&mut output, &value)?;
		writeln!(output)?;
		Ok(())
	}
}

/// Runs the parsed command and reports errors to standard error.
fn main() -> Result<()> {
	CommandLine::parse().run()
}
