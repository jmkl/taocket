use serde::Serialize;
use wry::WebView;

#[macro_export]
macro_rules! callback {
    ($webview:expr, $event_name:expr, $value:expr) => {{
        if let Ok(json) = serde_json::to_string(&$value) {
            let script = format!(
                r#"
                (() => {{
                    const event = new CustomEvent("{}", {{
                        detail: {}
                    }});
                    window.dispatchEvent(event);
                }})();
                "#,
                $event_name, json
            );

            if let Err(e) = $webview.evaluate_script(&script) {
                eprintln!("Failed to emit '{}' event: {}", $event_name, e);
            }
        }
    }};
}

#[macro_export]
macro_rules! emit_js {
    ($webview:expr,$value:expr) => {
        $webview.evaluate_script(&value);
    };
}
#[macro_export]
macro_rules! create_struct {
    ($name:ident) => {
        struct $name {
            value: i32,
        }
    };
}
