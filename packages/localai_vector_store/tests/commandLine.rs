//! Tests the installed command with its public input and output formats.

#![allow(non_snake_case)]

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////
//	CommandLine — run the vector store command
///////////////////////////////////////////////////////////////////////////////
///////////////////////////////////////////////////////////////////////////////

/// Runs the compiled command against a temporary parent folder.
struct CommandLine;

impl CommandLine {
	/// Runs arguments with the selected parent directory and returns the process output.
	fn run(parentDirectory: &Path, arguments: &[&str]) -> Result<Output> {
		Ok(Command::new(env!("CARGO_BIN_EXE_localai_vector_store"))
			.arg("--parent-dir")
			.arg(parentDirectory)
			.args(arguments)
			.output()?)
	}

	/// Returns the successful command output as JSON, or reports the command error.
	fn json(output: Output) -> Result<Value> {
		ensure!(
			output.status.success() == true,
			"{}",
			String::from_utf8_lossy(&output.stderr)
		);
		Ok(serde_json::from_slice(&output.stdout)?)
	}

	/// Creates a store for the test model in parentDirectory and returns its identifier.
	fn create(parentDirectory: &Path) -> Result<String> {
		let result = Self::createWithDescription(parentDirectory, "Test documents")?;
		Ok(result["identifier"]
			.as_str()
			.context("Missing store identifier")?
			.to_owned())
	}

	/// Creates a store with description in parentDirectory and returns the command output.
	fn createWithDescription(parentDirectory: &Path, description: &str) -> Result<Value> {
		Self::json(Self::run(
			parentDirectory,
			&[
				"create",
				"--description",
				description,
				"--model-provider",
				"test-provider",
				"--model",
				"test/model",
				"--dimensions",
				"2",
			],
		)?)
	}

	/// Writes a file with model and documents in the embedding command's format.
	fn writeEmbeddings(path: &Path, model: &str, documents: Value) -> Result<()> {
		let output = json!({
			"model": model, "backend": "cpu", "dimensions": 2, "documents": documents
		});
		Ok(fs::write(path, serde_json::to_vec(&output)?)?)
	}

	/// Returns a document with source, vector, and one perpendicular chunk.
	fn document(source: &str, vector: [f32; 2]) -> Value {
		json!({
			"source": source, "input_index": 0, "global_vector": vector,
			"chunks": [{"index": 0, "start_byte": 0, "end_byte": 1, "vector": [-vector[1], vector[0]]}]
		})
	}
}

#[test]
fn storesPersistAndSearchAcrossInvocations() -> Result<()> {
	let temporaryDirectory = tempfile::tempdir()?;
	let parentDirectory = temporaryDirectory.path().join("stores");
	let identifier = CommandLine::create(&parentDirectory)?;
	let secondIdentifier = CommandLine::create(&parentDirectory)?;
	assert_ne!(identifier, secondIdentifier);
	let readableDirectory = parentDirectory.join(format!("test_documents__{}", &identifier[..8]));
	assert!(readableDirectory.join("store.sqlite3").is_file());
	let legacyIdentifier = identifier.replace('_', "-");
	let legacyDirectory = parentDirectory.join(format!("test-documents--{}", &identifier[..8]));
	fs::rename(&readableDirectory, &legacyDirectory)?;
	{
		let connection = rusqlite::Connection::open(legacyDirectory.join("store.sqlite3"))?;
		let savedIdentifier: String =
			connection.query_row("SELECT identifier FROM store_metadata", [], |row| row.get(0))?;
		assert_eq!(savedIdentifier, identifier);
		assert_eq!(identifier.matches('_').count(), 4);
		assert_eq!(identifier.contains('-'), false);
		connection.execute("UPDATE store_metadata SET identifier = ?1", [&legacyIdentifier])?;
	}

	let documentsPath = temporaryDirectory.path().join("documents.json");
	let queryPath = temporaryDirectory.path().join("query.json");
	CommandLine::writeEmbeddings(
		&documentsPath,
		"test/model",
		json!([
			CommandLine::document("library.txt", [1.0, 0.0]),
			CommandLine::document("garden.txt", [-1.0, 0.0])
		]),
	)?;
	CommandLine::writeEmbeddings(
		&queryPath,
		"test/model",
		json!([CommandLine::document("text:1", [2.0, 0.0])]),
	)?;
	let documentsPathStr = documentsPath.to_str().context("Invalid document path")?;
	let queryPathStr = queryPath.to_str().context("Invalid query path")?;

	let added = CommandLine::json(CommandLine::run(
		&parentDirectory,
		&[
			"add",
			"--store",
			&legacyIdentifier,
			"--input",
			documentsPathStr,
			"--model-provider",
			"test-provider",
			"--vectors",
			"both",
		],
	)?)?;
	assert_eq!(added["inserted"], 4);
	assert_eq!(added["store"], identifier);

	let listed = CommandLine::json(CommandLine::run(&parentDirectory, &["list"])?)?;
	assert_eq!(listed.as_array().context("Expected a list of stores")?.len(), 2);
	let described = CommandLine::json(CommandLine::run(
		&parentDirectory,
		&["describe", "--store", &identifier],
	)?)?;
	assert_eq!(described["description"], "Test documents");
	assert_eq!(
		described["embedding_model"],
		json!({
			"provider": "test-provider", "model": "test/model", "revision": null, "dimensions": 2
		})
	);
	assert_eq!(described["embedding_count"], 4);
	assert_eq!(described["similarity_measure"], "cosine");

	let matches = CommandLine::json(CommandLine::run(
		&parentDirectory,
		&[
			"search",
			"--store",
			&identifier,
			"--query",
			queryPathStr,
			"--model-provider",
			"test-provider",
			"--limit",
			"2",
		],
	)?)?;
	assert_eq!(matches.as_array().context("Expected a list of matches")?.len(), 2);
	assert_eq!(
		matches[0],
		json!({
			"identifier": "[\"library.txt\",null]", "source": "library.txt", "score": 1.0
		})
	);
	assert_eq!(
		matches[1],
		json!({
			"identifier": "[\"garden.txt\",0]", "source": "garden.txt", "score": 0.0
		})
	);

	fs::create_dir(&readableDirectory)?;
	fs::write(readableDirectory.join("keep.txt"), "existing folder")?;
	let renamedDirectory = parentDirectory.join(format!("test_documents__{}_2", &identifier[..8]));
	let renamed = CommandLine::json(CommandLine::run(
		&parentDirectory,
		&["rename-folder", "--store", &identifier],
	)?)?;
	assert_eq!(
		renamed["directory"].as_str().map(Path::new),
		Some(renamedDirectory.as_path())
	);
	assert_eq!(legacyDirectory.exists(), false);
	assert_eq!(
		fs::read_to_string(readableDirectory.join("keep.txt"))?,
		"existing folder"
	);
	assert_eq!(renamed["identifier"], identifier);
	assert_eq!(renamed["description"], "Test documents");
	assert_eq!(renamed["embedding_count"], 4);
	{
		let connection = rusqlite::Connection::open(renamedDirectory.join("store.sqlite3"))?;
		let savedIdentifier: String =
			connection.query_row("SELECT identifier FROM store_metadata", [], |row| row.get(0))?;
		assert_eq!(savedIdentifier, legacyIdentifier);
	}
	assert_eq!(
		CommandLine::json(CommandLine::run(
			&parentDirectory,
			&["rename-folder", "--store", &identifier],
		)?)?,
		renamed
	);
	let matchesAfterRename = CommandLine::json(CommandLine::run(
		&parentDirectory,
		&[
			"search",
			"--store",
			&identifier,
			"--query",
			queryPathStr,
			"--model-provider",
			"test-provider",
			"--limit",
			"2",
		],
	)?)?;
	assert_eq!(matchesAfterRename, matches);

	let listed = CommandLine::json(CommandLine::run(&parentDirectory, &["list"])?)?;
	assert!(listed
		.as_array()
		.context("Expected a list of stores")?
		.contains(&renamed));
	let addedAfterRename = CommandLine::json(CommandLine::run(
		&parentDirectory,
		&[
			"add",
			"--store",
			&identifier,
			"--input",
			queryPathStr,
			"--model-provider",
			"test-provider",
		],
	)?)?;
	assert_eq!(addedAfterRename["inserted"], 1);

	let unknownIdentifier = uuid::Uuid::new_v4().to_string();
	let missing = CommandLine::run(&parentDirectory, &["describe", "--store", &unknownIdentifier])?;
	assert_eq!(missing.status.success(), false);
	assert_eq!(parentDirectory.join(unknownIdentifier).exists(), false);
	Ok(())
}

#[test]
fn rejectedImportsLeaveSavedEmbeddingsUntouched() -> Result<()> {
	let temporaryDirectory = tempfile::tempdir()?;
	let parentDirectory = temporaryDirectory.path().join("stores");
	let identifier = CommandLine::create(&parentDirectory)?;
	let filePath = temporaryDirectory.path().join("input.json");
	let filePathStr = filePath.to_str().context("Invalid input path")?;
	let addArguments = [
		"add",
		"--store",
		&identifier,
		"--input",
		filePathStr,
		"--model-provider",
		"test-provider",
	];
	let searchArguments = [
		"search",
		"--store",
		&identifier,
		"--query",
		filePathStr,
		"--model-provider",
		"test-provider",
	];
	CommandLine::writeEmbeddings(
		&filePath,
		"test/model",
		json!([CommandLine::document("original", [1.0, 0.0])]),
	)?;
	CommandLine::json(CommandLine::run(&parentDirectory, &addArguments)?)?;

	for (model, vector, revision) in [
		("another/model", json!([1.0, 0.0]), &[][..]),
		("test/model", json!([1.0]), &[][..]),
		(
			"test/model",
			json!([1.0, 0.0]),
			&["--model-revision", "revision-one"][..],
		),
	] {
		CommandLine::writeEmbeddings(
			&filePath,
			model,
			json!([
				{"source": "new", "global_vector": vector, "chunks": []}
			]),
		)?;
		for operation in [addArguments, searchArguments] {
			let arguments = [operation.as_slice(), revision].concat();
			assert_eq!(CommandLine::run(&parentDirectory, &arguments)?.status.success(), false);
		}
	}

	CommandLine::writeEmbeddings(
		&filePath,
		"test/model",
		json!([
			CommandLine::document("new", [-1.0, 0.0]),
			CommandLine::document("original", [-1.0, 0.0]),
		]),
	)?;
	assert_eq!(
		CommandLine::run(&parentDirectory, &addArguments)?.status.success(),
		false
	);

	CommandLine::writeEmbeddings(
		&filePath,
		"test/model",
		json!([CommandLine::document("query", [1.0, 0.0])]),
	)?;
	let matches = CommandLine::json(CommandLine::run(&parentDirectory, &searchArguments)?)?;
	assert_eq!(
		matches,
		json!([{"identifier": "[\"original\",null]", "source": "original", "score": 1.0}])
	);
	Ok(())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn createsStoreInDefaultApplicationDataDirectory() -> Result<()> {
	let temporaryDirectory = tempfile::tempdir()?;
	let applicationData = temporaryDirectory.path().join("application-data");
	let mut command = Command::new(env!("CARGO_BIN_EXE_localai_vector_store"));
	command
		.env("HOME", temporaryDirectory.path())
		.env("XDG_DATA_HOME", &applicationData)
		.args([
			"create",
			"--description",
			"Default store",
			"--model-provider",
			"test-provider",
			"--model",
			"test/model",
			"--dimensions",
			"2",
		]);
	let store = CommandLine::json(command.output()?)?;
	let identifier = store["identifier"].as_str().context("Missing store identifier")?;
	#[cfg(target_os = "macos")]
	let applicationData = temporaryDirectory.path().join("Library/Application Support");
	let expectedDirectory = applicationData
		.join("heidrun-localai-rust")
		.join("vector-stores")
		.join(format!("default_store__{}", &identifier[..8]));
	assert_eq!(
		store["directory"].as_str().map(Path::new),
		Some(expectedDirectory.as_path())
	);
	assert!(expectedDirectory.join("store.sqlite3").is_file());
	Ok(())
}

#[test]
fn concurrentCreatesWithFallbackNamesKeepBothStores() -> Result<()> {
	let temporaryDirectory = tempfile::tempdir()?;
	let parentDirectory = temporaryDirectory.path();
	let (first, second) = std::thread::scope(|scope| {
		let first = scope.spawn(|| CommandLine::createWithDescription(parentDirectory, "../?!"));
		let second = CommandLine::createWithDescription(parentDirectory, "../?!");
		(first.join().expect("Creation thread panicked"), second)
	});
	let first = first?;
	let second = second?;
	assert_ne!(first["identifier"], second["identifier"]);
	for store in [&first, &second] {
		let identifier = store["identifier"].as_str().context("Missing store identifier")?;
		let expectedDirectory = parentDirectory.join(format!("store__{}", &identifier[..8]));
		assert_eq!(
			store["directory"].as_str().map(Path::new),
			Some(expectedDirectory.as_path())
		);
	}
	let listed = CommandLine::json(CommandLine::run(parentDirectory, &["list"])?)?;
	assert_eq!(listed.as_array().context("Expected a list of stores")?.len(), 2);
	Ok(())
}

#[test]
fn renameWaitsForOpenStoreToClose() -> Result<()> {
	let temporaryDirectory = tempfile::tempdir()?;
	let parentDirectory = temporaryDirectory.path();
	let identifier = CommandLine::create(parentDirectory)?;
	let readableDirectory = parentDirectory.join(format!("test_documents__{}", &identifier[..8]));
	let legacyDirectory = parentDirectory.join(identifier.replace('_', "-"));
	fs::rename(&readableDirectory, &legacyDirectory)?;

	let store = localai_vector_store::Store::open(
		parentDirectory,
		localai_vector_store::Store::parseIdentifier(&identifier)?,
	)?;
	let renameArguments = ["rename-folder", "--store", &identifier];
	let rejected = CommandLine::run(parentDirectory, &renameArguments)?;
	assert_eq!(rejected.status.success(), false);
	assert!(String::from_utf8_lossy(&rejected.stderr).contains("Close stores and retry"));
	assert!(legacyDirectory.join("store.sqlite3").is_file());

	drop(store);
	CommandLine::json(CommandLine::run(parentDirectory, &renameArguments)?)?;
	assert!(readableDirectory.join("store.sqlite3").is_file());
	Ok(())
}

/// Checks format selection before and after a subcommand using a populated store.
#[test]
fn listSupportsAllOutputFormats() -> Result<()> {
	let temporaryDirectory = tempfile::tempdir()?;
	let parentDirectory = temporaryDirectory.path();
	CommandLine::createWithDescription(parentDirectory, "Markdown *literal*")?;
	let defaultOutput = CommandLine::json(CommandLine::run(parentDirectory, &["list"])?)?;
	let jsonOutput = CommandLine::json(CommandLine::run(parentDirectory, &["list", "-f", "json"])?)?;
	assert_eq!(jsonOutput, defaultOutput);

	let textOutput = CommandLine::run(parentDirectory, &["--format", "text", "list"])?;
	assert_eq!(textOutput.status.success(), true);
	let text = String::from_utf8(textOutput.stdout)?;
	assert_eq!(text.contains(r#"description: "Markdown *literal*""#), true);
	assert_eq!(text.contains('\x1b'), false);

	let markdownOutput = CommandLine::run(parentDirectory, &["list", "-f", "markdown"])?;
	assert_eq!(markdownOutput.status.success(), true);
	let markdown = String::from_utf8(markdownOutput.stdout)?;
	assert_eq!(markdown.contains(r#"- **description:** "Markdown &#42;literal&#42;""#), true);
	assert_eq!(markdown.contains('\x1b'), false);

	let invalidOutput = CommandLine::run(parentDirectory, &["list", "-f", "invalid"])?;
	assert_eq!(invalidOutput.status.success(), false);
	assert_eq!(invalidOutput.stdout.is_empty(), true);
	Ok(())
}
