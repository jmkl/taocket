use global_hotkey::{
    GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyInfo {
    pub id: String,
    pub key: String,
    pub hotkey_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum HotkeyResponse {
    Registered { key: String },
    Failed { err: String },
}

pub struct TaocketHotkeyManager {
    manager: GlobalHotKeyManager,
    pub registered_hotkeys: HashMap<u32, HotKey>,
}

impl TaocketHotkeyManager {
    /// Create a new hotkey manager
    pub fn new() -> Result<Self, HotkeyError> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| HotkeyError::InitializationFailed(e.to_string()))?;

        Ok(Self {
            manager,
            registered_hotkeys: HashMap::new(),
        })
    }

    /// Register a hotkey with a client-provided ID
    pub fn register_hotkey(&mut self, key: String) -> HotkeyResponse {
        match parse_hotkey(&key) {
            Ok(ok_key) => match self.manager.register(ok_key) {
                Ok(_) => {
                    self.registered_hotkeys.insert(ok_key.id(), ok_key);
                    return HotkeyResponse::Registered { key: key };
                }
                Err(err) => {
                    let err: String = format!("Error:{err:?}");
                    return HotkeyResponse::Failed { err: err };
                }
            },
            Err(err) => {
                return HotkeyResponse::Failed {
                    err: format!("{err:?}"),
                };
            }
        }
    }
}

fn split_key_str(shortcut: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = shortcut.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '+' {
            if !current.is_empty() {
                result.push(current.clone());
                current.clear();
            }

            // Check if next char is also '+'
            if chars.peek() == Some(&'+') {
                chars.next(); // consume the second '+'
                result.push("+".to_string());
            }
        } else {
            current.push(ch);
        }
    }

    // Push the last component if any
    if !current.is_empty() {
        result.push(current);
    }

    result
}

pub(crate) fn parse_hotkey(key_string: &str) -> Result<HotKey, HotkeyError> {
    let parts: Vec<String> = split_key_str(key_string);

    if parts.is_empty() {
        return Err(HotkeyError::InvalidKeyFormat(key_string.to_string()));
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "win" | "cmd" | "meta" => modifiers |= Modifiers::SUPER,
            key => {
                if key_code.is_some() {
                    return Err(HotkeyError::InvalidKeyFormat(
                        "Multiple non-modifier keys specified".to_string(),
                    ));
                }
                key_code = Some(parse_key_code(key)?);
            }
        }
    }

    let code = key_code
        .ok_or_else(|| HotkeyError::InvalidKeyFormat("No key code specified".to_string()))?;

    Ok(HotKey::new(Some(modifiers), code))
}

pub(crate) fn parse_key_code(key: &str) -> Result<Code, HotkeyError> {
    use Code::*;
    match key.to_uppercase().as_str() {
        "BACKQUOTE" | "`" => Ok(Backquote),
        "BACKSLASH" | "\\" => Ok(Backslash),
        "BRACKETLEFT" | "[" => Ok(BracketLeft),
        "BRACKETRIGHT" | "]" => Ok(BracketRight),
        "PAUSE" | "PAUSEBREAK" => Ok(Pause),
        "COMMA" | "," => Ok(Comma),
        "DIGIT0" | "0" => Ok(Digit0),
        "DIGIT1" | "1" => Ok(Digit1),
        "DIGIT2" | "2" => Ok(Digit2),
        "DIGIT3" | "3" => Ok(Digit3),
        "DIGIT4" | "4" => Ok(Digit4),
        "DIGIT5" | "5" => Ok(Digit5),
        "DIGIT6" | "6" => Ok(Digit6),
        "DIGIT7" | "7" => Ok(Digit7),
        "DIGIT8" | "8" => Ok(Digit8),
        "DIGIT9" | "9" => Ok(Digit9),
        "EQUAL" | "=" => Ok(Equal),
        "KEYA" | "A" => Ok(KeyA),
        "KEYB" | "B" => Ok(KeyB),
        "KEYC" | "C" => Ok(KeyC),
        "KEYD" | "D" => Ok(KeyD),
        "KEYE" | "E" => Ok(KeyE),
        "KEYF" | "F" => Ok(KeyF),
        "KEYG" | "G" => Ok(KeyG),
        "KEYH" | "H" => Ok(KeyH),
        "KEYI" | "I" => Ok(KeyI),
        "KEYJ" | "J" => Ok(KeyJ),
        "KEYK" | "K" => Ok(KeyK),
        "KEYL" | "L" => Ok(KeyL),
        "KEYM" | "M" => Ok(KeyM),
        "KEYN" | "N" => Ok(KeyN),
        "KEYO" | "O" => Ok(KeyO),
        "KEYP" | "P" => Ok(KeyP),
        "KEYQ" | "Q" => Ok(KeyQ),
        "KEYR" | "R" => Ok(KeyR),
        "KEYS" | "S" => Ok(KeyS),
        "KEYT" | "T" => Ok(KeyT),
        "KEYU" | "U" => Ok(KeyU),
        "KEYV" | "V" => Ok(KeyV),
        "KEYW" | "W" => Ok(KeyW),
        "KEYX" | "X" => Ok(KeyX),
        "KEYY" | "Y" => Ok(KeyY),
        "KEYZ" | "Z" => Ok(KeyZ),
        "MINUS" | "-" => Ok(Minus),
        "PERIOD" | "." => Ok(Period),
        "QUOTE" | "'" => Ok(Quote),
        "SEMICOLON" | ";" => Ok(Semicolon),
        "SLASH" | "/" => Ok(Slash),
        "BACKSPACE" => Ok(Backspace),
        "CAPSLOCK" => Ok(CapsLock),
        "ENTER" => Ok(Enter),
        "SPACE" => Ok(Space),
        "TAB" => Ok(Tab),
        "DELETE" => Ok(Delete),
        "END" => Ok(End),
        "HOME" => Ok(Home),
        "INSERT" => Ok(Insert),
        "PAGEDOWN" => Ok(PageDown),
        "PAGEUP" => Ok(PageUp),
        "PRINTSCREEN" => Ok(PrintScreen),
        "SCROLLLOCK" => Ok(ScrollLock),
        "ARROWDOWN" | "DOWN" => Ok(ArrowDown),
        "ARROWLEFT" | "LEFT" => Ok(ArrowLeft),
        "ARROWRIGHT" | "RIGHT" => Ok(ArrowRight),
        "ARROWUP" | "UP" => Ok(ArrowUp),
        "NUMLOCK" => Ok(NumLock),
        "NUMPAD0" | "NUM0" => Ok(Numpad0),
        "NUMPAD1" | "NUM1" => Ok(Numpad1),
        "NUMPAD2" | "NUM2" => Ok(Numpad2),
        "NUMPAD3" | "NUM3" => Ok(Numpad3),
        "NUMPAD4" | "NUM4" => Ok(Numpad4),
        "NUMPAD5" | "NUM5" => Ok(Numpad5),
        "NUMPAD6" | "NUM6" => Ok(Numpad6),
        "NUMPAD7" | "NUM7" => Ok(Numpad7),
        "NUMPAD8" | "NUM8" => Ok(Numpad8),
        "NUMPAD9" | "NUM9" => Ok(Numpad9),
        "NUMPADADD" | "NUMADD" | "NUMPADPLUS" | "NUMPLUS" | "+" => Ok(NumpadAdd),
        "NUMPADDECIMAL" | "NUMDECIMAL" => Ok(NumpadDecimal),
        "NUMPADDIVIDE" | "NUMDIVIDE" => Ok(NumpadDivide),
        "NUMPADENTER" | "NUMENTER" => Ok(NumpadEnter),
        "NUMPADEQUAL" | "NUMEQUAL" => Ok(NumpadEqual),
        "NUMPADMULTIPLY" | "NUMMULTIPLY" => Ok(NumpadMultiply),
        "NUMPADSUBTRACT" | "NUMSUBTRACT" => Ok(NumpadSubtract),
        "ESCAPE" | "ESC" => Ok(Escape),
        "F1" => Ok(F1),
        "F2" => Ok(F2),
        "F3" => Ok(F3),
        "F4" => Ok(F4),
        "F5" => Ok(F5),
        "F6" => Ok(F6),
        "F7" => Ok(F7),
        "F8" => Ok(F8),
        "F9" => Ok(F9),
        "F10" => Ok(F10),
        "F11" => Ok(F11),
        "F12" => Ok(F12),
        "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => Ok(AudioVolumeDown),
        "AUDIOVOLUMEUP" | "VOLUMEUP" => Ok(AudioVolumeUp),
        "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => Ok(AudioVolumeMute),
        "MEDIAPLAY" => Ok(MediaPlay),
        "MEDIAPAUSE" => Ok(MediaPause),
        "MEDIAPLAYPAUSE" => Ok(MediaPlayPause),
        "MEDIASTOP" => Ok(MediaStop),
        "MEDIATRACKNEXT" => Ok(MediaTrackNext),
        "MEDIATRACKPREV" | "MEDIATRACKPREVIOUS" => Ok(MediaTrackPrevious),
        "F13" => Ok(F13),
        "F14" => Ok(F14),
        "F15" => Ok(F15),
        "F16" => Ok(F16),
        "F17" => Ok(F17),
        "F18" => Ok(F18),
        "F19" => Ok(F19),
        "F20" => Ok(F20),
        "F21" => Ok(F21),
        "F22" => Ok(F22),
        "F23" => Ok(F23),
        "F24" => Ok(F24),

        _ => Err(HotkeyError::UnknownKey(key.to_string())),
    }
}

#[derive(Debug, Clone)]
pub enum HotkeyError {
    InitializationFailed(String),
    InvalidKeyFormat(String),
    UnknownKey(String),
}

impl std::fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InitializationFailed(e) => {
                write!(f, "Failed to initialize hotkey manager: {}", e)
            }
            Self::InvalidKeyFormat(e) => write!(f, "Invalid key format: {}", e),
            Self::UnknownKey(k) => write!(f, "Unknown key: {}", k),
        }
    }
}

impl std::error::Error for HotkeyError {}
