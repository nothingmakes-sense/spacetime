//! Keyboard / mouse state.
//!
//! **In:** winit key/mouse events. **Out:** held flags + rising-edge
//! [`InputEdges`] consumed once per physics tick. Mouse pixels are stored
//! raw; `ItemUi::set_mouse_pixels` converts them to HUD NDC.

use winit::keyboard::KeyCode;

use crate::settings::{Action, Settings};

#[derive(Default, Debug, Clone)]
pub struct InputState {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sprint: bool,
    pub mouse_captured: bool,
    pub sit: bool,
    pub interact: bool,
    pub attack: bool,
    pub inventory: bool,
    pub drop: bool,
    pub craft: bool,
    pub transfer: bool,
    pub take: bool,
    pub recipe_prev: bool,
    pub recipe_next: bool,
    pub cursor_left: bool,
    pub cursor_right: bool,
    pub cursor_up: bool,
    pub cursor_down: bool,
    pub hotbar: Option<usize>,
    pub wheel: i32,
    /// Last cursor position in window pixels (origin top-left).
    pub mouse_x: f32,
    pub mouse_y: f32,
    sit_was: bool,
    interact_was: bool,
    attack_was: bool,
    inventory_was: bool,
    drop_was: bool,
    craft_was: bool,
    transfer_was: bool,
    take_was: bool,
    recipe_prev_was: bool,
    recipe_next_was: bool,
    cursor_left_was: bool,
    cursor_right_was: bool,
    cursor_up_was: bool,
    cursor_down_was: bool,
    debug_was: bool,
    pub debug: bool,
    lmb: bool,
    rmb: bool,
    lmb_was: bool,
    rmb_was: bool,
}

impl InputState {
    pub fn handle_key(&mut self, settings: &Settings, code: KeyCode, pressed: bool) {
        if settings.matches(Action::Forward, code) {
            self.forward = pressed;
        }
        if settings.matches(Action::Back, code) {
            self.back = pressed;
        }
        if settings.matches(Action::Left, code) {
            self.left = pressed;
        }
        if settings.matches(Action::Right, code) {
            self.right = pressed;
        }
        if settings.matches(Action::Jump, code) {
            self.jump = pressed;
        }
        if settings.matches(Action::Sprint, code) {
            self.sprint = pressed;
        }
        if settings.matches(Action::Sit, code) {
            self.sit = pressed;
        }
        if settings.matches(Action::Interact, code) {
            self.interact = pressed;
        }
        if settings.matches(Action::Attack, code) {
            self.attack = pressed;
        }
        if settings.matches(Action::Inventory, code) {
            self.inventory = pressed;
        }
        if settings.matches(Action::Drop, code) {
            self.drop = pressed;
        }
        if settings.matches(Action::Craft, code) {
            self.craft = pressed;
        }
        if settings.matches(Action::Transfer, code) {
            self.transfer = pressed;
        }
        if settings.matches(Action::Take, code) {
            self.take = pressed;
        }
        if settings.matches(Action::RecipePrev, code) {
            self.recipe_prev = pressed;
        }
        if settings.matches(Action::RecipeNext, code) {
            self.recipe_next = pressed;
        }
        match code {
            KeyCode::F3 => self.debug = pressed,
            KeyCode::ArrowLeft => self.cursor_left = pressed,
            KeyCode::ArrowRight => self.cursor_right = pressed,
            KeyCode::ArrowUp => self.cursor_up = pressed,
            KeyCode::ArrowDown => self.cursor_down = pressed,
            KeyCode::Digit1 => self.hotbar = pressed.then_some(0),
            KeyCode::Digit2 => self.hotbar = pressed.then_some(1),
            KeyCode::Digit3 => self.hotbar = pressed.then_some(2),
            KeyCode::Digit4 => self.hotbar = pressed.then_some(3),
            KeyCode::Digit5 => self.hotbar = pressed.then_some(4),
            KeyCode::Digit6 => self.hotbar = pressed.then_some(5),
            KeyCode::Digit7 => self.hotbar = pressed.then_some(6),
            KeyCode::Digit8 => self.hotbar = pressed.then_some(7),
            KeyCode::Digit9 => self.hotbar = pressed.then_some(8),
            _ => {}
        }
    }

    pub fn set_attack(&mut self, pressed: bool) {
        self.attack = pressed;
    }

    pub fn set_lmb(&mut self, pressed: bool) {
        self.lmb = pressed;
    }

    pub fn set_rmb(&mut self, pressed: bool) {
        self.rmb = pressed;
    }

    pub fn set_mouse(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    pub fn add_wheel(&mut self, delta: f32) {
        if delta > 0.0 {
            self.wheel -= 1;
        } else if delta < 0.0 {
            self.wheel += 1;
        }
    }

    /// Rising-edge flags. Call once per simulation tick.
    pub fn consume_edges(&mut self) -> InputEdges {
        let edges = InputEdges {
            sit: self.sit && !self.sit_was,
            interact: self.interact && !self.interact_was,
            attack: self.attack && !self.attack_was,
            inventory: self.inventory && !self.inventory_was,
            drop: self.drop && !self.drop_was,
            craft: self.craft && !self.craft_was,
            transfer: self.transfer && !self.transfer_was,
            take: self.take && !self.take_was,
            recipe_prev: self.recipe_prev && !self.recipe_prev_was,
            recipe_next: self.recipe_next && !self.recipe_next_was,
            cursor_left: self.cursor_left && !self.cursor_left_was,
            cursor_right: self.cursor_right && !self.cursor_right_was,
            cursor_up: self.cursor_up && !self.cursor_up_was,
            cursor_down: self.cursor_down && !self.cursor_down_was,
            debug: self.debug && !self.debug_was,
            lmb: self.lmb && !self.lmb_was,
            rmb: self.rmb && !self.rmb_was,
            hotbar: self.hotbar.take(),
            wheel: std::mem::take(&mut self.wheel),
        };
        self.sit_was = self.sit;
        self.interact_was = self.interact;
        self.attack_was = self.attack;
        self.inventory_was = self.inventory;
        self.drop_was = self.drop;
        self.craft_was = self.craft;
        self.transfer_was = self.transfer;
        self.take_was = self.take;
        self.recipe_prev_was = self.recipe_prev;
        self.recipe_next_was = self.recipe_next;
        self.cursor_left_was = self.cursor_left;
        self.cursor_right_was = self.cursor_right;
        self.cursor_up_was = self.cursor_up;
        self.cursor_down_was = self.cursor_down;
        self.debug_was = self.debug;
        self.lmb_was = self.lmb;
        self.rmb_was = self.rmb;
        edges
    }

    pub fn toggle_mouse_capture(&mut self) {
        self.mouse_captured = !self.mouse_captured;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InputEdges {
    pub sit: bool,
    pub interact: bool,
    pub attack: bool,
    pub inventory: bool,
    pub drop: bool,
    pub craft: bool,
    pub transfer: bool,
    pub take: bool,
    pub recipe_prev: bool,
    pub recipe_next: bool,
    pub cursor_left: bool,
    pub cursor_right: bool,
    pub cursor_up: bool,
    pub cursor_down: bool,
    pub debug: bool,
    pub lmb: bool,
    pub rmb: bool,
    pub hotbar: Option<usize>,
    pub wheel: i32,
}
