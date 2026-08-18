use winit::keyboard::KeyCode;

#[derive(Default, Debug, Clone)]
pub struct InputState {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sprint: bool,
    pub mouse_captured: bool,
}

impl InputState {
    pub fn handle_key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.back = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.jump = pressed,
            KeyCode::ShiftLeft => self.sprint = pressed,
            _ => {}
        }
    }

    pub fn toggle_mouse_capture(&mut self) {
        self.mouse_captured = !self.mouse_captured;
    }
}