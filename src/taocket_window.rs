use crossbeam_channel::{self, Sender};
use global_hotkey::HotKeyState::Released;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};
use tao::{
    dpi::LogicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Window, WindowBuilder},
};
use wry::{NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder, http::Request};

use crate::{
    CustomEvent, callback,
    taocket_config::TaocketConfig,
    taocket_context::{Clients, WebviewContext, WindowContext},
    taocket_hotkey::{HotkeyAndFunc, TaocketHotkeyManager},
    taocket_protocol, taocket_utils,
    ws::{self, Message},
};

// ============================================================================
// Traits
// ============================================================================

/// Trait for providing assets (embedded or from filesystem)
pub trait AssetProvider: Send + Sync + std::fmt::Debug {
    fn get(&self, path: &str) -> Option<Vec<u8>>;
    fn exists(&self, path: &str) -> bool {
        self.get(path).is_some()
    }
}

impl<T: rust_embed::RustEmbed + Send + Sync + std::fmt::Debug> AssetProvider for T {
    fn get(&self, path: &str) -> Option<Vec<u8>> {
        T::get(path).map(|f| f.data.to_vec())
    }
}

// ============================================================================
// Configuration
// ============================================================================

pub struct WindowAttrs {
    pub dev_url: Option<String>,
    pub build_path: String,

    pub with_devtools: bool,
    pub websocket_port: u16,
}

impl std::fmt::Debug for WindowAttrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowAttrs")
            .field("dev_url", &self.dev_url)
            .field("build_path", &self.build_path)
            .field("with_devtools", &self.with_devtools)
            .field("websocket_port", &self.websocket_port)
            .finish()
    }
}

impl Clone for WindowAttrs {
    fn clone(&self) -> Self {
        Self {
            dev_url: self.dev_url.clone(),
            build_path: self.build_path.clone(),

            with_devtools: self.with_devtools,
            websocket_port: self.websocket_port,
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payload<T> {
    pub id: i32,
    pub event: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpcMessage<T> {
    pub payload: Payload<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
enum InternalWindowEvent {
    Minimize,
    Maximize,
    UnMaximize,
    Close,
    Focus,
    IsFocus,
    Move,
    IsMaximized,
    IsMinimized,
}

impl InternalWindowEvent {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Minimize => "Minimize",
            Self::Maximize => "Maximize",
            Self::UnMaximize => "UnMaximize",
            Self::Close => "Close",
            Self::Move => "Move",
            Self::Focus => "Focus",
            Self::IsFocus => "IsFocus",
            Self::IsMaximized => "IsMaximized",
            Self::IsMinimized => "IsMinimized",
        }
    }

    fn to_str_response(&self) -> String {
        format!("{}-response", self.as_str())
    }
}

// ============================================================================
// Builder
// ============================================================================

pub struct TaocketBuilder<A: AssetProvider + 'static, E: CustomEvent = (), X: CustomEvent = ()> {
    config: TaocketConfig,
    embedded_assets: Option<Arc<A>>,
    attr: WindowAttrs,
    _phantom: std::marker::PhantomData<E>,
    _phantom2: std::marker::PhantomData<X>,
}

impl<A: AssetProvider + 'static, E: CustomEvent, X: CustomEvent> TaocketBuilder<A, E, X> {
    pub fn new(config_path: &str, assets: Option<Arc<A>>) -> Self {
        let taocket_config = TaocketConfig::load(config_path).unwrap();
        let attr = WindowAttrs {
            dev_url: if cfg!(debug_assertions) {
                Some(taocket_config.dev_url.clone())
            } else {
                None
            },
            build_path: taocket_config.build_path.to_string_lossy().to_string(),
            with_devtools: taocket_config.devtools,
            websocket_port: taocket_config.websocket_port,
        };
        Self {
            //  pub embedded_assets: Option<Arc<dyn AssetProvider>>,
            embedded_assets: assets,
            config: taocket_config,
            attr: attr,
            _phantom: std::marker::PhantomData,
            _phantom2: std::marker::PhantomData,
        }
    }

    pub fn run<F, S, W, H>(
        self,
        init_window: S,
        handler: F,
        ws_handler: W,
        hotkey_handler: H,
    ) -> wry::Result<()>
    where
        E: DeserializeOwned + Serialize,
        X: DeserializeOwned + Serialize + Clone + std::fmt::Debug,
        S: FnOnce(&Window, Arc<Mutex<TaocketHotkeyManager>>, TaocketConfig),
        F: Fn(Payload<E>, WindowContext<E>) + Send + 'static,
        W: Fn(u64, Message, &Clients, &EventLoopProxy<E>) + Send + 'static,
        H: Fn(Dispatcher<X>, &HotkeyAndFunc) + Send + 'static,
    {
        let event_loop = EventLoopBuilder::<E>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        let window = self.create_window(&event_loop)?;
        let hotkey_manager =
            TaocketHotkeyManager::new().expect("Failed to initialize hotkey manager");
        let manager = Arc::new(Mutex::new(hotkey_manager));
        let manager_clone = Arc::clone(&manager);
        let manager_clone_eventloop = Arc::clone(&manager);
        init_window(&window, manager_clone, self.config.clone());
        let websocket_clients = Arc::new(Mutex::new(HashMap::new()));
        let webview_holder = self.create_webview(&window, &websocket_clients, &proxy, handler)?;

        self.spawn_websocket_thread(websocket_clients, ws_handler, &proxy);
        self.run_event_loop(
            event_loop,
            window,
            webview_holder,
            manager_clone_eventloop,
            hotkey_handler,
        )
    }

    fn create_window(
        &self,
        event_loop: &tao::event_loop::EventLoop<E>,
    ) -> wry::Result<Arc<Window>> {
        let window = WindowBuilder::new()
            .with_transparent(true)
            .with_inner_size(LogicalSize::new(
                self.config.size.width,
                self.config.size.height,
            ))
            .with_always_on_top(self.config.top_most)
            .build(event_loop)
            .expect("Failed to create window");

        Ok(Arc::new(window))
    }

    fn create_webview<F>(
        &self,
        window: &Arc<Window>,
        websocket_clients: &Clients,
        proxy: &EventLoopProxy<E>,
        handler: F,
    ) -> wry::Result<WebviewContext>
    where
        E: DeserializeOwned + Serialize,
        F: Fn(Payload<E>, WindowContext<E>) + Send + 'static,
    {
        let window_clone = Arc::clone(window);
        let clients_clone = Arc::clone(websocket_clients);
        let proxy_clone = Arc::new(proxy.clone());
        let webview_holder: WebviewContext = Arc::new(Mutex::new(None));
        let webview_clone = Arc::clone(&webview_holder);

        let webview_builder = WebViewBuilder::new()
            .with_devtools(self.config.devtools)
            .with_initialization_script(include_str!("scripts/init.js"))
            .with_initialization_script(include_str!("scripts/dragevent.js"))
            .with_new_window_req_handler(Self::handle_new_window_request)
            .with_ipc_handler(move |req: Request<String>| {
                Self::handle_ipc_message(
                    req,
                    &window_clone,
                    &clients_clone,
                    &webview_clone,
                    &proxy_clone,
                    &handler,
                );
            });
        let dev_url = self.attr.dev_url.as_deref().unwrap_or("");
        let webview_builder = if cfg!(debug_assertions) {
            webview_builder
                .with_url(dev_url)
                .with_on_page_load_handler(|p, s| println!("loading page {s}"))
        } else {
            self.setup_production_protocol(webview_builder)
        };

        let webview = webview_builder.build(window)?;
        *webview_holder.lock() = Some(webview);
        Ok(webview_holder)
    }

    fn setup_production_protocol<'a>(&self, builder: WebViewBuilder<'a>) -> WebViewBuilder<'a> {
        let protocol_name = "taocket";
        let emmbeded_assets = self.embedded_assets.as_ref().map(Arc::clone);
        let build_path = self.attr.build_path.clone();

        let build_path = if emmbeded_assets.is_none() {
            Some(taocket_utils::resolve_frontend_path(build_path))
        } else {
            None
        };

        builder
            .with_custom_protocol(protocol_name.to_string(), move |_webview_id, request| {
                match Self::handle_asset_request(request, &emmbeded_assets, &build_path) {
                    Ok(response) => response.map(Into::into),
                    Err(e) => {
                        eprintln!("Asset request error: {}", e);
                        Self::error_response("Internal server error")
                            .unwrap()
                            .map(Into::into)
                    }
                }
            })
            .with_new_window_req_handler(Self::handle_new_window_request)
            .with_url(&format!("{}://localhost", protocol_name))
    }

    fn handle_asset_request(
        request: wry::http::Request<Vec<u8>>,
        embedded_assets: &Option<Arc<A>>,
        build_path: &Option<PathBuf>,
    ) -> wry::Result<wry::http::Response<Vec<u8>>> {
        let path = request.uri().path().trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };

        // Try embedded assets first
        if let Some(assets) = embedded_assets {
            if let Some(content) = assets.get(path) {
                return Self::create_response(path, content);
            }
        }

        // Fallback to filesystem
        if let Some(base_path) = build_path {
            return Ok(taocket_protocol::handle_custom_protocol(request, base_path));
        }

        Self::not_found_response(path)
    }

    fn create_response(path: &str, content: Vec<u8>) -> wry::Result<wry::http::Response<Vec<u8>>> {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .as_ref()
            .to_string();

        Ok(wry::http::Response::builder()
            .status(200)
            .header("Content-Type", mime)
            .header("Access-Control-Allow-Origin", "*")
            .body(content)?)
    }

    fn not_found_response(path: &str) -> wry::Result<wry::http::Response<Vec<u8>>> {
        Ok(wry::http::Response::builder()
            .status(404)
            .header("Content-Type", "text/plain")
            .body(format!("404 Not Found: {}", path).into_bytes())?)
    }

    fn error_response(message: &str) -> wry::Result<wry::http::Response<Vec<u8>>> {
        Ok(wry::http::Response::builder()
            .status(500)
            .header("Content-Type", "text/plain")
            .body(message.as_bytes().to_vec())?)
    }

    fn handle_new_window_request(url: String, _: NewWindowFeatures) -> NewWindowResponse {
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
        proxy: &Arc<EventLoopProxy<E>>,
        handler: &F,
    ) where
        E: DeserializeOwned + Serialize,
        F: Fn(Payload<E>, WindowContext<E>),
    {
        let body = req.body();

        // Handle internal window events
        if let Ok(msg) = serde_json::from_str::<IpcMessage<InternalWindowEvent>>(body) {
            if let Some(ref webview) = *webview_holder.lock() {
                handle_internal_window_event(msg.payload, window, webview);
            }
            return;
        }

        // Handle custom user events
        if let Ok(msg) = serde_json::from_str::<IpcMessage<E>>(body) {
            let context = WindowContext::with_proxy(
                Arc::clone(window),
                Arc::clone(webview_holder),
                Arc::clone(clients),
                Arc::clone(proxy),
            );
            handler(msg.payload, context);
        }
    }

    fn run_event_loop<H>(
        self,
        event_loop: tao::event_loop::EventLoop<E>,
        window: Arc<Window>,
        webview_holder: WebviewContext,
        hotkeymanager: Arc<Mutex<TaocketHotkeyManager>>,
        hotkey_handler: H,
    ) -> wry::Result<()>
    where
        E: Serialize,
        X: Serialize + std::fmt::Debug,
        H: Fn(Dispatcher<X>, &HotkeyAndFunc) + Send + 'static,
    {
        let receiver = global_hotkey::GlobalHotKeyEvent::receiver();
        let (tx, rx) = crossbeam_channel::unbounded::<TxEvent<X>>();
        let dispatcher = Dispatcher::new(tx);
        event_loop.run(move |event, _, control_flow| {
            // *control_flow = ControlFlow::Wait;
            *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + Duration::from_millis(16));

            if let Ok(event) = receiver.try_recv() {

			let guard = hotkeymanager.lock();
			for(key,hk) in	guard.registered_hotkeys.iter(){
				if key == &event.id && event.state == Released{
					hotkey_handler(dispatcher.clone(),hk);
				}
			}
                  }
            match event {
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::UserEvent(custom_event) => {
                    if let Some(ref webview) = *webview_holder.lock() {
                        if let Ok(json) = serde_json::to_string(&custom_event) {


                            let script = format!(
                                "window.dispatchEvent(new CustomEvent('taocket:websocket|event', {{ detail: {} }}));",
                                json
                            );

                            if let Err(e) = webview.evaluate_script(&script) {
                                eprintln!("Failed to send event to frontend: {}", e);
                            }
                        }
                    }
                }
                Event::WindowEvent { event, .. } => match event {
                    tao::event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                },
                _ => {}
            }
            while let Ok(msg) = rx.try_recv(){
            match msg{
                TxEvent::User(_) => {
                            },
                TxEvent::Window(w) => {
               	            match w {
                                UserWindowEvent::Minimize => {
                                window.set_minimized(true);
                                },
                                UserWindowEvent::Maximize => window.set_maximized(true),
                                UserWindowEvent::UnMaximize =>window.set_maximized(false) ,
                                UserWindowEvent::Close => *control_flow=ControlFlow::Exit,
                                UserWindowEvent::Focus => window.set_focus(),
                            }
                            },
                TxEvent::Script(scrpt) => {

		                        if let Some(ref webview) = *webview_holder.lock() {
		                         println!("message");
		                         _ = webview.evaluate_script(&scrpt);
		                        }
                },
            };

            }
        });
    }

    fn spawn_websocket_thread<W>(
        &self,
        websocket_clients: Clients,
        ws_handler: W,
        proxy: &EventLoopProxy<E>,
    ) where
        W: Fn(u64, Message, &Clients, &EventLoopProxy<E>) + Send + 'static,
        E: CustomEvent,
    {
        let event_hub =
            ws::launch(self.attr.websocket_port).expect("Failed to launch WebSocket server");
        let event_proxy = proxy.clone();

        std::thread::spawn(move || {
            loop {
                match event_hub.poll_event() {
                    ws::Event::Connect(client_id, responder) => {
                        websocket_clients.lock().insert(client_id, responder);
                    }
                    ws::Event::Disconnect(client_id) => {
                        websocket_clients.lock().remove(&client_id);
                    }
                    ws::Event::Message(client_id, message) => {
                        ws_handler(client_id, message, &websocket_clients, &event_proxy);
                    }
                }
            }
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum UserWindowEvent {
    Minimize,
    Maximize,
    UnMaximize,
    Close,
    Focus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "source", content = "payload")]
pub enum TxEvent<X> {
    User(X),
    Script(String),
    Window(UserWindowEvent),
}
#[derive(Debug, Clone)]
pub struct Dispatcher<X>
where
    X: Serialize + std::fmt::Debug + Send + Clone + 'static,
{
    tx: Sender<TxEvent<X>>,
}
impl<X> Dispatcher<X>
where
    X: Serialize + std::fmt::Debug + Send + Clone + 'static,
{
    pub fn new(tx: Sender<TxEvent<X>>) -> Self {
        Self { tx }
    }
    pub fn send_script(&self, script: String) {
        self.tx.send(TxEvent::Script(script)).unwrap();
    }
    pub fn send_user(&self, event: X) {
        self.tx.send(TxEvent::User(event)).unwrap();
    }

    pub fn send_window(&self, event: UserWindowEvent) {
        self.tx.send(TxEvent::Window(event)).unwrap();
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

pub fn broadcast_message(clients: &Clients, message: String) {
    for (_, responder) in clients.lock().iter() {
        let _ = responder.send(Message::Text(message.clone()));
    }
}

// ============================================================================
// Internal Window Event Handler
// ============================================================================

fn handle_internal_window_event(
    payload: Payload<InternalWindowEvent>,
    window: &Window,
    webview: &WebView,
) {
    match payload.event {
        InternalWindowEvent::Minimize => window.set_minimized(true),
        InternalWindowEvent::Maximize => {
            let is_maximized = window.is_maximized();
            window.set_maximized(!is_maximized);
        }
        InternalWindowEvent::UnMaximize => window.set_maximized(false),
        InternalWindowEvent::Close => std::process::exit(0),
        InternalWindowEvent::Move => {
            let _ = window.drag_window();
        }
        InternalWindowEvent::Focus => window.set_focus(),
        InternalWindowEvent::IsMaximized => {
            send_window_state_response(webview, &payload.event, window.is_maximized());
        }
        InternalWindowEvent::IsMinimized => {
            send_window_state_response(webview, &payload.event, window.is_minimized());
        }
        InternalWindowEvent::IsFocus => {
            send_window_state_response(webview, &payload.event, window.is_focused());
        }
    }
}

fn send_window_state_response(webview: &WebView, event: &InternalWindowEvent, state: bool) {
    let payload = WindowAttrPayload {
        attr_type: event.as_str(),
        value: serde_json::Value::Bool(state),
    };
    callback!(webview, event.to_str_response(), payload);
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
