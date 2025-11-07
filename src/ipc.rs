use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use wry::WebView;

/// Message received from JS
#[derive(Deserialize, Debug, Clone)]
pub struct IpcRequest {
    pub id: i32,
    pub event: String,
    #[serde(default)]
    pub payload: Option<String>,
}

/// Message sent back to JS
#[derive(Serialize)]
pub struct IpcResponse<T: Serialize> {
    pub id: i32,
    pub result: T,
}

/// Message for error handling
#[derive(Serialize)]
pub struct IpcError {
    pub id: i32,
    pub error: String,
}

/// Send JSON message back to WebView
pub fn send_js<T: Serialize>(webview: &Arc<Mutex<Option<WebView>>>, id: i32, value: T) {
    let response = IpcResponse { id, result: value };
    if let Ok(json) = serde_json::to_string(&response) {
        if let Some(wv) = webview.lock().unwrap().as_ref() {
            let js = format!("window.postMessage({})", json);
            let _ = wv.evaluate_script(&js);
        }
    }
}

/// Send error message back to WebView
pub fn send_error(webview: &Arc<Mutex<Option<WebView>>>, id: i32, error: String) {
    let err = IpcError { id, error };
    if let Ok(json) = serde_json::to_string(&err) {
        if let Some(wv) = webview.lock().unwrap().as_ref() {
            let js = format!("window.postMessage({})", json);
            let _ = wv.evaluate_script(&js);
        }
    }
}

/// Macro helper to reply to frontend
#[macro_export]
macro_rules! emit {
    // full form: emit!(webview, "event", value)
    ($webview:expr, $id:expr, $value_expr:expr) => {
        $crate::ipc::send_js($webview, $id, $value_expr);
    };
    // shorthand form: emit!(webview, "event")
    ($webview:expr, $id:expr) => {
        $crate::ipc::send_js($webview, $id, serde_json::json!(null));
    };
}

/// Macro helper to send errors to frontend
#[macro_export]
macro_rules! emit_err {
    ($webview:expr, $id:expr, $msg:expr) => {{
        $crate::ipc::send_error($webview, $id, $msg.to_string());
    }};
}
