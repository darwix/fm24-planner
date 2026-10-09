#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub code: String,
    pub message: String,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    pub fn query(message: impl Into<String>) -> Self {
        Self::new("INVALID_QUERY", message)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new("IO", e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
