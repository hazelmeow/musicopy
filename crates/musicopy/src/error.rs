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

/// Creates a CoreError by wrapping anyhow::anyhow!.
macro_rules! core_error {
    ($msg:literal $(,)?) => {
        CoreError::from(anyhow::anyhow!($msg))
    };
    ($err:expr $(,)?) => {
        CoreError::from(anyhow::anyhow!($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        CoreError::from(anyhow::anyhow!($fmt, $($arg)*))
    };
}
pub(crate) use core_error;
