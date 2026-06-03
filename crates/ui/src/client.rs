use {async_trait::async_trait, std::sync::OnceLock};

#[async_trait(?Send)]
pub trait Client: Send + Sync + std::fmt::Debug {
    fn toggle_theme(&self);

    /// Acquire a screen wake lock. Returns `None` if the platform can't grant
    /// one (unsupported, denied, …). Dropping the returned guard releases the
    /// lock.
    async fn acquire_wake_lock(&self) -> Option<Box<dyn WakeLock>>;
}

/// A held screen wake lock. Dropping it releases the lock.
#[async_trait(?Send)]
pub trait WakeLock {
    /// Resolves if the lock is released out from under us — e.g. the browser
    /// drops it when the tab is hidden. On platforms that hold the lock until
    /// it's dropped, this never resolves.
    async fn lost(&self);
}

static CLIENT: OnceLock<Box<dyn Client>> = OnceLock::new();

pub fn initialize_client(client: Box<dyn Client>) {
    CLIENT.set(client).unwrap();
}

pub fn client() -> &'static dyn Client {
    CLIENT.get().unwrap().as_ref()
}
