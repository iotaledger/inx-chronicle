use thiserror::Error;

/// The different errors that can happen with INX.
// TODO: Maybe this should be called `ConversionError`?
// TODO: Consider splitting up this error.
#[derive(Debug, Error)]
#[allow(missing_docs)]
pub enum InxError {
    #[error("expected {expected} bytes but received {actual}")]
    InvalidByteLength { actual: usize, expected: usize },
    #[error("{0}")]
    InvalidRawBytes(String),
    #[error("missing field: {0}")]
    MissingField(&'static str),
    #[error("gRPC status code: {0}")]
    StatusCode(#[from] tonic::Status),
    #[error(transparent)]
    TonicError(#[from] tonic::transport::Error),
}
