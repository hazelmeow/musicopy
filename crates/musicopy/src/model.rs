//! The model is the serialized state of the application sent to the UI.
//!
//! The model should be available immediately (synchronously queryable by the
//! application during initialization), with updated snapshots pushed to the UI
//! when data changes. On the Rust side, one model is kept around and mutated,
//! then serialized in its entirety and sent to the UI. Sending via FFI always
//! requires serialization, so when possible we should slice the model into
//! subtrees and send snapshots independently.

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
