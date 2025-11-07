use std::sync::{Arc, Mutex};

use tao::window::Window;
use wry::WebView;

use crate::{Payload, WindowControl, emit};

// Internal handler for WindowControl events
pub fn handle_window_control_internal(
    payload: Payload<WindowControl>,
    webview: Arc<Mutex<Option<WebView>>>,
    window: Arc<Window>,
) {
    match payload.event {
        WindowControl::Minimize => {
            window.set_minimized(true);
        }
        WindowControl::Maximize => {
            window.set_maximized(true);
        }
        WindowControl::UnMaximize => {
            window.set_maximized(false);
        }
        WindowControl::Close => {
            std::process::exit(0);
        }
        WindowControl::GetSize => {
            let size = window.inner_size();
            let response = serde_json::json!({
                "width": size.width,
                "height": size.height
            });
            emit!(&webview, payload.id, response);
        }
        WindowControl::SetSize { width, height } => {
            use tao::dpi::LogicalSize;
            window.set_inner_size(LogicalSize::new(width, height));
            let response = serde_json::json!({"success": true});
            emit!(&webview, payload.id, response);
        }
        WindowControl::SetPosition { x, y } => {
            use tao::dpi::LogicalPosition;
            window.set_outer_position(LogicalPosition::new(x, y));
            let response = serde_json::json!({"success": true});
            emit!(&webview, payload.id, response);
        }
        WindowControl::GetPosition => {
            let position = window.outer_position().unwrap_or_default();
            let response = serde_json::json!({
                "x": position.x,
                "y": position.y
            });
            emit!(&webview, payload.id, response);
        }
    }
}
