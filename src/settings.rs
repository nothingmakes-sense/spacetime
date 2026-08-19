//! Persistent game settings: keybinds, display, graphics.
//!
//! Stored as `spacetime_settings.cfg` next to the crate / cwd. The format is
//! `key=value` so we do not pull in serde. Escape always opens pause — that
//! binding is not remappable.

use std::fs;
use std::path::{Path, PathBuf};

use winit::keyboard::KeyCode;

use crate::config::{MOUSE_SENSITIVITY, WINDOW_HEIGHT, WINDOW_WIDTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Jump,
    Sprint,
    Sit,
    Interact,
    Attack,
    Inventory,
    Drop,
    Craft,
    Transfer,
    Take,
    RecipePrev,
    RecipeNext,
}

impl Action {
    pub const ALL: [Action; 16] = [
        Self::Forward,
        Self::Back,
        Self::Left,
        Self::Right,
        Self::Jump,
        Self::Sprint,
        Self::Sit,
        Self::Interact,
        Self::Attack,
        Self::Inventory,
        Self::Drop,
        Self::Craft,
        Self::Transfer,
        Self::Take,
        Self::RecipePrev,
        Self::RecipeNext,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Back => "back",
            Self::Left => "left",
            Self::Right => "right",
            Self::Jump => "jump",
            Self::Sprint => "sprint",
            Self::Sit => "sit",
            Self::Interact => "interact",
            Self::Attack => "attack",
            Self::Inventory => "inventory",
            Self::Drop => "drop",
            Self::Craft => "craft",
            Self::Transfer => "transfer",
            Self::Take => "take",
            Self::RecipePrev => "recipe_prev",
            Self::RecipeNext => "recipe_next",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "FORWARD",
            Self::Back => "BACK",
            Self::Left => "STRAFE LEFT",
            Self::Right => "STRAFE RIGHT",
            Self::Jump => "JUMP",
            Self::Sprint => "SPRINT",
            Self::Sit => "SIT",
            Self::Interact => "INTERACT",
            Self::Attack => "ATTACK",
            Self::Inventory => "INVENTORY",
            Self::Drop => "DROP",
            Self::Craft => "CRAFT",
            Self::Transfer => "DEPOSIT",
            Self::Take => "TAKE",
            Self::RecipePrev => "PREV RECIPE",
            Self::RecipeNext => "NEXT RECIPE",
        }
    }

    pub fn default_key(self) -> KeyCode {
        match self {
            Self::Forward => KeyCode::KeyW,
            Self::Back => KeyCode::KeyS,
            Self::Left => KeyCode::KeyA,
            Self::Right => KeyCode::KeyD,
            Self::Jump => KeyCode::Space,
            Self::Sprint => KeyCode::ShiftLeft,
            Self::Sit => KeyCode::KeyC,
            Self::Interact => KeyCode::KeyE,
            Self::Attack => KeyCode::KeyQ,
            Self::Inventory => KeyCode::Tab,
            Self::Drop => KeyCode::KeyG,
            Self::Craft => KeyCode::KeyR,
            Self::Transfer => KeyCode::KeyT,
            Self::Take => KeyCode::KeyY,
            Self::RecipePrev => KeyCode::BracketLeft,
            Self::RecipeNext => KeyCode::BracketRight,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

impl Quality {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Low,
            1 => Self::Medium,
            3 => Self::Ultra,
            _ => Self::High,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Ultra => 3,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Ultra => "ULTRA",
        }
    }

    /// Ambient / specular / shininess fed to the Phong light UBO.
    pub fn light(self) -> (f32, f32, f32) {
        match self {
            Self::Low => (0.32, 0.18, 8.0),
            Self::Medium => (0.26, 0.28, 16.0),
            Self::High => (0.22, 0.35, 28.0),
            Self::Ultra => (0.18, 0.48, 48.0),
        }
    }

    pub fn anisotropy(self) -> f32 {
        match self {
            Self::Low => 1.0,
            Self::Medium => 4.0,
            Self::High => 8.0,
            Self::Ultra => 16.0,
        }
    }
}

pub const RESOLUTIONS: &[(u32, u32, &str)] = &[
    (1280, 720, "1280 X 720"),
    (1600, 900, "1600 X 900"),
    (1920, 1080, "1920 X 1080"),
    (2560, 1440, "2560 X 1440"),
    (3840, 2160, "3840 X 2160"),
];

#[derive(Clone, Debug)]
pub struct Settings {
    pub vsync: bool,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
    pub quality: Quality,
    /// 50..=150, 100 = default.
    pub brightness: u8,
    /// 50..=200, 100 = default [`MOUSE_SENSITIVITY`].
    pub mouse_sens: u8,
    pub binds: [KeyCode; Action::ALL.len()],
}

impl Default for Settings {
    fn default() -> Self {
        let mut binds = [KeyCode::KeyW; Action::ALL.len()];
        for (i, a) in Action::ALL.iter().enumerate() {
            binds[i] = a.default_key();
        }
        Self {
            vsync: true,
            fullscreen: false,
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
            quality: Quality::High,
            brightness: 100,
            mouse_sens: 100,
            binds,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let mut s = Self::default();
        let Some(text) = read_cfg() else {
            return s;
        };
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let k = k.trim();
            let v = v.trim();
            match k {
                "vsync" => s.vsync = v == "1" || v.eq_ignore_ascii_case("true"),
                "fullscreen" => s.fullscreen = v == "1" || v.eq_ignore_ascii_case("true"),
                "width" => s.width = v.parse().unwrap_or(s.width),
                "height" => s.height = v.parse().unwrap_or(s.height),
                "quality" => s.quality = Quality::from_u8(v.parse().unwrap_or(2)),
                "brightness" => s.brightness = v.parse::<u8>().unwrap_or(100).clamp(50, 150),
                "mouse_sens" => s.mouse_sens = v.parse::<u8>().unwrap_or(100).clamp(50, 200),
                other if other.starts_with("bind.") => {
                    let id = &other[5..];
                    if let Some((i, _)) = Action::ALL.iter().enumerate().find(|(_, a)| a.id() == id) {
                        if let Some(code) = keycode_from_name(v) {
                            s.binds[i] = code;
                        }
                    }
                }
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) {
        let mut out = String::from("# spacetime settings — edit or use the pause menu\n");
        out.push_str(&format!("vsync={}\n", self.vsync as u8));
        out.push_str(&format!("fullscreen={}\n", self.fullscreen as u8));
        out.push_str(&format!("width={}\n", self.width));
        out.push_str(&format!("height={}\n", self.height));
        out.push_str(&format!("quality={}\n", self.quality.as_u8()));
        out.push_str(&format!("brightness={}\n", self.brightness));
        out.push_str(&format!("mouse_sens={}\n", self.mouse_sens));
        for (i, a) in Action::ALL.iter().enumerate() {
            out.push_str(&format!("bind.{}={}\n", a.id(), keycode_token(self.binds[i])));
        }
        let path = cfg_path();
        if let Err(e) = fs::write(&path, out) {
            log::warn!("could not write {}: {e}", path.display());
        } else {
            log::info!("saved settings to {}", path.display());
        }
    }

    pub fn key(&self, action: Action) -> KeyCode {
        self.binds[action as usize]
    }

    pub fn set_key(&mut self, action: Action, code: KeyCode) {
        // Drop duplicates so two actions cannot share a key.
        for slot in &mut self.binds {
            if *slot == code {
                *slot = KeyCode::F24;
            }
        }
        self.binds[action as usize] = code;
    }

    pub fn matches(&self, action: Action, code: KeyCode) -> bool {
        self.key(action) == code
            || (action == Action::Sprint && matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight))
            || (action == Action::Inventory && matches!(code, KeyCode::Tab | KeyCode::KeyI) && self.key(action) == KeyCode::Tab)
            || (action == Action::Sit && code == KeyCode::KeyF && self.key(action) == KeyCode::KeyC)
    }

    pub fn mouse_sensitivity(&self) -> f32 {
        MOUSE_SENSITIVITY * (self.mouse_sens as f32 / 100.0)
    }

    pub fn brightness_mul(&self) -> f32 {
        self.brightness as f32 / 100.0
    }
}

fn cfg_path() -> PathBuf {
    let cwd = Path::new("spacetime_settings.cfg");
    if cwd.exists() {
        return cwd.to_path_buf();
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("spacetime_settings.cfg")
}

fn read_cfg() -> Option<String> {
    let a = Path::new("spacetime_settings.cfg");
    if let Ok(s) = fs::read_to_string(a) {
        return Some(s);
    }
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("spacetime_settings.cfg")).ok()
}

pub fn keycode_from_name(name: &str) -> Option<KeyCode> {
    Some(match name {
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "Space" => KeyCode::Space,
        "Tab" => KeyCode::Tab,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "AltLeft" => KeyCode::AltLeft,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Comma" => KeyCode::Comma,
        "Period" => KeyCode::Period,
        "Slash" => KeyCode::Slash,
        "Semicolon" => KeyCode::Semicolon,
        "Quote" => KeyCode::Quote,
        "Backquote" => KeyCode::Backquote,
        "Minus" => KeyCode::Minus,
        "Equal" => KeyCode::Equal,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "Enter" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        _ => return None,
    })
}

pub fn keycode_token(code: KeyCode) -> &'static str {
    match code {
        KeyCode::KeyA => "KeyA",
        KeyCode::KeyB => "KeyB",
        KeyCode::KeyC => "KeyC",
        KeyCode::KeyD => "KeyD",
        KeyCode::KeyE => "KeyE",
        KeyCode::KeyF => "KeyF",
        KeyCode::KeyG => "KeyG",
        KeyCode::KeyH => "KeyH",
        KeyCode::KeyI => "KeyI",
        KeyCode::KeyJ => "KeyJ",
        KeyCode::KeyK => "KeyK",
        KeyCode::KeyL => "KeyL",
        KeyCode::KeyM => "KeyM",
        KeyCode::KeyN => "KeyN",
        KeyCode::KeyO => "KeyO",
        KeyCode::KeyP => "KeyP",
        KeyCode::KeyQ => "KeyQ",
        KeyCode::KeyR => "KeyR",
        KeyCode::KeyS => "KeyS",
        KeyCode::KeyT => "KeyT",
        KeyCode::KeyU => "KeyU",
        KeyCode::KeyV => "KeyV",
        KeyCode::KeyW => "KeyW",
        KeyCode::KeyX => "KeyX",
        KeyCode::KeyY => "KeyY",
        KeyCode::KeyZ => "KeyZ",
        KeyCode::Digit0 => "Digit0",
        KeyCode::Digit1 => "Digit1",
        KeyCode::Digit2 => "Digit2",
        KeyCode::Digit3 => "Digit3",
        KeyCode::Digit4 => "Digit4",
        KeyCode::Digit5 => "Digit5",
        KeyCode::Digit6 => "Digit6",
        KeyCode::Digit7 => "Digit7",
        KeyCode::Digit8 => "Digit8",
        KeyCode::Digit9 => "Digit9",
        KeyCode::Space => "Space",
        KeyCode::Tab => "Tab",
        KeyCode::ShiftLeft => "ShiftLeft",
        KeyCode::ShiftRight => "ShiftRight",
        KeyCode::ControlLeft => "ControlLeft",
        KeyCode::AltLeft => "AltLeft",
        KeyCode::BracketLeft => "BracketLeft",
        KeyCode::BracketRight => "BracketRight",
        KeyCode::Comma => "Comma",
        KeyCode::Period => "Period",
        KeyCode::Slash => "Slash",
        KeyCode::Semicolon => "Semicolon",
        KeyCode::Quote => "Quote",
        KeyCode::Backquote => "Backquote",
        KeyCode::Minus => "Minus",
        KeyCode::Equal => "Equal",
        KeyCode::ArrowLeft => "ArrowLeft",
        KeyCode::ArrowRight => "ArrowRight",
        KeyCode::ArrowUp => "ArrowUp",
        KeyCode::ArrowDown => "ArrowDown",
        KeyCode::Enter => "Enter",
        KeyCode::Backspace => "Backspace",
        KeyCode::F24 => "UNSET",
        _ => "Key",
    }
}

/// Short label for the pause menu (fits the bitmap font).
pub fn keycode_label(code: KeyCode) -> &'static str {
    match code {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::Space => "SPACE",
        KeyCode::Tab => "TAB",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "SHIFT",
        KeyCode::ControlLeft | KeyCode::ControlRight => "CTRL",
        KeyCode::AltLeft | KeyCode::AltRight => "ALT",
        KeyCode::BracketLeft => "[",
        KeyCode::BracketRight => "]",
        KeyCode::Comma => ",",
        KeyCode::Period => ".",
        KeyCode::Slash => "/",
        KeyCode::Minus => "-",
        KeyCode::Equal => "=",
        KeyCode::ArrowLeft => "LEFT",
        KeyCode::ArrowRight => "RIGHT",
        KeyCode::ArrowUp => "UP",
        KeyCode::ArrowDown => "DOWN",
        KeyCode::Enter => "ENTER",
        KeyCode::Backspace => "BKSP",
        KeyCode::F24 => "-",
        _ => "KEY",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_tokens() {
        for a in Action::ALL {
            let tok = keycode_token(a.default_key());
            assert_eq!(keycode_from_name(tok), Some(a.default_key()));
        }
    }
}
