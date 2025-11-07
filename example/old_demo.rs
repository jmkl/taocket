use serde_json::json;
use std::{collections::HashMap, sync::Arc};
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data")]
#[serde(rename = "AppEventHandler")]
#[ts(export, export_to = "../taoket_types.d.ts")]
pub enum MyEvent {
    Close,
    Minimize,
    Maximize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WsMessage {
    name: &'static str,
    message: &'static str,
}
fn main() {
    let config = AppConfig {
        with_decorations: false,
        dev_url: Some("http://localhost:5173".into()),
        build_path: "frontend/dist".into(),
        with_devtools: true,
    };

    App::builder::<MyEvent>(config)
        .run(|payload, ctx| match payload.event {
            MyEvent::Minimize => {
                emit!(&ctx.webview, payload.id, json!("fuck"));
                ctx.window.set_minimized(true);
            }
            MyEvent::Close => {
                std::process::exit(69);
            }
            MyEvent::Maximize => {
                ctx.window.set_maximized(true);
            }
        })
        .unwrap();
}
