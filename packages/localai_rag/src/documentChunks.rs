//! Source ranges and Markdown structure used for indexing and citation.

use anyhow::Result;
use localai_embeddings::EmbeddingEngine;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// A source excerpt with citation coordinates.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SourceChunk {
	/// Exact UTF-8 excerpt.
	pub text: String,
	/// Inclusive byte offset.
	#[serde(rename = "start_byte")]
	pub startByte: usize,
	/// Exclusive byte offset.
	#[serde(rename = "end_byte")]
	pub endByte: usize,
	/// One-based first source line.
	#[serde(rename = "start_line")]
	pub startLine: usize,
	/// One-based last source line.
	#[serde(rename = "end_line")]
	pub endLine: usize,
	/// Markdown structure when the file is Markdown.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub markdown: Option<Markdown>,
}

/// The Markdown structure surrounding one excerpt.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Markdown {
	/// Full paths from the outermost heading to the innermost.
	#[serde(rename = "heading_paths")]
	headingPaths: Vec<Vec<Heading>>,
	/// Type of source block included in the excerpt.
	#[serde(rename = "block_kind")]
	blockKind: BlockKind,
	/// Table information when applicable.
	#[serde(skip_serializing_if = "Option::is_none")]
	table: Option<Table>,
}

/// One heading in the section ancestry.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Heading {
	/// Markdown heading level from one to six.
	level: u8,
	/// Visible heading text.
	text: String,
}

/// Type of Markdown block represented by an excerpt.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum BlockKind {
	Heading,
	Paragraph,
	Table,
	CodeBlock,
}

/// Column labels and data row positions in one table excerpt.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct Table {
	/// Visible column names, in source order.
	columns: Vec<String>,
	/// First included data row, counting from one below the header.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "start_row")]
	startRow: Option<usize>,
	/// Last included data row, counting from one below the header.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(rename = "end_row")]
	endRow: Option<usize>,
	/// Explicit caption from the preceding source line.
	#[serde(skip_serializing_if = "Option::is_none")]
	title: Option<String>,
}

/// A source excerpt and the text to send to the model.
pub struct DocumentChunk {
	/// Exact source excerpt with locations.
	pub sourceChunk: SourceChunk,
	/// Excerpt with bounded Markdown context prepended.
	pub embeddingText: String,
}

/// Builds chunks in source order for one document.
pub struct DocumentChunks<'text> {
	/// Original document.
	text: &'text str,
	/// The collection's tokenizer and model limit.
	embeddingEngine: &'text EmbeddingEngine,
	/// Start offsets of source lines.
	lineStarts: Vec<usize>,
	/// Current heading ancestry.
	headings: Vec<Heading>,
	/// Completed chunks in source order.
	chunks: Vec<DocumentChunk>,
}

impl<'text> DocumentChunks<'text> {
	/// Splits text using embeddingEngine. Selects structure-aware parsing when isMarkdown is true.
	pub fn build(
		text: &'text str,
		embeddingEngine: &'text EmbeddingEngine,
		isMarkdown: bool,
	) -> Result<Vec<DocumentChunk>> {
		let mut documentChunks = Self {
			text,
			embeddingEngine,
			lineStarts: std::iter::once(0)
				.chain(text.match_indices('\n').map(|(index, _)| index + 1))
				.collect(),
			headings: Vec::new(),
			chunks: Vec::new(),
		};
		if isMarkdown == true {
			documentChunks._markdown()?;
		} else {
			documentChunks._append(0..text.len(), None, "", false)?;
		}
		Ok(documentChunks.chunks)
	}

	/// Uses parsed Markdown boundaries to keep headings, fenced code, and tables distinct.
	fn _markdown(&mut self) -> Result<()> {
		let markdownSource = Self::_separateTableCaptions(self.text);
		let mut events = Parser::new_ext(&markdownSource, Options::ENABLE_TABLES).into_offset_iter();
		let mut proseStart = 0;
		while let Some((event, range)) = events.next() {
			match event {
				Event::Start(Tag::Heading { level, .. }) => {
					self._prose(proseStart..range.start)?;
					let mut headingText = String::new();
					for (event, _) in events.by_ref() {
						match event {
							Event::End(TagEnd::Heading(_)) => break,
							_ => Self::_appendVisibleText(&mut headingText, event),
						}
					}
					let level = level as u8;
					self.headings.retain(|heading| heading.level < level);
					let ancestorContext = self._context(None)?;
					self.headings.push(Heading {
						level,
						text: headingText,
					});
					let markdown = self._structure(BlockKind::Heading, None);
					self._append(range.clone(), Some(markdown), &ancestorContext, false)?;
					proseStart = range.end;
				}
				Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
					self._prose(proseStart..range.start)?;
					let prefix = self._context(None)?;
					let markdown = self._structure(BlockKind::CodeBlock, None);
					self._append(range.clone(), Some(markdown), &prefix, true)?;
					proseStart = range.end;
				}
				Event::Start(Tag::Table(_)) => {
					let (tableStart, title) = self._tableTitle(proseStart, range.start);
					self._prose(proseStart..tableStart)?;
					let mut columns = Vec::new();
					let mut cellText = None;
					let mut rows = Vec::new();
					let mut isHeader = false;
					for (event, rowRange) in events.by_ref() {
						match event {
							Event::End(TagEnd::Table) => break,
							Event::Start(Tag::TableHead) => isHeader = true,
							Event::End(TagEnd::TableHead) => isHeader = false,
							Event::Start(Tag::TableCell) if isHeader == true => cellText = Some(String::new()),
							Event::End(TagEnd::TableCell) if isHeader == true => {
								columns.push(cellText.take().unwrap_or_default());
							}
							Event::Start(Tag::TableRow) if isHeader == false => rows.push(rowRange),
							event => {
								if let Some(cellText) = &mut cellText {
									Self::_appendVisibleText(cellText, event);
								}
							}
						}
					}
					self._table(tableStart..range.end, columns, title, &rows)?;
					proseStart = range.end;
				}
				_ => {}
			}
		}
		self._prose(proseStart..self.text.len())
	}

	/// Separates captions from confirmed pipe tables without changing any byte offsets.
	fn _separateTableCaptions(text: &str) -> String {
		let mut protectedRanges = Parser::new_ext(text, Options::ENABLE_TABLES)
			.into_offset_iter()
			.filter_map(|(event, range)| match event {
				Event::Start(Tag::Table(_) | Tag::CodeBlock(_)) => Some(range),
				_ => None,
			})
			.peekable();
		let mut markdownSource = String::with_capacity(text.len());
		let mut offset = 0;
		for line in text.split_inclusive('\n') {
			let lineStart = offset;
			offset += line.len();
			let content = line.trim_end_matches(['\r', '\n']);
			if Self::_caption(content).is_none() == true {
				markdownSource.push_str(line);
				continue;
			}
			while protectedRanges.peek().is_some_and(|range| range.end <= lineStart) {
				protectedRanges.next();
			}
			let isInsideBlock = protectedRanges.peek().is_some_and(|range| range.start < offset);
			if isInsideBlock == false && Self::_startsWithPipeTable(&text[offset..]) == true {
				markdownSource.extend(std::iter::repeat_n(' ', content.len()));
				markdownSource.push_str(&line[content.len()..]);
			} else {
				markdownSource.push_str(line);
			}
		}
		markdownSource
	}

	/// Returns the explicit caption in line when it is written at a Markdown block boundary.
	fn _caption(line: &str) -> Option<&str> {
		let content = line.trim_start_matches(' ');
		if line.len() - content.len() >= 4 {
			return None;
		}
		content
			.strip_prefix("Table: ")
			.map(str::trim)
			.filter(|title| title.is_empty() == false)
	}

	/// Checks whether Markdown recognizes the two lines following a caption as a table header and delimiter.
	fn _startsWithPipeTable(text: &str) -> bool {
		let headerEnd = text.find('\n').map_or(text.len(), |index| index + 1);
		let delimiterEnd = text[headerEnd..]
			.find('\n')
			.map_or(text.len(), |index| headerEnd + index + 1);
		Parser::new_ext(&text[..delimiterEnd], Options::ENABLE_TABLES)
			.next()
			.is_some_and(|event| matches!(event, Event::Start(Tag::Table(_))))
	}

	/// Appends readable content from an inline Markdown event.
	fn _appendVisibleText(text: &mut String, event: Event<'_>) {
		match event {
			Event::Text(value) | Event::Code(value) => text.push_str(&value),
			Event::SoftBreak | Event::HardBreak => text.push(' '),
			_ => {}
		}
	}

	/// Returns the preceding explicit title and its starting byte, or the table start.
	fn _tableTitle(&self, proseStart: usize, tableStart: usize) -> (usize, Option<String>) {
		let preceding = self.text[proseStart..tableStart].strip_suffix('\n').unwrap_or_default();
		let line = preceding.rsplit('\n').next().unwrap_or_default().trim_end_matches('\r');
		match Self::_caption(line) {
			Some(title) => (
				proseStart + preceding.len() - line.len() - usize::from(preceding.ends_with('\r')),
				Some(title.to_owned()),
			),
			None => (tableStart, None),
		}
	}

	/// Appends a whole table when it fits, otherwise groups rows and splits only oversized rows.
	fn _table(
		&mut self,
		range: Range<usize>,
		columns: Vec<String>,
		title: Option<String>,
		rows: &[Range<usize>],
	) -> Result<()> {
		let mut table = Table {
			columns,
			title,
			startRow: (rows.is_empty() == false).then_some(1),
			endRow: (rows.is_empty() == false).then_some(rows.len()),
		};
		let prefix = self._context(Some(&table))?;
		if rows.is_empty() == true || self._fits(range.clone(), &prefix)? == true {
			return self._append(
				range,
				Some(self._structure(BlockKind::Table, Some(table))),
				&prefix,
				true,
			);
		}

		let mut firstRow = 0;
		while firstRow < rows.len() {
			let mut lastRow = firstRow;
			while lastRow + 1 < rows.len() && self._fits(rows[firstRow].start..rows[lastRow + 1].end, &prefix)? == true
			{
				lastRow += 1;
			}
			table.startRow = Some(firstRow + 1);
			table.endRow = Some(lastRow + 1);
			let markdown = self._structure(BlockKind::Table, Some(table.clone()));
			self._append(rows[firstRow].start..rows[lastRow].end, Some(markdown), &prefix, false)?;
			firstRow = lastRow + 1;
		}
		Ok(())
	}

	/// Appends prose using the current heading path.
	fn _prose(&mut self, range: Range<usize>) -> Result<()> {
		if self.text[range.clone()].trim().is_empty() == true {
			return Ok(());
		}
		let prefix = self._context(None)?;
		self._append(range, Some(self._structure(BlockKind::Paragraph, None)), &prefix, false)
	}

	/// Returns whether the excerpt and prefix fit together.
	fn _fits(&self, range: Range<usize>, prefix: &str) -> Result<bool> {
		Ok(self
			.embeddingEngine
			.countTokens(&format!("{prefix}{}", &self.text[range]))?
			<= self.embeddingEngine.max_tokens())
	}

	/// Returns Markdown metadata for the current section and block.
	fn _structure(&self, blockKind: BlockKind, table: Option<Table>) -> Markdown {
		Markdown {
			headingPaths: (self.headings.is_empty() == false)
				.then(|| self.headings.clone())
				.into_iter()
				.collect(),
			blockKind,
			table,
		}
	}

	/// Returns up to half the model limit of heading, caption, and column context.
	fn _context(&self, table: Option<&Table>) -> Result<String> {
		let headings = self
			.headings
			.iter()
			.map(|heading| heading.text.as_str())
			.collect::<Vec<_>>()
			.join(" > ");
		let columns = table.map(|table| table.columns.join(" | ")).unwrap_or_default();
		let title = table.and_then(|table| table.title.as_deref()).unwrap_or_default();
		let fullContext = [headings.as_str(), title, columns.as_str()]
			.into_iter()
			.filter(|part| part.is_empty() == false)
			.collect::<Vec<_>>()
			.join("\n");
		let tokenBudget = self.embeddingEngine.max_tokens() / 2;
		if self.embeddingEngine.countTokens(&fullContext)? <= tokenBudget {
			return Ok(if fullContext.is_empty() {
				fullContext
			} else {
				format!("{fullContext}\n\n")
			});
		}

		let innerHeading = self
			.headings
			.last()
			.map(|heading| heading.text.as_str())
			.unwrap_or_default();
		let priorities = [innerHeading, columns.as_str()].map(str::trim);
		let count = priorities.iter().filter(|part| part.is_empty() == false).count().max(1);
		let mut context = String::new();
		for part in priorities.into_iter().filter(|part| part.is_empty() == false) {
			let boundaries = part
				.char_indices()
				.map(|(index, _)| index)
				.chain([part.len()])
				.collect::<Vec<_>>();
			let (mut low, mut high) = (0, boundaries.len() - 1);
			let budget = tokenBudget / count;
			while low < high {
				let middle = (low + high).div_ceil(2);
				if self.embeddingEngine.countTokens(&part[..boundaries[middle]])? <= budget {
					low = middle;
				} else {
					high = middle - 1;
				}
			}
			if low > 0 {
				context.push_str(&part[..boundaries[low]]);
				context.push('\n');
			}
		}
		while context.is_empty() == false && self.embeddingEngine.countTokens(&context)? > tokenBudget {
			context.pop();
		}
		Ok(context)
	}

	/// Splits a source range with the model tokenizer and records exact citation coordinates.
	fn _append(
		&mut self,
		range: Range<usize>,
		markdown: Option<Markdown>,
		prefix: &str,
		isLineBased: bool,
	) -> Result<()> {
		let source = &self.text[range.clone()];
		for piece in self.embeddingEngine.splitRetrievalText(source, prefix, isLineBased)? {
			let text = &source[piece.clone()];
			if text.trim().is_empty() == true {
				continue;
			}
			let startByte = range.start + piece.start;
			let endByte = range.start + piece.end;
			self.chunks.push(DocumentChunk {
				sourceChunk: SourceChunk {
					text: text.to_owned(),
					startByte,
					endByte,
					startLine: self.lineStarts.partition_point(|lineStart| *lineStart <= startByte),
					endLine: self.lineStarts.partition_point(|lineStart| *lineStart < endByte),
					markdown: markdown.clone(),
				},
				embeddingText: format!("{prefix}{text}"),
			});
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use localai_embeddings::EmbeddingConfig;

	/// Only an explicit caption directly before a confirmed table changes parser input.
	#[test]
	fn keepsCaptionLikeRowsAndProseIntact() {
		let table = "Name | City\n--- | ---\nNadia | Lima\nTable: Notes | Paris\nMarco | Rome\n";
		assert_eq!(DocumentChunks::_separateTableCaptions(table), table);
		let prose = "Table: Ordinary text\nThis | is still prose\nwithout a delimiter\n";
		assert_eq!(DocumentChunks::_separateTableCaptions(prose), prose);
		let fenced = "```\nTable: Example\nName | City\n--- | ---\n```\n";
		assert_eq!(DocumentChunks::_separateTableCaptions(fenced), fenced);

		let caption = "Table: European cities\n";
		let source = format!("{caption}{table}");
		let parsed = DocumentChunks::_separateTableCaptions(&source);
		assert_eq!(parsed.len(), source.len());
		assert_eq!(&parsed[caption.len()..], table);
		assert!(parsed[..caption.len()].trim().is_empty());
	}

	/// Verifies that the body after a heading stays whole when it fits with the heading context.
	#[test]
	#[ignore = "requires the cached default embedding model"]
	fn firstParagraphKeepsItsSourceRange() -> Result<()> {
		let embeddingEngine = EmbeddingEngine::load(EmbeddingConfig::default())?;
		let heading = "routing ".repeat(20);
		let paragraph = format!("{}\n", "river ".repeat(75).trim_end());
		let source = format!("# {}\n{paragraph}", heading.trim_end());
		let chunks = DocumentChunks::build(&source, &embeddingEngine, true)?;
		assert_eq!(chunks.len(), 2);
		assert_eq!(chunks[1].sourceChunk.text, paragraph);
		assert_eq!(chunks[1].sourceChunk.startLine, 2);
		assert_eq!(chunks[1].sourceChunk.startByte, source.len() - paragraph.len());
		assert!(embeddingEngine.countTokens(&chunks[1].embeddingText)? <= embeddingEngine.max_tokens());
		Ok(())
	}
}
