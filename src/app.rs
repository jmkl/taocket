use crate::{AppBuilder, AppConfig, UserEvent};

pub struct App;

impl App {
    pub fn new(config: AppConfig) -> wry::Result<()> {
        AppBuilder::<()>::new(config).run(|_payload, _ctx| {
            // No-op default handler
        })
    }

    pub fn builder<E: UserEvent>(config: AppConfig) -> AppBuilder<E> {
        AppBuilder::new(config)
    }
}
