/// Error type for FFI.
#[derive(Debug, thiserror::Error, uniffi::Object)]
#[error("{e:?}")]
pub struct CoreError {
    e: anyhow::Error,
}

#[uniffi::export]
impl CoreError {
    fn message(&self) -> String {
        self.to_string()
    }
}

impl From<anyhow::Error> for CoreError {
    fn from(e: anyhow::Error) -> Self {
        Self { e }
    }
}
