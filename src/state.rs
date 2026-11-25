use arc_swap::ArcSwap;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// Lock-free state owned by the renderer.
pub struct State<T: 'static> {
    published: Arc<ArcSwap<T>>,
}

/// Mutable state handle owned by the widget handler.
pub struct StateGuard<T: Clone + 'static> {
    published: Arc<ArcSwap<T>>,
    working: T,
}

impl<T: Clone + 'static> State<T> {
    /// Creates a new state pair for the renderer (`State`) and widget handler (`StateGuard`).
    pub fn new(initial: T) -> (Self, StateGuard<T>) {
        let published = Arc::new(ArcSwap::from_pointee(initial.clone()));
        let state = Self {
            published: Arc::clone(&published),
        };
        let guard = StateGuard {
            published,
            working: initial,
        };
        (state, guard)
    }

    /// Returns a cloned `Arc<T>` of the latest published snapshot (lock-free).
    pub fn get(&self) -> Arc<T> {
        self.published.load_full()
    }
}

impl<T: Clone + 'static> StateGuard<T> {
    /// Publishes a clone of the widget's working state into the atomic snapshot.
    pub fn sync(&mut self) {
        self.published.store(Arc::new(self.working.clone()));
    }
}

impl<T: Clone + 'static> Deref for StateGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.working
    }
}

impl<T: Clone + 'static> DerefMut for StateGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.working
    }
}
