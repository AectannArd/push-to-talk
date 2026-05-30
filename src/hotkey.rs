/// Parsed hotkey: the trigger key + which modifiers must be held.
pub struct ParsedHotkey {
    pub key: rdev::Key,
    pub needs_ctrl: bool,
    pub needs_shift: bool,
    pub needs_alt: bool,
    pub needs_win: bool,
}

/// Parse a hotkey string like `"Ctrl+Shift+T"` into its parts.
///
/// Supported modifiers: `Ctrl`, `Shift`, `Alt`, `Win`.
/// The last `+`-separated token is the key name (e.g. `T`, `Space`, `F1`, `Return`).
pub fn parse_hotkey(raw: &str) -> Result<ParsedHotkey, String> {
    let mut needs_ctrl = false;
    let mut needs_shift = false;
    let mut needs_alt = false;
    let mut needs_win = false;

    let parts: Vec<&str> = raw.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return Err("empty hotkey string".into());
    }

    // All but the last part are modifiers
    for part in &parts[..parts.len() - 1] {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => needs_ctrl = true,
            "shift" => needs_shift = true,
            "alt" => needs_alt = true,
            "win" | "windows" | "meta" | "super" => needs_win = true,
            other => return Err(format!("unknown modifier: {other}")),
        }
    }

    let key_name = parts[parts.len() - 1];
    let key = parse_key(key_name)?;

    Ok(ParsedHotkey {
        key,
        needs_ctrl,
        needs_shift,
        needs_alt,
        needs_win,
    })
}

/// Map a key name string to an `rdev::Key` variant.
fn parse_key(name: &str) -> Result<rdev::Key, String> {
    match name.to_lowercase().as_str() {
        // Letters
        "a" => Ok(rdev::Key::KeyA),
        "b" => Ok(rdev::Key::KeyB),
        "c" => Ok(rdev::Key::KeyC),
        "d" => Ok(rdev::Key::KeyD),
        "e" => Ok(rdev::Key::KeyE),
        "f" => Ok(rdev::Key::KeyF),
        "g" => Ok(rdev::Key::KeyG),
        "h" => Ok(rdev::Key::KeyH),
        "i" => Ok(rdev::Key::KeyI),
        "j" => Ok(rdev::Key::KeyJ),
        "k" => Ok(rdev::Key::KeyK),
        "l" => Ok(rdev::Key::KeyL),
        "m" => Ok(rdev::Key::KeyM),
        "n" => Ok(rdev::Key::KeyN),
        "o" => Ok(rdev::Key::KeyO),
        "p" => Ok(rdev::Key::KeyP),
        "q" => Ok(rdev::Key::KeyQ),
        "r" => Ok(rdev::Key::KeyR),
        "s" => Ok(rdev::Key::KeyS),
        "t" => Ok(rdev::Key::KeyT),
        "u" => Ok(rdev::Key::KeyU),
        "v" => Ok(rdev::Key::KeyV),
        "w" => Ok(rdev::Key::KeyW),
        "x" => Ok(rdev::Key::KeyX),
        "y" => Ok(rdev::Key::KeyY),
        "z" => Ok(rdev::Key::KeyZ),

        // Digits
        "0" => Ok(rdev::Key::Num0),
        "1" => Ok(rdev::Key::Num1),
        "2" => Ok(rdev::Key::Num2),
        "3" => Ok(rdev::Key::Num3),
        "4" => Ok(rdev::Key::Num4),
        "5" => Ok(rdev::Key::Num5),
        "6" => Ok(rdev::Key::Num6),
        "7" => Ok(rdev::Key::Num7),
        "8" => Ok(rdev::Key::Num8),
        "9" => Ok(rdev::Key::Num9),

        // Function keys
        "f1" => Ok(rdev::Key::F1),
        "f2" => Ok(rdev::Key::F2),
        "f3" => Ok(rdev::Key::F3),
        "f4" => Ok(rdev::Key::F4),
        "f5" => Ok(rdev::Key::F5),
        "f6" => Ok(rdev::Key::F6),
        "f7" => Ok(rdev::Key::F7),
        "f8" => Ok(rdev::Key::F8),
        "f9" => Ok(rdev::Key::F9),
        "f10" => Ok(rdev::Key::F10),
        "f11" => Ok(rdev::Key::F11),
        "f12" => Ok(rdev::Key::F12),

        // Special keys
        "space" => Ok(rdev::Key::Space),
        "return" | "enter" => Ok(rdev::Key::Return),
        "escape" | "esc" => Ok(rdev::Key::Escape),
        "tab" => Ok(rdev::Key::Tab),
        "backspace" | "back" => Ok(rdev::Key::Backspace),
        "delete" | "del" => Ok(rdev::Key::Delete),
        "insert" | "ins" => Ok(rdev::Key::Insert),
        "home" => Ok(rdev::Key::Home),
        "end" => Ok(rdev::Key::End),
        "pageup" | "pgup" => Ok(rdev::Key::PageUp),
        "pagedown" | "pgdn" => Ok(rdev::Key::PageDown),
        "up" => Ok(rdev::Key::UpArrow),
        "down" => Ok(rdev::Key::DownArrow),
        "left" => Ok(rdev::Key::LeftArrow),
        "right" => Ok(rdev::Key::RightArrow),
        "capslock" | "caps" => Ok(rdev::Key::CapsLock),
        "printscreen" | "prtsc" => Ok(rdev::Key::PrintScreen),
        "pause" | "break" => Ok(rdev::Key::Pause),

        // Numpad
        "num0" | "numpad0" => Ok(rdev::Key::Num0),
        "num1" | "numpad1" => Ok(rdev::Key::Num1),
        "num2" | "numpad2" => Ok(rdev::Key::Num2),
        "num3" | "numpad3" => Ok(rdev::Key::Num3),
        "num4" | "numpad4" => Ok(rdev::Key::Num4),
        "num5" | "numpad5" => Ok(rdev::Key::Num5),
        "num6" | "numpad6" => Ok(rdev::Key::Num6),
        "num7" | "numpad7" => Ok(rdev::Key::Num7),
        "num8" | "numpad8" => Ok(rdev::Key::Num8),
        "num9" | "numpad9" => Ok(rdev::Key::Num9),

        _ => Err(format!("unknown key: {name}")),
    }
}
