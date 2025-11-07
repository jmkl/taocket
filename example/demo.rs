use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tao::dpi::LogicalSize;
use taocket::{
    AppConfig,
    app::App,
    emit,
    taocket_window::{Payload, TaocketBuilder, WindowAttrs, broadcast_message},
    ws::{self, Event, Message, Responder},
};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "data")]
#[serde(rename = "AppEventHandler")]
#[ts(export, export_to = "../taoket_types.d.ts")]
pub enum MyEvent {
    Close,
    Minimize,
    Maximize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../taoket_types.d.ts")]
struct TemplateLine {
    text: String,
    scale: i32,
    include: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../taoket_types.d.ts")]
struct Template {
    name: String,
    content: Vec<TemplateLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(tag = "type", content = "value")]
#[ts(export, export_to = "../taoket_types.d.ts")]
enum FrontEndEvent {
    Template { template: Template },
    RawFilter,
    Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WsMessage {
    name: &'static str,
    message: &'static str,
}

fn main() -> wry::Result<()> {
    TaocketBuilder::<FrontEndEvent>::new(WindowAttrs {
        dev_url: Some("http://localhost:5173".into()),
        build_path: "frontend/dist".into(),
        with_devtools: true,
        websocket_port: 1818,
    })
    .run(
        |window| {
            window.set_decorations(false);
            window.set_inner_size(LogicalSize::new(300, 600));
        },
        |payload, _ctx| {
            let content = serde_json::to_string_pretty(&payload.event).unwrap();
            broadcast_message(&_ctx.clients, &content);

            match payload.event {
                FrontEndEvent::Template { template } => {}
                FrontEndEvent::RawFilter => todo!(),
                FrontEndEvent::Color => todo!(),
            }
        },
    )
}
fn _main() {
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
#[cfg(test)]
mod example_test {
    use super::*;
    #[test]
    fn frontend_event() {
        let message = FrontEndEvent::Template {
            template: Template {
                name: "rh".into(),
                content: vec![TemplateLine {
                    text: "hello".into(),
                    scale: 1,
                    include: true,
                }],
            },
        };
        //{"type":"Template","data":{"template":{"name":"rh","content":[{"text":"hello","scale":1,"include":true}]}}}
        let json = serde_json::to_string(&message).unwrap();

        let deserialized: FrontEndEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(message, deserialized);
    }
}
