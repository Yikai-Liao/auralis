//! Parser for SoX-ng-inspired effects files.
//!
//! Effects files are a text representation of the same positional command
//! tokens accepted by [`crate::parse_effect_chain`]. Each non-empty,
//! non-comment line may contain one or more implemented effect commands, and
//! `:` preserves an explicit chain boundary. Tokens are separated by
//! whitespace; single and double quotes group whitespace into one token; a
//! backslash escapes the next character outside single quotes; and `#` starts a
//! comment when it appears outside quotes. Successful parses return a typed
//! [`crate::EffectChain`], not long-lived string commands.
//!
//! # Examples
//!
//! ```
//! use auralis_effects::parse_effects_file_str;
//!
//! let chain = parse_effects_file_str(
//!     r#"
//!     gain -3
//!     dcshift 0.125 reverse
//!     "#,
//! )?;
//!
//! assert_eq!(chain.len(), 3);
//! assert_eq!(chain.commands()[0].render_tokens(), ["gain", "-3"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{
    EffectChain, EffectChainBoundary, EffectCommandParseError, EffectRegistry,
    chain::{is_chain_boundary_token, is_unsupported_boundary_control},
    chain_dispatch::command_end,
    parse_effect_command,
};

/// Crate-local result type for effects-file parsing.
pub type EffectsFileParseResult<T> = std::result::Result<T, EffectsFileParseError>;

/// Crate-local result type for reading and parsing effects files from disk.
pub type EffectsFileReadResult<T> = std::result::Result<T, EffectsFileReadError>;

/// Reads and parses an effects file from disk.
///
/// The file must be valid UTF-8 text using the syntax documented on
/// [`parse_effects_file_str`]. The returned chain contains typed effect
/// commands in file order. Empty files and files containing only blank lines or
/// comments parse to an empty chain; callers that need to reject empty chains
/// can check [`EffectChain::is_empty`].
///
/// # Errors
///
/// Returns [`EffectsFileReadError::Read`] when the file cannot be read as text,
/// or [`EffectsFileReadError::Parse`] when the file contains malformed tokens
/// or an unsupported/invalid effect command.
pub fn parse_effects_file(path: impl AsRef<Path>) -> EffectsFileReadResult<EffectChain> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|source| EffectsFileReadError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    parse_effects_file_str(&source).map_err(EffectsFileReadError::Parse)
}

/// Parses an effects file from a string.
///
/// Each non-empty, non-comment line may contain one or more positional effect
/// commands. `:` separates and preserves explicit chain segments. The parser
/// uses the same command-boundary rules as
/// [`crate::parse_effect_chain`], so a file such as:
///
/// ```text
/// gain -3
/// dcshift 0.125 : reverse
/// ```
///
/// produces the same typed chain as the flat CLI token stream `gain -3 dcshift
/// 0.125 : reverse`. A `#` outside quotes starts a comment. Single and
/// double quotes are removed after grouping token text, and a backslash escapes
/// the next character outside single quotes.
///
/// # Errors
///
/// Returns [`EffectsFileParseError`] with one-based line and column positions
/// when a token is malformed, an effect name is unknown or unsupported, an
/// effect command rejects its arguments, a boundary would create an empty
/// segment, or a file uses the not-yet-implemented `newfile` or `restart`
/// boundary controls.
pub fn parse_effects_file_str(source: &str) -> EffectsFileParseResult<EffectChain> {
    let mut tokens = Vec::new();

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_index + 1;
        tokens.extend(tokenize_line(line, line_number)?);
    }

    parse_file_commands(&tokens)
}

/// Errors produced while reading an effects file from disk.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EffectsFileReadError {
    /// The file could not be read as UTF-8 text.
    #[error("failed to read effects file `{}`: {source}", path.display())]
    Read {
        /// File path that failed to read.
        path: PathBuf,

        /// Source I/O error.
        #[source]
        source: io::Error,
    },

    /// The file was read but could not be parsed.
    #[error(transparent)]
    Parse(#[from] EffectsFileParseError),
}

/// Errors produced while parsing effects-file text.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum EffectsFileParseError {
    /// A single-quoted token reached the end of its line before closing.
    #[error("effects file line {line}, column {column}: unterminated single quote")]
    UnterminatedSingleQuote {
        /// One-based source line.
        line: usize,

        /// One-based source column.
        column: usize,
    },

    /// A double-quoted token reached the end of its line before closing.
    #[error("effects file line {line}, column {column}: unterminated double quote")]
    UnterminatedDoubleQuote {
        /// One-based source line.
        line: usize,

        /// One-based source column.
        column: usize,
    },

    /// A backslash appeared without a following character to escape.
    #[error("effects file line {line}, column {column}: dangling escape")]
    DanglingEscape {
        /// One-based source line.
        line: usize,

        /// One-based source column.
        column: usize,
    },

    /// One command in the file could not be parsed into a typed effect command.
    #[error(
        "effects file line {line}, column {column}: command `{command}` failed to parse: {source}"
    )]
    CommandParseFailed {
        /// One-based source line.
        line: usize,

        /// One-based source column.
        column: usize,

        /// Raw command tokens that were being parsed.
        command: String,

        /// Typed command parser source error.
        #[source]
        source: EffectCommandParseError,
    },

    /// A `:` boundary would create an empty chain segment.
    #[error("effects file line {line}, column {column}: chain boundary creates an empty segment")]
    EmptyBoundaryChain {
        /// One-based source line.
        line: usize,

        /// One-based source column.
        column: usize,
    },

    /// A SoX-ng boundary control token is known but not implemented yet.
    #[error(
        "effects file line {line}, column {column}: unsupported boundary control `{control}`; `newfile` and `restart` semantics are not implemented"
    )]
    UnsupportedBoundaryControl {
        /// One-based source line.
        line: usize,

        /// One-based source column.
        column: usize,

        /// Unsupported boundary control token.
        control: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    value: String,
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quote {
    Single { opening_column: usize },
    Double { opening_column: usize },
}

fn parse_file_commands(tokens: &[Token]) -> EffectsFileParseResult<EffectChain> {
    let token_values: Vec<&str> = tokens.iter().map(|token| token.value.as_str()).collect();
    let mut commands = Vec::new();
    let mut boundaries = Vec::new();
    let mut offset = 0;
    let mut pending_boundary_token: Option<&Token> = None;

    while offset < tokens.len() {
        let start = &tokens[offset];
        if is_chain_boundary_token(&start.value) {
            if commands.is_empty() || pending_boundary_token.is_some() {
                return Err(empty_boundary_chain(start));
            }
            boundaries.push(EffectChainBoundary::new(commands.len()));
            pending_boundary_token = Some(start);
            offset += 1;
            continue;
        }
        if is_unsupported_boundary_control(&start.value) {
            return Err(EffectsFileParseError::UnsupportedBoundaryControl {
                line: start.line,
                column: start.column,
                control: start.value.clone(),
            });
        }

        let descriptor = EffectRegistry::resolve(&start.value).map_err(|source| {
            command_parse_failed(
                start,
                start.value.clone(),
                EffectCommandParseError::from(source),
            )
        })?;
        let end = command_end(descriptor.kind(), &token_values, offset);
        let command_tokens = &tokens[offset..end];
        let command_values: Vec<&str> = command_tokens
            .iter()
            .map(|token| token.value.as_str())
            .collect();
        let command_text = command_tokens
            .iter()
            .map(|token| token.value.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        let command = parse_effect_command(&command_values).map_err(|source| {
            command_parse_failed(&tokens[offset], command_text.clone(), source)
        })?;

        commands.push(command);
        pending_boundary_token = None;
        offset = end;
    }

    if let Some(token) = pending_boundary_token {
        return Err(empty_boundary_chain(token));
    }

    Ok(EffectChain::from_parts(commands, boundaries))
}

fn command_parse_failed(
    token: &Token,
    command: String,
    source: EffectCommandParseError,
) -> EffectsFileParseError {
    EffectsFileParseError::CommandParseFailed {
        line: token.line,
        column: token.column,
        command,
        source,
    }
}

fn empty_boundary_chain(token: &Token) -> EffectsFileParseError {
    EffectsFileParseError::EmptyBoundaryChain {
        line: token.line,
        column: token.column,
    }
}

fn tokenize_line(line: &str, line_number: usize) -> EffectsFileParseResult<Vec<Token>> {
    let chars: Vec<(usize, char)> = line
        .chars()
        .enumerate()
        .map(|(index, character)| (index + 1, character))
        .collect();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        while index < chars.len() && chars[index].1.is_whitespace() {
            index += 1;
        }

        if index == chars.len() || chars[index].1 == '#' {
            break;
        }

        let (token, next_index) = tokenize_from(&chars, index, line_number)?;
        tokens.push(token);
        index = next_index;

        if index < chars.len() && chars[index].1 == '#' {
            break;
        }
    }

    Ok(tokens)
}

fn tokenize_from(
    chars: &[(usize, char)],
    start_index: usize,
    line_number: usize,
) -> EffectsFileParseResult<(Token, usize)> {
    let token_start_column = chars[start_index].0;
    let mut token = String::new();
    let mut quote = None;
    let mut index = start_index;

    while index < chars.len() {
        let (column, character) = chars[index];

        match quote {
            Some(Quote::Single { .. }) => {
                if character == '\'' {
                    quote = None;
                } else {
                    token.push(character);
                }
                index += 1;
            }
            Some(Quote::Double { .. }) => {
                index = push_double_quoted_character(
                    chars,
                    index,
                    line_number,
                    &mut token,
                    &mut quote,
                )?;
            }
            None if character.is_whitespace() || character == '#' => break,
            None if character == '\'' => {
                quote = Some(Quote::Single {
                    opening_column: column,
                });
                index += 1;
            }
            None if character == '"' => {
                quote = Some(Quote::Double {
                    opening_column: column,
                });
                index += 1;
            }
            None if character == '\\' => {
                index += 1;
                let Some((_, escaped)) = chars.get(index).copied() else {
                    return Err(EffectsFileParseError::DanglingEscape {
                        line: line_number,
                        column,
                    });
                };
                token.push(escaped);
                index += 1;
            }
            None => {
                token.push(character);
                index += 1;
            }
        }
    }

    reject_unterminated_quote(quote, line_number)?;

    Ok((
        Token {
            value: token,
            line: line_number,
            column: token_start_column,
        },
        index,
    ))
}

fn push_double_quoted_character(
    chars: &[(usize, char)],
    index: usize,
    line_number: usize,
    token: &mut String,
    quote: &mut Option<Quote>,
) -> EffectsFileParseResult<usize> {
    let (column, character) = chars[index];

    if character == '"' {
        *quote = None;
        Ok(index + 1)
    } else if character == '\\' {
        let escape_index = index + 1;
        let Some((_, escaped)) = chars.get(escape_index).copied() else {
            return Err(EffectsFileParseError::DanglingEscape {
                line: line_number,
                column,
            });
        };
        token.push(escaped);
        Ok(escape_index + 1)
    } else {
        token.push(character);
        Ok(index + 1)
    }
}

fn reject_unterminated_quote(
    quote: Option<Quote>,
    line_number: usize,
) -> EffectsFileParseResult<()> {
    match quote {
        Some(Quote::Single { opening_column }) => {
            Err(EffectsFileParseError::UnterminatedSingleQuote {
                line: line_number,
                column: opening_column,
            })
        }
        Some(Quote::Double { opening_column }) => {
            Err(EffectsFileParseError::UnterminatedDoubleQuote {
                line: line_number,
                column: opening_column,
            })
        }
        None => Ok(()),
    }
}
#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use super::{EffectsFileParseError, parse_effects_file, parse_effects_file_str, tokenize_line};
    use crate::{EffectCommand, EffectCommandParseError, parse_effect_chain};

    #[test]
    fn reads_effects_from_text_file() {
        let path = temp_path("auralis-effects-file-parser", "effects");
        fs::write(
            &path,
            "
            # one effect per line is the common form
            gain -3
            dcshift 0.125 # comments may follow commands
            fade t 2
            reverse
            ",
        )
        .unwrap();

        let chain = parse_effects_file(&path).unwrap();

        let rendered: Vec<Vec<String>> = chain
            .commands()
            .iter()
            .map(EffectCommand::render_tokens)
            .collect();
        assert_eq!(
            rendered,
            [
                ["gain", "-3"].map(String::from).to_vec(),
                ["dcshift", "0.125"].map(String::from).to_vec(),
                ["fade", "t", "2"].map(String::from).to_vec(),
                ["reverse"].map(String::from).to_vec(),
            ]
        );

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn ignores_blank_lines_and_documented_comments() {
        let chain = parse_effects_file_str(
            "
            # whole-line comment

            gain 0 # inline comment

            \t# tab-indented comment
            ",
        )
        .unwrap();

        assert_eq!(chain.len(), 1);
        assert_eq!(chain.commands()[0].render_tokens(), ["gain", "0"]);
    }

    #[test]
    fn parsed_effects_match_equivalent_cli_args() {
        let file_chain = parse_effects_file_str(
            "
            gain -3
            dcshift 0.125 reverse
            fade t 2
            ",
        )
        .unwrap();
        let cli_chain = parse_effect_chain(&[
            "gain", "-3", "dcshift", "0.125", "reverse", "fade", "t", "2",
        ])
        .unwrap();

        assert_eq!(file_chain, cli_chain);
    }

    #[test]
    fn boundary_syntax_is_preserved_across_effects_file_lines() {
        let chain = parse_effects_file_str(
            "
            gain -3 :
            dcshift 0.125 reverse
            ",
        )
        .unwrap();

        assert!(chain.has_boundaries());
        assert_eq!(chain.boundaries()[0].before_command(), 1);
        assert_eq!(
            chain.render_tokens(),
            ["gain", "-3", ":", "dcshift", "0.125", "reverse"]
        );
    }

    #[test]
    fn quoted_and_escaped_tokens_follow_sox_style_rules() {
        let tokens =
            tokenize_line(r#"gain "-3" dcshift '0.125' reverse\#literal # comment"#, 4).unwrap();
        let values: Vec<&str> = tokens.iter().map(|token| token.value.as_str()).collect();
        let columns: Vec<usize> = tokens.iter().map(|token| token.column).collect();

        assert_eq!(
            values,
            ["gain", "-3", "dcshift", "0.125", "reverse#literal"]
        );
        assert_eq!(columns, [1, 6, 11, 19, 27]);
    }

    #[test]
    fn malformed_quotes_report_line_and_column() {
        let error = parse_effects_file_str("gain \"-3").unwrap_err();

        assert_eq!(
            error,
            EffectsFileParseError::UnterminatedDoubleQuote { line: 1, column: 6 }
        );
        assert_eq!(
            error.to_string(),
            "effects file line 1, column 6: unterminated double quote"
        );

        let error = parse_effects_file_str("gain '-3").unwrap_err();

        assert_eq!(
            error,
            EffectsFileParseError::UnterminatedSingleQuote { line: 1, column: 6 }
        );
    }

    #[test]
    fn dangling_escape_reports_line_and_column() {
        let error = parse_effects_file_str("gain \\").unwrap_err();

        assert_eq!(
            error,
            EffectsFileParseError::DanglingEscape { line: 1, column: 6 }
        );
        assert_eq!(
            error.to_string(),
            "effects file line 1, column 6: dangling escape"
        );
    }

    #[test]
    fn command_parse_failures_report_command_start_line_and_column() {
        let error = parse_effects_file_str("gain -3 trim reverse").unwrap_err();

        assert_eq!(
            error,
            EffectsFileParseError::CommandParseFailed {
                line: 1,
                column: 9,
                command: "trim".to_owned(),
                source: EffectCommandParseError::MissingArgument {
                    effect: "trim",
                    argument: "position",
                },
            }
        );
        assert_eq!(
            error.to_string(),
            "effects file line 1, column 9: command `trim` failed to parse: effect `trim` requires argument `position`"
        );
    }

    #[test]
    fn empty_boundary_segments_report_line_and_column() {
        let error = parse_effects_file_str("gain -3\n:").unwrap_err();

        assert_eq!(
            error,
            EffectsFileParseError::EmptyBoundaryChain { line: 2, column: 1 }
        );
        assert_eq!(
            error.to_string(),
            "effects file line 2, column 1: chain boundary creates an empty segment"
        );

        let error = parse_effects_file_str("gain -3 :\n# comment").unwrap_err();
        assert_eq!(
            error,
            EffectsFileParseError::EmptyBoundaryChain { line: 1, column: 9 }
        );
    }

    #[test]
    fn unsupported_boundary_controls_report_line_and_column() {
        let error = parse_effects_file_str("gain -3 : newfile").unwrap_err();

        assert_eq!(
            error,
            EffectsFileParseError::UnsupportedBoundaryControl {
                line: 1,
                column: 11,
                control: "newfile".to_owned(),
            }
        );
        assert_eq!(
            error.to_string(),
            "effects file line 1, column 11: unsupported boundary control `newfile`; `newfile` and `restart` semantics are not implemented"
        );

        let error = parse_effects_file_str("gain -3\nrestart").unwrap_err();
        assert_eq!(
            error,
            EffectsFileParseError::UnsupportedBoundaryControl {
                line: 2,
                column: 1,
                control: "restart".to_owned(),
            }
        );
    }

    #[test]
    fn unsupported_effect_names_are_reported_at_file_position() {
        let error = parse_effects_file_str("gain -3\nbandreject 100 200").unwrap_err();

        assert_eq!(
            error.to_string(),
            "effects file line 2, column 1: command `bandreject` failed to parse: known SoX-ng effect `bandreject` is not implemented by Auralis; missing SoX-ng coverage entry for `bandreject`"
        );
    }

    #[test]
    fn comments_only_parse_to_empty_chain() {
        let chain = parse_effects_file_str(
            "
            # no effects yet

            ",
        )
        .unwrap();

        assert!(chain.is_empty());
    }

    fn temp_path(prefix: &str, extension: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        std::env::temp_dir().join(format!("{prefix}-{nanos}.{extension}"))
    }
}
