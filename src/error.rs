use core::fmt;
use core::str::Utf8Error;

/// Errors that can occur during CheetahString operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// UTF-8 validation failed
    Utf8Error(Utf8Error),
    /// Index out of bounds
    IndexOutOfBounds { index: usize, len: usize },
    /// Invalid character boundary
    InvalidCharBoundary { index: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
            Error::IndexOutOfBounds { index, len } => {
                write!(f, "index {} out of bounds (len: {})", index, len)
            }
            Error::InvalidCharBoundary { index } => {
                write!(f, "index {} is not a char boundary", index)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Utf8Error(e) => Some(e),
            _ => None,
        }
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::Utf8Error(e)
    }
}

/// Result type for CheetahString operations
pub type Result<T> = core::result::Result<T, Error>;
