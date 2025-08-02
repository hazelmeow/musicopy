use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// A counter that can be updated across the FFI boundary.
///
/// A CounterModel wraps a shared reference to an atomic u64, which can be
/// updated in the core and cheaply read repeatedly in the UI for fast updates.
#[derive(Debug, uniffi::Object)]
pub struct CounterModel(Arc<AtomicU64>);

#[uniffi::export]
impl CounterModel {
    #[uniffi::constructor]
    pub fn new(n: u64) -> Self {
        Self(Arc::new(AtomicU64::new(n)))
    }

    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl From<Arc<AtomicU64>> for CounterModel {
    fn from(counter: Arc<AtomicU64>) -> Self {
        Self(counter)
    }
}

impl From<&Arc<AtomicU64>> for CounterModel {
    fn from(counter: &Arc<AtomicU64>) -> Self {
        Self(counter.clone())
    }
}
