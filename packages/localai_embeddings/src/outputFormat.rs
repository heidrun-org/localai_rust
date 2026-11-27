//! Formats command results for terminal, Markdown, and JSON consumers.

#![allow(non_snake_case)]

use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;
use serde_json::Value;

/// Format of a command result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum OutputFormat {
	/// Human-readable output, with color on a terminal.
	Text,
	/// A Markdown list of the result fields.
	Markdown,
	/// One JSON document.
	Json,
}

impl OutputFormat {
	/// Renders value with terminal color when isTerminal is true and the format is text.
	/// Returns the rendered result or a serialization error.
	pub(crate) fn render(self, value: &impl Serialize, isTerminal: bool) -> Result<String> {
		if self == Self::Json {
			return Ok(serde_json::to_string_pretty(value)? + "\n");
		}
		let mut text = String::new();
		self._append(&mut text, &serde_json::to_value(value)?, 0, None, isTerminal);
		Ok(text)
	}

	/// Appends value under an optional label at the given depth. Adds color only for terminal text.
	fn _append(self, text: &mut String, value: &Value, depth: usize, label: Option<&str>, isTerminal: bool) {
		use std::fmt::Write as _;
		let isMarkdown = self == Self::Markdown;
		let isContainer = match value {
			Value::Object(fields) => fields.is_empty() == false,
			Value::Array(items) => items.is_empty() == false,
			_ => false,
		};
		if label.is_some() || isContainer == false {
			let indent = " ".repeat(depth * if isMarkdown { 4 } else { 2 });
			text.push_str(&indent);
			if isMarkdown == true {
				text.push_str("- ");
			}
			if let Some(label) = label {
				if isMarkdown == true {
					let _ = write!(text, "**{}:**", Self::_escapeMarkdown(label));
				} else if isTerminal == true {
					let label = Self::_escapeControls(label);
					let _ = write!(text, "\x1b[1;36m{label}:\x1b[0m");
				} else {
					let _ = write!(text, "{}:", Self::_escapeControls(label));
				}
				if isContainer == false {
					text.push(' ');
				}
			}
			if isContainer == false {
				let scalar = value.to_string();
				if isMarkdown == true {
					text.push_str(&Self::_escapeMarkdown(&scalar));
				} else if isTerminal == true && label.is_none() {
					let _ = write!(text, "\x1b[36m{scalar}\x1b[0m");
				} else {
					text.push_str(&scalar);
				}
			}
			text.push('\n');
		}
		let childDepth = depth + usize::from(label.is_some());
		match value {
			Value::Object(fields) => {
				for (name, field) in fields {
					self._append(text, field, childDepth, Some(name), isTerminal);
				}
			}
			Value::Array(items) => {
				for (index, item) in items.iter().enumerate() {
					let label = (index + 1).to_string();
					self._append(text, item, childDepth, Some(&label), isTerminal);
				}
			}
			_ => {}
		}
	}

	/// Escapes control characters in a field name and returns the printable name.
	fn _escapeControls(text: &str) -> String {
		let mut escaped = String::new();
		for character in text.chars() {
			if character.is_control() == true {
				escaped.extend(character.escape_default());
			} else {
				escaped.push(character);
			}
		}
		escaped
	}

	/// Escapes Markdown punctuation in text and returns the escaped text.
	fn _escapeMarkdown(text: &str) -> String {
		use std::fmt::Write as _;
		let mut escaped = String::new();
		for character in text.chars() {
			if matches!(character, '&' | '<' | '>' | '*' | '_' | '[' | ']' | '`' | '~' | '\\')
				|| character.is_control()
			{
				let _ = write!(escaped, "&#{};", u32::from(character));
			} else {
				escaped.push(character);
			}
		}
		escaped
	}
}
