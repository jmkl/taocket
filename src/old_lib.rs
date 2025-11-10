pub mod app;
mod config;
mod events;
pub mod ipc;
mod protocol;
mod taocket_macro;
pub mod taocket_window;
mod userevent;
mod windowcontrol;
pub mod ws;
pub use config::AppConfig;
pub use events::{Payload, WindowControl, WindowControlMessage};
pub use ipc::*;
use open;
use std::sync::{Arc, Mutex};
use userevent::{HandlerContext, UserEvent};
use windowcontrol::handle_window_control_internal;

use serde::{Serialize, de::DeserializeOwned};
use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Window, WindowBuilder},
};
use wry::{
    NewWindowFeatures, NewWindowResponse, WebView, WebViewBuilder,
    http::{Request, Response, header::CONTENT_TYPE},
};

pub struct AppBuilder<E: UserEvent = ()> {
    config: AppConfig,
    _phantom: std::marker::PhantomData<E>,
}

impl<E: UserEvent> AppBuilder<E> {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn run<F>(self, handler: F) -> wry::Result<()>
    where
        E: DeserializeOwned + Serialize,
        F: Fn(Payload<E>, HandlerContext<E>) + Send + 'static,
    {
        let event_loop = EventLoopBuilder::<E>::with_user_event().build();
        let event_proxy = event_loop.create_proxy();
        let wv_ref: Arc<Mutex<Option<WebView>>> = Arc::new(Mutex::new(None));
        let wv_clone = Arc::clone(&wv_ref);

        let window = Arc::new(
            WindowBuilder::new()
                .with_decorations(self.config.with_decorations)
                .with_resizable(true)
                .build(&event_loop)
                .expect("Error building window..."),
        );

        let mut webview_builder = WebViewBuilder::new();
        println!("DEV MODE {:?}", &self.config.dev_url);

        // Configure URL based on dev mode or production
        if let Some(dev_url) = &self.config.dev_url {
            webview_builder = webview_builder
                .with_new_window_req_handler(|url: String, _features: NewWindowFeatures| {
                    if let Err(e) = open::that(&url) {
                        eprintln!("Failed to open URL: {e}");
                    }
                    NewWindowResponse::Deny
                })
                .with_url(dev_url);
        } else {
            let protocol_name: String = "guilib".into();
            let build_path = self.config.build_path.clone();
            println!("frontend dir {:?}", &build_path);
            println!("Current dir: {:?}", std::env::current_dir()?);
            webview_builder = webview_builder
                .with_custom_protocol(protocol_name.clone(), move |_webview_id, request| {
                    match protocol::get_response(request, &build_path) {
                        Ok(response) => response.map(Into::into),
                        Err(e) => {
                            eprintln!("Protocol error: {}", e);
                            Response::builder()
                                .header(CONTENT_TYPE, "text/plain")
                                .status(500)
                                .body(e.to_string().as_bytes().to_vec())
                                .unwrap()
                                .map(Into::into)
                        }
                    }
                })
                .with_new_window_req_handler(|url: String, _features: NewWindowFeatures| {
                    println!("the url{}", &url);
                    if let Err(e) = open::that(&url) {
                        eprintln!("Failed to open URL: {e}");
                    }

                    // Prevent new WebView windows; just open externally
                    NewWindowResponse::Deny
                })
                .with_url(&format!("{}://localhost", protocol_name));
        }

        let window_clone = Arc::clone(&window);
        let proxy_clone = event_proxy.clone();

        webview_builder = webview_builder
            .with_devtools(self.config.with_devtools)
            .with_ipc_handler(move |req: Request<String>| {
                // Try to parse as WindowControl first
                if let Ok(imsg) =
                    serde_json::from_str::<WindowControlMessage<WindowControl>>(req.body())
                {
                    handle_window_control_internal(
                        imsg.payload,
                        Arc::clone(&wv_clone),
                        Arc::clone(&window_clone),
                    );
                }
                // Try to parse as user event and pass to handler
                else if let Ok(imsg) = serde_json::from_str::<WindowControlMessage<E>>(req.body())
                {
                    let ctx = HandlerContext {
                        webview: Arc::clone(&wv_clone),
                        window: Arc::clone(&window_clone),
                        event_proxy: proxy_clone.clone(),
                    };
                    handler(imsg.payload, ctx);
                } else {
                    eprintln!("Failed to parse IPC message: {}", req.body());
                }
            });

        let webview = webview_builder.build(&window)?;
        *wv_ref.lock().unwrap() = Some(webview);
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: tao::event::WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => (),
            }
        });
    }

    pub fn run_with_setup<F, S>(self, setup: S, handler: F) -> wry::Result<()>
    where
        E: DeserializeOwned + Serialize,
        F: Fn(Payload<E>, HandlerContext<E>) + Send + 'static,
        S: FnOnce(&Window),
    {
        let event_loop = EventLoopBuilder::<E>::with_user_event().build();
        let event_proxy = event_loop.create_proxy();

        let wv_ref: Arc<Mutex<Option<WebView>>> = Arc::new(Mutex::new(None));
        let wv_clone = Arc::clone(&wv_ref);

        let window = Arc::new(
            WindowBuilder::new()
                .with_decorations(self.config.with_decorations)
                .with_resizable(true)
                .build(&event_loop)
                .expect("Error building window..."),
        );

        // Call setup function before creating webview
        setup(&window);

        let mut webview_builder = WebViewBuilder::new();
        println!("DEV MODE {:?}", &self.config.dev_url);

        if let Some(dev_url) = &self.config.dev_url {
            webview_builder = webview_builder
                .with_new_window_req_handler(|url: String, _features: NewWindowFeatures| {
                    println!("the url{}", &url);
                    if let Err(e) = open::that(&url) {
                        eprintln!("Failed to open URL: {e}");
                    }
                    NewWindowResponse::Deny
                })
                .with_url(dev_url);
        } else {
            let protocol_name: String = "guilib".into();
            let build_path = self.config.build_path.clone();
            println!("frontend dir {:?}", &build_path);
            println!("Current dir: {:?}", std::env::current_dir()?);
            webview_builder = webview_builder
                .with_custom_protocol(protocol_name.clone(), move |_webview_id, request| {
                    protocol::handle_custom_protocol(request, &build_path).map(Into::into)
                })
                .with_new_window_req_handler(|url: String, _features: NewWindowFeatures| {
                    println!("the url{}", &url);
                    if let Err(e) = open::that(&url) {
                        eprintln!("Failed to open URL: {e}");
                    }
                    NewWindowResponse::Deny
                })
                .with_url(&format!("{}://localhost", protocol_name));
        }

        let window_clone = Arc::clone(&window);
        let proxy_clone = event_proxy.clone();

        webview_builder = webview_builder
            .with_devtools(self.config.with_devtools)
            .with_ipc_handler(move |req: Request<String>| {
                // Try to parse as WindowControl first
                if let Ok(imsg) =
                    serde_json::from_str::<WindowControlMessage<WindowControl>>(req.body())
                {
                    handle_window_control_internal(
                        imsg.payload,
                        Arc::clone(&wv_clone),
                        Arc::clone(&window_clone),
                    );
                }
                // Try to parse as user event and pass to handler
                else if let Ok(imsg) = serde_json::from_str::<WindowControlMessage<E>>(req.body())
                {
                    let ctx = HandlerContext {
                        webview: Arc::clone(&wv_clone),
                        window: Arc::clone(&window_clone),
                        event_proxy: proxy_clone.clone(),
                    };
                    handler(imsg.payload, ctx);
                } else {
                    eprintln!("Failed to parse IPC message: {}", req.body());
                }
            });

        let webview = webview_builder.build(&window)?;

        *wv_ref.lock().unwrap() = Some(webview);

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: tao::event::WindowEvent::CloseRequested,
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => (),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_serialization() {
        let imsg = WindowControlMessage {
            payload: Payload {
                id: 32,
                value: Some(serde_json::json!("Something")),
                event: WindowControl::GetPosition,
            },
        };
        let json = serde_json::to_string(&imsg).unwrap();
        println!("{}", json);

        let deserialized: WindowControlMessage<WindowControl> =
            serde_json::from_str(&json).unwrap();
        assert_eq!(imsg, deserialized);
    }
}
