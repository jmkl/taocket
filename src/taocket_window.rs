use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Window, WindowBuilder},
};
use wry::{NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder, http::Request};

use crate::{
    callback, emit_js,
    ws::{self, Message, Responder},
};

// ============================================================================
// Type Aliases
// ============================================================================

type Clients = Arc<Mutex<HashMap<u64, Responder>>>;
type WebviewContext = Arc<Mutex<Option<WebView>>>;

// ============================================================================
// Traits
// ============================================================================

/// Trait for custom events that can be sent through the event loop
pub trait CustomEvent: Clone + Send + 'static {}
impl<T: Clone + Send + 'static> CustomEvent for T {}

// ============================================================================
// Context Types
// ============================================================================

/// Context provided to event handlers, containing window and WebSocket client references
#[derive(Clone)]
pub struct WindowContext {
    pub window: Arc<Window>,
    pub clients: Clients,
}

impl WindowContext {
    fn new(window: Arc<Window>, clients: Clients) -> Self {
        Self { window, clients }
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Configuration options for the window
#[derive(Debug, Clone)]
pub struct WindowAttrs {
    pub dev_url: Option<String>,
    pub build_path: String,
    pub with_devtools: bool,
    pub websocket_port: u16,
}

// ============================================================================
// Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowAttrPayload {
    #[serde(rename = "type")]
    pub attr_type: &'static str,
    pub value: serde_json::Value,
}

/// Generic payload wrapper for events with optional data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payload<T> {
    pub id: i32,
    pub event: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// IPC message wrapper
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpcMessage<T> {
    pub payload: Payload<T>,
}

/// WebSocket message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WsMessage {
    name: &'static str,
    message: &'static str,
}

/// Internal window control events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
enum InternalWindowEvent {
    Minimize,
    Maximize,
    UnMaximize,
    Close,
    Move,
    IsMaximized,
    IsMinimized,
}
impl InternalWindowEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Minimize => "Minimize",
            Self::Maximize => "Maximize",
            Self::UnMaximize => "UnMaximize",
            Self::Close => "Close",
            Self::Move => "Move",
            Self::IsMaximized => "IsMaximized",
            Self::IsMinimized => "IsMinimized",
        }
    }
    pub fn to_str_response(&self) -> String {
        format!("{}-response", self.as_str())
    }
}
// ============================================================================
// Builder
// ============================================================================

/// Builder for creating and running a Taocket window application
#[derive(Debug, Clone)]
pub struct TaocketBuilder<E: CustomEvent = ()> {
    attr: WindowAttrs,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: CustomEvent> TaocketBuilder<E> {
    /// Creates a new TaocketBuilder with the given window attributes
    pub fn new(attr: WindowAttrs) -> Self {
        Self {
            attr,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Runs the application with the provided initialization and event handler
    ///
    /// # Arguments
    /// * `init_window` - Function called once during window initialization
    /// * `handler` - Function called for each user event
    pub fn run<F, S>(self, init_window: S, handler: F) -> wry::Result<()>
    where
        E: DeserializeOwned + Serialize,
        S: FnOnce(&Window),
        F: Fn(Payload<E>, WindowContext) + Send + 'static,
    {
        let event_loop = EventLoopBuilder::<E>::with_user_event().build();

        let window = self.create_window(&event_loop)?;
        init_window(&window);

        let websocket_clients = Arc::new(Mutex::new(HashMap::new()));
        let webview_holder = self.create_webview(&window, &websocket_clients, handler)?;

        self.run_event_loop(event_loop, window, websocket_clients, webview_holder)
    }

    // ------------------------------------------------------------------------
    // Private Helper Methods
    // ------------------------------------------------------------------------

    fn create_window(
        &self,
        event_loop: &tao::event_loop::EventLoop<E>,
    ) -> wry::Result<Arc<Window>> {
        let window = WindowBuilder::new()
            .with_transparent(true)
            .build(event_loop)
            .expect("Failed to create window");

        Ok(Arc::new(window))
    }

    fn create_webview<F>(
        &self,
        window: &Arc<Window>,
        websocket_clients: &Clients,
        handler: F,
    ) -> wry::Result<WebviewContext>
    where
        E: DeserializeOwned + Serialize,
        F: Fn(Payload<E>, WindowContext) + Send + 'static,
    {
        let window_for_ipc = Arc::clone(window);
        let clients_for_ipc = Arc::clone(websocket_clients);
        let webview_holder: WebviewContext = Arc::new(Mutex::new(None));
        let webview_for_handler = Arc::clone(&webview_holder);

        let webview = WebViewBuilder::new()
            .with_initialization_script(include_str!("scripts/init.js"))
            .with_initialization_script(include_str!("scripts/dragevent.js"))
            .with_new_window_req_handler(Self::handle_new_window_request)
            .with_url(self.attr.dev_url.as_deref().unwrap_or(""))
            .with_ipc_handler(move |req: Request<String>| {
                Self::handle_ipc_message(
                    req,
                    &window_for_ipc,
                    &clients_for_ipc,
                    &webview_for_handler,
                    &handler,
                );
            })
            .build(window)?;

        *webview_holder.lock() = Some(webview);
        Ok(webview_holder)
    }

    fn handle_new_window_request(url: String, _features: NewWindowFeatures) -> NewWindowResponse {
        if let Err(e) = open::that(&url) {
            eprintln!("Failed to open URL: {}", e);
        }
        NewWindowResponse::Deny
    }

    fn handle_ipc_message<F>(
        req: Request<String>,
        window: &Arc<Window>,
        clients: &Clients,
        webview_holder: &WebviewContext,
        handler: &F,
    ) where
        E: DeserializeOwned,
        F: Fn(Payload<E>, WindowContext),
    {
        let body = req.body();

        // Try to parse as internal window event
        if let Ok(msg) = serde_json::from_str::<IpcMessage<InternalWindowEvent>>(body) {
            if let Some(ref webview) = *webview_holder.lock() {
                handle_internal_window_event(msg.payload, window, webview);
            }
            return;
        }
        if let Ok(msg) = serde_json::from_str::<IpcMessage<E>>(body) {
            let context = WindowContext::new(Arc::clone(window), Arc::clone(clients));
            handler(msg.payload, context);
        }
    }

    fn run_event_loop(
        self,
        event_loop: tao::event_loop::EventLoop<E>,
        window: Arc<Window>,
        websocket_clients: Clients,
        _webview_holder: WebviewContext,
    ) -> wry::Result<()> {
        // Spawn WebSocket event polling thread
        self.spawn_websocket_thread(websocket_clients.clone());

        // Run the main event loop
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::WindowEvent { event, .. } => {
                    if let tao::event::WindowEvent::CloseRequested = event {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                _ => {}
            }
        });
    }

    fn spawn_websocket_thread(&self, websocket_clients: Clients) {
        let event_hub =
            ws::launch(self.attr.websocket_port).expect("Failed to launch WebSocket server");

        std::thread::spawn(move || {
            loop {
                match event_hub.poll_event() {
                    ws::Event::Connect(client_id, responder) => {
                        websocket_clients.lock().insert(client_id, responder);
                    }
                    ws::Event::Disconnect(client_id) => {
                        websocket_clients.lock().remove(&client_id);
                    }
                    ws::Event::Message(client_id, _message) => {
                        Self::handle_websocket_message(client_id, &websocket_clients);
                    }
                }
            }
        });
    }

    fn handle_websocket_message(client_id: u64, clients: &Clients) {
        if let Some(responder) = clients.lock().get(&client_id) {
            let response = WsMessage {
                name: "jul",
                message: "im stuff",
            };

            if let Ok(json) = serde_json::to_string(&response) {
                responder.send(Message::Text(json));
            }
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Broadcasts a message to all connected WebSocket clients
pub fn broadcast_message(clients: &Clients, message: &str) {
    for (_id, responder) in clients.lock().iter() {
        responder.send(Message::Text(message.to_string()));
    }
}

// ============================================================================
// Internal Window Event Handler
// ============================================================================

fn handle_internal_window_event(
    payload: Payload<InternalWindowEvent>,
    window: &Window,
    _webview: &WebView,
) {
    match payload.event {
        InternalWindowEvent::Minimize => {
            window.set_minimized(true);
        }
        InternalWindowEvent::Maximize => {
            window.set_maximized(true);
        }
        InternalWindowEvent::UnMaximize => {
            window.set_maximized(false);
        }
        InternalWindowEvent::Close => {
            std::process::exit(0);
        }
        InternalWindowEvent::Move => {
            let _ = window.drag_window();
        }
        InternalWindowEvent::IsMaximized => {
            let ismax = window.is_maximized();
            let le_payload = WindowAttrPayload {
                attr_type: &payload.event.as_str(),
                value: serde_json::Value::Bool(ismax),
            };

            callback!(_webview, payload.event.to_str_response(), le_payload);
        }
        InternalWindowEvent::IsMinimized => {
            std::process::exit(0);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_serialization() {
        let message = IpcMessage {
            payload: Payload {
                id: 1,
                event: InternalWindowEvent::Close,
                value: Some(serde_json::Value::Bool(true)),
            },
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: IpcMessage<InternalWindowEvent> = serde_json::from_str(&json).unwrap();

        assert_eq!(message, deserialized);
    }

    #[test]
    fn test_payload_without_value() {
        let payload: Payload<InternalWindowEvent> = Payload {
            id: 42,
            event: InternalWindowEvent::Minimize,
            value: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(!json.contains("value"));
    }
}
