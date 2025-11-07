use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum WindowControl {
    Minimize,
    Maximize,
    UnMaximize,
    Close,
    GetSize,
    GetPosition,
    SetSize { width: u32, height: u32 },
    SetPosition { x: i32, y: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Payload<T> {
    pub id: i32,

    pub event: T,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowControlMessage<T> {
    pub payload: Payload<T>,
}

impl<T> WindowControlMessage<T> {
    pub fn new(id: i32, event: T) -> Self {
        Self {
            payload: Payload {
                id,
                event,
                value: None,
            },
        }
    }

    pub fn with_value(id: i32, event: T, value: serde_json::Value) -> Self {
        Self {
            payload: Payload {
                id,
                event,
                value: Some(value),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_window_control_serialization() {
        let controls = vec![
            (WindowControl::Minimize, r#"{"type":"Minimize"}"#),
            (WindowControl::GetSize, r#"{"type":"GetSize"}"#),
            (
                WindowControl::SetSize {
                    width: 800,
                    height: 600,
                },
                r#"{"type":"SetSize","data":{"width":800,"height":600}}"#,
            ),
        ];

        for (control, expected) in controls {
            let serialized = serde_json::to_string::<WindowControl>(&control).unwrap();
            assert_eq!(serialized, expected);

            let deserialized: WindowControl = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, control);
        }
    }

    #[test]
    fn test_imsg_creation() {
        let msg = WindowControlMessage::new(1, WindowControl::Maximize);
        assert_eq!(msg.payload.id, 1);
        assert_eq!(msg.payload.event, WindowControl::Maximize);
        assert!(msg.payload.value.is_none());

        let msg_with_value =
            WindowControlMessage::with_value(2, WindowControl::Close, json!({"reason": "test"}));
        assert_eq!(msg_with_value.payload.id, 2);
        assert!(msg_with_value.payload.value.is_some());
    }

    #[test]
    fn test_payload_skip_none() {
        let payload = Payload {
            id: 1,
            event: WindowControl::Minimize,
            value: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(!json.contains("value"));

        let payload_with_value = Payload {
            id: 1,
            event: WindowControl::Minimize,
            value: Some(json!({"test": true})),
        };

        let json = serde_json::to_string(&payload_with_value).unwrap();
        assert!(json.contains("value"));
    }
}
