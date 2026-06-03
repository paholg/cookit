use std::sync::OnceLock;

pub trait Client: Send + Sync + std::fmt::Debug {
    fn toggle_theme(&self);
}

static CLIENT: OnceLock<Box<dyn Client>> = OnceLock::new();

pub fn initialize_client(client: Box<dyn Client>) {
    CLIENT.set(client).unwrap();
}

pub fn client() -> &'static dyn Client {
    CLIENT.get().unwrap().as_ref()
}
