use thiserror::Error;

use std::io::Error as IoError;

/// Errors that may arise during ANTEX file parsing process.
#[derive(Debug, Error)]
pub enum ParsingError {}
