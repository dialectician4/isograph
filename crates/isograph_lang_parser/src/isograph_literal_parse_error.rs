use common_lang_types::{ScalarFieldName, WithLocation, WithSpan};
use thiserror::Error;

use super::peekable_lexer::LowLevelParseError;

pub(crate) type ParseResultWithLocation<T> = Result<T, WithLocation<IsographLiteralParseError>>;
pub(crate) type ParseResultWithSpan<T> = Result<T, WithSpan<IsographLiteralParseError>>;

/// Errors tha make semantic sense when referring to parsing a Isograph literal
#[derive(Error, Debug)]
pub enum IsographLiteralParseError {
    #[error("{error}")]
    ParseError { error: LowLevelParseError },

    #[error("Expected a type (e.g. String, [String], or String!)")]
    ExpectedTypeAnnotation,

    #[error("Unparsed tokens remaining")]
    LeftoverTokens,

    #[error("Isograph literals must be immediately called, and passed a function")]
    ExpectedAssociatedJsFunction,

    #[error("Isograph literals must start with field or entrypoint")]
    ExpectedFieldOrEntrypoint,

    #[error(
        "This isograph field literal must be exported as a named export, for example \
        as `export const {suggested_const_export_name}`"
    )]
    ExpectedLiteralToBeExported {
        suggested_const_export_name: ScalarFieldName,
    },

    #[error("Expected a valid value, like $foo or 42")]
    ExpectedNonConstantValue,

    #[error("Descriptions are currently disallowed")]
    DescriptionsAreDisallowed,

    #[error("Expected a comma or linebreak")]
    ExpectedCommaOrLineBreak,

    #[error(
        "Selection sets are required. If you do not want to \
        select any fields, write an empty selection set: {{}}"
    )]
    ExpectedSelectionSet,

    #[error(
        "You must call the iso function with parentheses. \"iso`...`\" is \
        not supported."
    )]
    ExpectedParenthesesAroundIsoLiteral,
}

impl From<LowLevelParseError> for IsographLiteralParseError {
    fn from(error: LowLevelParseError) -> Self {
        IsographLiteralParseError::ParseError { error }
    }
}
