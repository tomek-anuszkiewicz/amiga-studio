//! Physical host key positions to HRM chapter 8 raw Amiga keycodes.

use keyboard::{SCANCODE_CAPS_LOCK, SCANCODE_CTRL, SCANCODE_L_AMIGA, SCANCODE_R_AMIGA};

const SCANCODE_BACKQUOTE: u8 = 0x00;
const SCANCODE_DIGIT1: u8 = 0x01;
const SCANCODE_DIGIT2: u8 = 0x02;
const SCANCODE_DIGIT3: u8 = 0x03;
const SCANCODE_DIGIT4: u8 = 0x04;
const SCANCODE_DIGIT5: u8 = 0x05;
const SCANCODE_DIGIT6: u8 = 0x06;
const SCANCODE_DIGIT7: u8 = 0x07;
const SCANCODE_DIGIT8: u8 = 0x08;
const SCANCODE_DIGIT9: u8 = 0x09;
const SCANCODE_DIGIT0: u8 = 0x0a;
const SCANCODE_MINUS: u8 = 0x0b;
const SCANCODE_EQUAL: u8 = 0x0c;
const SCANCODE_BACKSLASH: u8 = 0x0d;
pub(crate) const SCANCODE_NUMPAD0: u8 = 0x0f;
const SCANCODE_KEY_Q: u8 = 0x10;
pub(crate) const SCANCODE_KEY_W: u8 = 0x11;
const SCANCODE_KEY_E: u8 = 0x12;
const SCANCODE_KEY_R: u8 = 0x13;
const SCANCODE_KEY_T: u8 = 0x14;
const SCANCODE_KEY_Y: u8 = 0x15;
const SCANCODE_KEY_U: u8 = 0x16;
const SCANCODE_KEY_I: u8 = 0x17;
const SCANCODE_KEY_O: u8 = 0x18;
const SCANCODE_KEY_P: u8 = 0x19;
const SCANCODE_BRACKET_LEFT: u8 = 0x1a;
const SCANCODE_BRACKET_RIGHT: u8 = 0x1b;
const SCANCODE_NUMPAD1: u8 = 0x1d;
const SCANCODE_NUMPAD2: u8 = 0x1e;
const SCANCODE_NUMPAD3: u8 = 0x1f;
pub(crate) const SCANCODE_KEY_A: u8 = 0x20;
pub(crate) const SCANCODE_KEY_S: u8 = 0x21;
pub(crate) const SCANCODE_KEY_D: u8 = 0x22;
const SCANCODE_KEY_F: u8 = 0x23;
const SCANCODE_KEY_G: u8 = 0x24;
const SCANCODE_KEY_H: u8 = 0x25;
const SCANCODE_KEY_J: u8 = 0x26;
const SCANCODE_KEY_K: u8 = 0x27;
const SCANCODE_KEY_L: u8 = 0x28;
const SCANCODE_SEMICOLON: u8 = 0x29;
const SCANCODE_QUOTE: u8 = 0x2a;
const SCANCODE_INTL_BACKSLASH: u8 = 0x30;
const SCANCODE_NUMPAD4: u8 = 0x2d;
const SCANCODE_NUMPAD5: u8 = 0x2e;
const SCANCODE_NUMPAD6: u8 = 0x2f;
const SCANCODE_KEY_Z: u8 = 0x31;
const SCANCODE_KEY_X: u8 = 0x32;
pub(crate) const SCANCODE_KEY_C: u8 = 0x33;
const SCANCODE_KEY_V: u8 = 0x34;
const SCANCODE_KEY_B: u8 = 0x35;
const SCANCODE_KEY_N: u8 = 0x36;
const SCANCODE_KEY_M: u8 = 0x37;
const SCANCODE_COMMA: u8 = 0x38;
const SCANCODE_PERIOD: u8 = 0x39;
const SCANCODE_SLASH: u8 = 0x3a;
pub(crate) const SCANCODE_NUMPAD_DECIMAL: u8 = 0x3c;
const SCANCODE_NUMPAD7: u8 = 0x3d;
const SCANCODE_NUMPAD8: u8 = 0x3e;
const SCANCODE_NUMPAD9: u8 = 0x3f;
pub(crate) const SCANCODE_SPACE: u8 = 0x40;
const SCANCODE_BACKSPACE: u8 = 0x41;
const SCANCODE_TAB: u8 = 0x42;
const SCANCODE_NUMPAD_ENTER: u8 = 0x43;
const SCANCODE_ENTER: u8 = 0x44;
const SCANCODE_ESCAPE: u8 = 0x45;
const SCANCODE_DELETE: u8 = 0x46;
const SCANCODE_NUMPAD_SUBTRACT: u8 = 0x4a;
pub(crate) const SCANCODE_ARROW_UP: u8 = 0x4c;
pub(crate) const SCANCODE_ARROW_DOWN: u8 = 0x4d;
pub(crate) const SCANCODE_ARROW_RIGHT: u8 = 0x4e;
pub(crate) const SCANCODE_ARROW_LEFT: u8 = 0x4f;
const SCANCODE_F1: u8 = 0x50;
const SCANCODE_F2: u8 = 0x51;
const SCANCODE_F3: u8 = 0x52;
const SCANCODE_F4: u8 = 0x53;
const SCANCODE_F5: u8 = 0x54;
const SCANCODE_F6: u8 = 0x55;
const SCANCODE_F7: u8 = 0x56;
const SCANCODE_F8: u8 = 0x57;
const SCANCODE_F9: u8 = 0x58;
const SCANCODE_F10: u8 = 0x59;
const SCANCODE_NUMPAD_DIVIDE: u8 = 0x5c;
const SCANCODE_NUMPAD_MULTIPLY: u8 = 0x5d;
const SCANCODE_NUMPAD_ADD: u8 = 0x5e;
const SCANCODE_INSERT: u8 = 0x5f;
const SCANCODE_SHIFT_LEFT: u8 = 0x60;
const SCANCODE_SHIFT_RIGHT: u8 = 0x61;
const SCANCODE_ALT_LEFT: u8 = 0x64;
const SCANCODE_ALT_RIGHT: u8 = 0x65;

pub(crate) const MAX_AMIGA_KEYCODE: u8 = 0x7f;

pub fn amiga_key_code(code: &str) -> Option<u8> {
    Some(match code {
        "Backquote" => SCANCODE_BACKQUOTE,
        "Digit1" => SCANCODE_DIGIT1,
        "Digit2" => SCANCODE_DIGIT2,
        "Digit3" => SCANCODE_DIGIT3,
        "Digit4" => SCANCODE_DIGIT4,
        "Digit5" => SCANCODE_DIGIT5,
        "Digit6" => SCANCODE_DIGIT6,
        "Digit7" => SCANCODE_DIGIT7,
        "Digit8" => SCANCODE_DIGIT8,
        "Digit9" => SCANCODE_DIGIT9,
        "Digit0" => SCANCODE_DIGIT0,
        "Minus" => SCANCODE_MINUS,
        "Equal" => SCANCODE_EQUAL,
        "Backslash" => SCANCODE_BACKSLASH,
        "Numpad0" => SCANCODE_NUMPAD0,
        "KeyQ" => SCANCODE_KEY_Q,
        "KeyW" => SCANCODE_KEY_W,
        "KeyE" => SCANCODE_KEY_E,
        "KeyR" => SCANCODE_KEY_R,
        "KeyT" => SCANCODE_KEY_T,
        "KeyY" => SCANCODE_KEY_Y,
        "KeyU" => SCANCODE_KEY_U,
        "KeyI" => SCANCODE_KEY_I,
        "KeyO" => SCANCODE_KEY_O,
        "KeyP" => SCANCODE_KEY_P,
        "BracketLeft" => SCANCODE_BRACKET_LEFT,
        "BracketRight" => SCANCODE_BRACKET_RIGHT,
        "Numpad1" => SCANCODE_NUMPAD1,
        "Numpad2" => SCANCODE_NUMPAD2,
        "Numpad3" => SCANCODE_NUMPAD3,
        "KeyA" => SCANCODE_KEY_A,
        "KeyS" => SCANCODE_KEY_S,
        "KeyD" => SCANCODE_KEY_D,
        "KeyF" => SCANCODE_KEY_F,
        "KeyG" => SCANCODE_KEY_G,
        "KeyH" => SCANCODE_KEY_H,
        "KeyJ" => SCANCODE_KEY_J,
        "KeyK" => SCANCODE_KEY_K,
        "KeyL" => SCANCODE_KEY_L,
        "Semicolon" => SCANCODE_SEMICOLON,
        "Quote" => SCANCODE_QUOTE,
        "IntlBackslash" => SCANCODE_INTL_BACKSLASH,
        "Numpad4" => SCANCODE_NUMPAD4,
        "Numpad5" => SCANCODE_NUMPAD5,
        "Numpad6" => SCANCODE_NUMPAD6,
        "KeyZ" => SCANCODE_KEY_Z,
        "KeyX" => SCANCODE_KEY_X,
        "KeyC" => SCANCODE_KEY_C,
        "KeyV" => SCANCODE_KEY_V,
        "KeyB" => SCANCODE_KEY_B,
        "KeyN" => SCANCODE_KEY_N,
        "KeyM" => SCANCODE_KEY_M,
        "Comma" => SCANCODE_COMMA,
        "Period" => SCANCODE_PERIOD,
        "Slash" => SCANCODE_SLASH,
        "NumpadDecimal" => SCANCODE_NUMPAD_DECIMAL,
        "Numpad7" => SCANCODE_NUMPAD7,
        "Numpad8" => SCANCODE_NUMPAD8,
        "Numpad9" => SCANCODE_NUMPAD9,
        "Space" => SCANCODE_SPACE,
        "Backspace" => SCANCODE_BACKSPACE,
        "Tab" => SCANCODE_TAB,
        "NumpadEnter" => SCANCODE_NUMPAD_ENTER,
        "Enter" => SCANCODE_ENTER,
        "Escape" => SCANCODE_ESCAPE,
        "Delete" => SCANCODE_DELETE,
        "NumpadSubtract" => SCANCODE_NUMPAD_SUBTRACT,
        "ArrowUp" => SCANCODE_ARROW_UP,
        "ArrowDown" => SCANCODE_ARROW_DOWN,
        "ArrowRight" => SCANCODE_ARROW_RIGHT,
        "ArrowLeft" => SCANCODE_ARROW_LEFT,
        "F1" => SCANCODE_F1,
        "F2" => SCANCODE_F2,
        "F3" => SCANCODE_F3,
        "F4" => SCANCODE_F4,
        "F5" => SCANCODE_F5,
        "F6" => SCANCODE_F6,
        "F7" => SCANCODE_F7,
        "F8" => SCANCODE_F8,
        "F9" => SCANCODE_F9,
        "F10" => SCANCODE_F10,
        "NumpadDivide" => SCANCODE_NUMPAD_DIVIDE,
        "NumpadMultiply" => SCANCODE_NUMPAD_MULTIPLY,
        "NumpadAdd" => SCANCODE_NUMPAD_ADD,
        "Insert" => SCANCODE_INSERT,
        "ShiftLeft" => SCANCODE_SHIFT_LEFT,
        "ShiftRight" => SCANCODE_SHIFT_RIGHT,
        "CapsLock" => SCANCODE_CAPS_LOCK,
        "ControlLeft" | "ControlRight" => SCANCODE_CTRL,
        "AltLeft" => SCANCODE_ALT_LEFT,
        "AltRight" => SCANCODE_ALT_RIGHT,
        "MetaLeft" => SCANCODE_L_AMIGA,
        "MetaRight" => SCANCODE_R_AMIGA,
        _ => return None,
    })
}

/// Frontend bindings name physical key positions, excluding derived text commands.
pub fn frontend_key(name: &str) -> Option<egui::Key> {
    use egui::Key;
    let key = Key::from_name(name)?;
    match key {
        Key::Copy
        | Key::Cut
        | Key::Paste
        | Key::Colon
        | Key::Plus
        | Key::Pipe
        | Key::Questionmark
        | Key::Exclamationmark
        | Key::OpenCurlyBracket
        | Key::CloseCurlyBracket => None,
        _ => Some(key),
    }
}

/// Normalize egui binding names and DOM/native positions through the same codes.
pub fn frontend_key_code(name: &str) -> Option<u8> {
    use egui::Key;
    let key = frontend_key(name)?;
    amiga_key_code(match key {
        Key::A => "KeyA",
        Key::B => "KeyB",
        Key::C => "KeyC",
        Key::D => "KeyD",
        Key::E => "KeyE",
        Key::F => "KeyF",
        Key::G => "KeyG",
        Key::H => "KeyH",
        Key::I => "KeyI",
        Key::J => "KeyJ",
        Key::K => "KeyK",
        Key::L => "KeyL",
        Key::M => "KeyM",
        Key::N => "KeyN",
        Key::O => "KeyO",
        Key::P => "KeyP",
        Key::Q => "KeyQ",
        Key::R => "KeyR",
        Key::S => "KeyS",
        Key::T => "KeyT",
        Key::U => "KeyU",
        Key::V => "KeyV",
        Key::W => "KeyW",
        Key::X => "KeyX",
        Key::Y => "KeyY",
        Key::Z => "KeyZ",
        Key::Num0 => "Digit0",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",
        Key::OpenBracket => "BracketLeft",
        Key::CloseBracket => "BracketRight",
        Key::Backtick => "Backquote",
        Key::Equals => "Equal",
        Key::ArrowUp => "ArrowUp",
        Key::ArrowDown => "ArrowDown",
        Key::ArrowLeft => "ArrowLeft",
        Key::ArrowRight => "ArrowRight",
        _ => key.name(),
    })
}
