use emacs::{defun, Result, Value, IntoLisp, ResultExt, Vector};
use emacs::failure;

use tree_sitter::{Parser, Point};

use crate::types::{SharedTree, shared, Language, Range};

/// Create a new parser.
#[defun(user_ptr)]
fn make_parser() -> Result<Parser> {
    Ok(Parser::new())
}

/// Set the LANGUAGE that PARSER should use for parsing.
///
/// This may fail if there was a version mismatch: the loaded LANGUAGE was generated
/// with an incompatible version of tree-sitter-cli.
#[defun]
fn set_language(parser: &mut Parser, language: Language) -> Result<()> {
    parser.set_language(language.into()).map_err(failure::err_msg);
    Ok(())
}

// TODO: Add a version that reuses a single byte buffer to avoid multiple allocations. Also allow
// `parse` to pass a soft size limit to the input function.

// TODO: Add parse_buffer.

/// Use PARSER to parse source code generated by INPUT-FUNCTION, returning a tree.
///
/// INPUT-FUNCTION should take 3 parameters: (BYTE-OFFSET ROW COLUMN), and return a
/// fragment of the source code, starting from the position identified by either
/// BYTE-OFFSET or [ROW COLUMN].
///
/// If you have already parsed an earlier version of this document, and it has since
/// been edited, pass the previously parsed OLD-TREE so that its unchanged parts can
/// be reused. This will save time and memory. For this to work correctly, you must
/// have already edited it using `ts-edit-tree' function in a way that exactly
/// matches the source code changes.
///
/// Note that indexing is assumed to be zero-based, while Emacs normally uses
/// one-based indexing for accessing buffer content.
#[defun(user_ptr(direct))]
fn parse(parser: &mut Parser, input_function: Value, old_tree: Option<&SharedTree>) -> Result<SharedTree> {
    let env = input_function.env;
    let old_tree = match old_tree {
        Some(v) => Some(v.try_borrow()?),
        _ => None,
    };
    let old_tree = match &old_tree {
        Some(r) => Some(&**r),
        _ => None,
    };
    let input = |byte: usize, position: Point| -> String {
        let fragment = env
            .call(
                "funcall",
                &[
                    input_function,
                    byte.into_lisp(env).unwrap_or_propagate(),
                    position.row.into_lisp(env).unwrap_or_propagate(),
                    position.column.into_lisp(env).unwrap_or_propagate(),
                ],
            )
            .unwrap_or_propagate();
        fragment.into_rust::<String>().unwrap_or_propagate()
    };
    // TODO: Support error cases (None).
    let tree = parser.parse_buffering_with(input, old_tree).unwrap();
    Ok(shared(tree))
}

/// Use PARSER to parse the INPUT string, returning a tree.
#[defun]
fn parse_string(parser: &mut Parser, input: String) -> Result<SharedTree> {
    let tree = parser.parse(input, None).unwrap();
    Ok(shared(tree))
}

/// Instruct PARSER to start the next parse from the beginning.
///
/// If PARSER previously failed because of a timeout or a cancellation, then by
/// default, it will resume where it left off on the next parse. If you don't want
/// to resume, and instead intend to use PARSER to parse some other code, you must
/// call this function first.
///
/// Note: timeout and cancellation are not yet properly supported.
#[defun]
fn _reset_parser(parser: &mut Parser) -> Result<()> {
    Ok(parser.reset())
}

/// Return the duration in microseconds that PARSER is allowed to take each parse.
/// Note: timeout and cancellation are not yet properly supported.
#[defun]
fn _timeout_micros(parser: &mut Parser) -> Result<u64> {
    Ok(parser.timeout_micros())
}

/// Set MAX-DURATION in microseconds that PARSER is allowed to take each parse.
/// Note: timeout and cancellation are not yet properly supported.
#[defun]
fn _set_timeout_micros(parser: &mut Parser, max_duration: u64) -> Result<()> {
    Ok(parser.set_timeout_micros(max_duration))
}

/// Set the RANGES of text that PARSER should include when parsing.
///
/// By default, PARSER will always include entire documents. This function allows
/// you to parse only a portion of a document but still return a syntax tree whose
/// ranges match up with the document as a whole. RANGES should be a vector, and can
/// be disjointed.
///
/// This is useful for parsing multi-language documents.
#[defun]
fn set_included_ranges(parser: &mut Parser, ranges: Value) -> Result<()> {
    let ranges = Vector(ranges);
    let len = ranges.size()?;
    let included = &mut Vec::with_capacity(len);
    for i in 0..len {
        let range: Range = ranges.get(i)?;
        included.push(range.into());
    }
    Ok(parser.set_included_ranges(included))
}
