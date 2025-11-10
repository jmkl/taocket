pub mod taocket_config;
pub mod taocket_context;
pub mod taocket_hotkey;
pub mod taocket_macro;
pub mod taocket_protocol;
pub mod taocket_utils;
pub mod taocket_window;
pub mod ws;
/// Trait for custom events that can be sent through the event loop
pub trait CustomEvent: Clone + Send + 'static {}
impl<T: Clone + Send + 'static> CustomEvent for T {}
