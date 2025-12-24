use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::Mutex;

pub struct ImeState {
    pub client_id: i32,
    pub text: String,
}

pub static IME_STATE: Lazy<Mutex<Option<ImeState>>> = Lazy::new(|| Mutex::new(None));

pub fn handle_message(message: &crate::PlatformMessage) -> bool {
    let channel_str = &message.channel;

    // See https://api.flutter.dev/flutter/services/SystemChannels/textInput-constant.html.

    if channel_str == "flutter/textinput" {
        let slice = &message.message;
        if let Ok(Value::Object(map)) = serde_json::from_slice(slice) {
            if let Some(Value::String(method)) = map.get("method") {
                match method.as_str() {
                    "TextInput.setClient" => {
                        if let Some(Value::Array(args)) = map.get("args") {
                            if let Some(id) = args.get(0).and_then(|v| v.as_i64()) {
                                *IME_STATE.lock().unwrap() = Some(ImeState {
                                    client_id: id as i32,
                                    text: String::new(),
                                });
                            }
                        }
                    }
                    "TextInput.setEditingState" => {
                        if let Some(Value::Array(args)) = map.get("args") {
                            if let Some(Value::Object(state_map)) = args.get(0) {
                                if let Some(text) = state_map.get("text").and_then(|v| v.as_str()) {
                                    if let Some(state) = IME_STATE.lock().unwrap().as_mut() {
                                        state.text = text.to_string();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    false
}
