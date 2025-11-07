use serde::{Deserialize, Serialize};

use tao::dpi::LogicalSize;
use taocket::{
    taocket_window::{TaocketBuilder, WindowAttrs, broadcast_message},
    ws::Message,
};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../taoket_types.d.ts")]
struct TemplateLine {
    text: String,
    scale: i32,
    include: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "../taoket_types.d.ts")]
struct WebsocketMessageResponse {
    from_server: bool,
    #[serde(rename = "type")]
    msg_type: String,
    content: String,
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
            broadcast_message(&_ctx.clients, content);

            match payload.event {
                FrontEndEvent::Template { template } => {}
                FrontEndEvent::RawFilter => todo!(),
                FrontEndEvent::Color => todo!(),
            }
        },
        //web socket message
        |id, message, clients| match message {
            Message::Text(some) => {
                let response = WebsocketMessageResponse {
                    from_server: false,
                    msg_type: "some".into(),
                    content: format!("Reply to {some}"),
                };
                if let Ok(json) = serde_json::to_string(&response) {
                    let clients_guard = clients.lock(); // one lock for the duration
                    if let Some(_responder) = clients_guard.get(&id) {
                        for (_id, responder) in clients_guard.iter() {
                            let _ = responder.send(Message::Text(json.clone()));
                        }
                    }
                }
            }
        },
    )
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
