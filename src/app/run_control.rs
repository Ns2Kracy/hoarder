use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::core::types::JobId;

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
pub struct JobRunRegistry {
    running: Mutex<HashMap<JobId, CancellationToken>>,
}

impl JobRunRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a running job and return its cancellation token.
    ///
    /// # Panics
    ///
    /// Panics if the registry mutex is poisoned.
    pub fn register(&self, job_id: JobId) -> CancellationToken {
        let token = CancellationToken::new();
        self.running.lock().unwrap().insert(job_id, token.clone());
        token
    }

    /// Remove a job from the running registry.
    ///
    /// # Panics
    ///
    /// Panics if the registry mutex is poisoned.
    pub fn unregister(&self, job_id: JobId) {
        self.running.lock().unwrap().remove(&job_id);
    }

    /// Signal cancellation for a running job.
    ///
    /// # Panics
    ///
    /// Panics if the registry mutex is poisoned.
    pub fn cancel(&self, job_id: JobId) -> bool {
        let token = self.running.lock().unwrap().get(&job_id).cloned();
        token.is_some_and(|token| {
            token.cancel();
            true
        })
    }
}
