use parking_lot::Mutex;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event_loop::EventLoopProxy,
    window::Window,
};
use wry::WebView;

// ============================================================================
// Type Aliases
// ============================================================================

pub type Clients = Arc<Mutex<HashMap<u64, crate::ws::Responder>>>;
pub type WebviewContext = Arc<Mutex<Option<WebView>>>;

// ============================================================================
// Script Events
// ============================================================================

#[derive(Debug, Clone)]
pub enum ScriptEvent {
    Raw(String),
    CustomEvent {
        name: String,
        detail: String,
    },
    Reload,
    Navigate(String),
    #[cfg(debug_assertions)]
    ToggleDevTools,
}

// ============================================================================
// Window Context
// ============================================================================

pub struct WindowContext<E: Clone + Send + 'static = ()> {
    window: Arc<Window>,
    webview: WebviewContext,
    clients: Clients,
    event_proxy: Option<Arc<EventLoopProxy<E>>>,
}

impl<E: Clone + Send + 'static> WindowContext<E> {
    pub fn new(window: Arc<Window>, webview: WebviewContext, clients: Clients) -> Self {
        Self {
            window,
            webview,
            clients,
            event_proxy: None,
        }
    }

    pub fn with_proxy(
        window: Arc<Window>,
        webview: WebviewContext,
        clients: Clients,
        proxy: Arc<EventLoopProxy<E>>,
    ) -> Self {
        Self {
            window,
            webview,
            clients,
            event_proxy: Some(proxy),
        }
    }

    // ========================================================================
    // Script Execution
    // ========================================================================

    pub fn execute_script(&self, event: ScriptEvent) -> Result<(), String> {
        let webview_guard = self.webview.lock();
        let webview = webview_guard.as_ref().ok_or("Webview not initialized")?;

        let script = match event {
            ScriptEvent::Raw(js) => js,
            ScriptEvent::CustomEvent { name, detail } => {
                format!(
                    "window.dispatchEvent(new CustomEvent('{}', {{ detail: {} }}));",
                    name, detail
                )
            }
            ScriptEvent::Reload => {
                return webview
                    .evaluate_script("window.location.reload();")
                    .map_err(|e| e.to_string());
            }
            ScriptEvent::Navigate(url) => {
                format!("window.location.href = '{}';", url)
            }
            #[cfg(debug_assertions)]
            ScriptEvent::ToggleDevTools => {
                webview.open_devtools();
                return Ok(());
            }
        };

        webview.evaluate_script(&script).map_err(|e| e.to_string())
    }

    // ========================================================================
    // Event Emission
    // ========================================================================

    pub fn emit_event(&self, event: E) -> Result<(), String>
    where
        E: Serialize,
    {
        self.event_proxy
            .as_ref()
            .ok_or("Event proxy not available")?
            .send_event(event)
            .map_err(|_| format!("Failed to send event"))
    }

    // ========================================================================
    // WebSocket Operations
    // ========================================================================

    pub fn broadcast(&self, message: impl Into<String>) {
        let msg = message.into();
        for (_, client) in self.clients.lock().iter() {
            let _ = client.send(crate::ws::Message::Text(msg.clone()));
        }
    }

    pub fn send_to_client(
        &self,
        client_id: u64,
        message: impl Into<String>,
    ) -> Result<bool, String> {
        Ok(self
            .clients
            .lock()
            .get(&client_id)
            .ok_or_else(|| format!("Client {} not found", client_id))?
            .send(crate::ws::Message::Text(message.into())))
    }

    pub fn client_count(&self) -> usize {
        self.clients.lock().len()
    }

    // ========================================================================
    // Window Operations
    // ========================================================================

    pub fn window(&self) -> &Arc<Window> {
        &self.window
    }
    pub fn set_position(&self, position: (f32, f32)) {
        _ = &self
            .window
            .set_outer_position(LogicalPosition::new(position.0, position.1));
    }

    pub fn set_size(&self, size: (f32, f32)) {
        _ = &self.window.set_inner_size(LogicalSize::new(size.0, size.1));
    }

    pub fn clients(&self) -> &Clients {
        &self.clients
    }
    pub fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }
    pub fn minimize(&self) {
        self.window.set_minimized(true);
    }
    pub fn maximize(&self) {
        self.window.set_maximized(true);
    }
    pub fn is_maximized(&self) -> bool {
        self.window.is_maximized()
    }

    pub fn is_minimized(&self) -> bool {
        self.window.is_minimized()
    }
    pub fn set_focus(&self) {
        self.window.set_focus();
    }
    pub fn set_always_on_top(&self, top_most: bool) {
        self.window.set_always_on_top(top_most);
    }
    pub fn set_always_on_bottom(&self, bottom_most: bool) {
        self.window.set_always_on_bottom(bottom_most);
    }
    pub fn set_fullscreen(&self, fullscreen: bool) {
        if fullscreen {
            self.window
                .set_fullscreen(Some(tao::window::Fullscreen::Borderless(None)));
        } else {
            self.window.set_fullscreen(None);
        }
    }
}

impl<E: Clone + Send + 'static> Clone for WindowContext<E> {
    fn clone(&self) -> Self {
        Self {
            window: Arc::clone(&self.window),
            webview: Arc::clone(&self.webview),
            clients: Arc::clone(&self.clients),
            event_proxy: self.event_proxy.clone(),
        }
    }
}
