use std::sync::{Arc, Mutex};
use tao::{event_loop::EventLoopProxy, window::Window};
use wry::WebView;

pub trait UserEvent: Clone + Send + 'static {}
impl<T: Clone + Send + 'static> UserEvent for T {}
/// Context provided to event handlers
pub struct HandlerContext<E: UserEvent> {
    pub webview: Arc<Mutex<Option<WebView>>>,
    pub window: Arc<Window>,
    pub event_proxy: EventLoopProxy<E>,
}

impl<E: UserEvent> Clone for HandlerContext<E> {
    fn clone(&self) -> Self {
        Self {
            webview: Arc::clone(&self.webview),
            window: Arc::clone(&self.window),
            event_proxy: self.event_proxy.clone(),
        }
    }
}
