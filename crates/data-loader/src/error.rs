//! Error types for the data-loader crate.
//!
//! Rust error handling concepts demonstrated:
//! - thiserror for defining custom error types
//! - Enum variants for different error cases
//! - Error messages with context
//! - Automatic `Display` and `Error` trait implementations

use thiserror::Error;

/// Errors that can occur during data loading and parsing
///
/// Rust concept: Using an enum for errors lets us handle different cases
/// The `#[derive(Error)]` macro from thiserror automatically implements
/// the `std::error::Error` trait and `Display` based on our `#[error(...)]` attributes
#[derive(Error, Debug)]
pub enum DataLoadError {
    /// File could not be found or opened
    #[error("Failed to open file: {path}")]
    FileNotFound { path: String },

    /// I/O error occurred while reading file
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Line in data file couldn't be parsed
    ///
    /// This variant stores context about where the error occurred
    #[error("Parse error at line {line} in {file}: {reason}")]
    ParseError {
        file: String,
        line: usize,
        reason: String,
    },

    /// A data field had an invalid value
    #[error("Invalid value for {field}: {value}")]
    InvalidValue { field: String, value: String },

    /// Expected number of fields in a line doesn't match actual
    #[error("Expected {expected} fields but found {found} in line {line}")]
    FieldCountMismatch {
        expected: usize,
        found: usize,
        line: usize,
    },

    /// Referenced entity doesn't exist (e.g., rating for non-existent movie)
    #[error("Missing reference: {entity} with id {id}")]
    MissingReference { entity: String, id: u32 },

    /// Data validation failed
    #[error("Validation failed: {0}")]
    ValidationError(String),
}

/// Convenience type alias for Results in this crate
///
/// Rust concept: Type aliases make code more readable
/// Instead of writing `Result<T, DataLoadError>` everywhere,
/// we can write `Result<T>`
pub type Result<T> = std::result::Result<T, DataLoadError>;
