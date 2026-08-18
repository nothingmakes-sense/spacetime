use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec3};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use crate::assets::{AssetManager, Model};
use crate::config::*;
use crate::game_mode::GameMode;
use crate::input::InputState;
use crate::physics::PhysicsWorld;
use crate::player::Player;
use crate::vulkan::VulkanContext;

pub struct App {
    window: Option<Arc<Window>>,
    vulkan: Option<VulkanContext>,

    assets: AssetManager,
    physics: PhysicsWorld,
    mode: GameMode,
    player: Player,
    input: InputState,

    last_frame: Instant,
    accumulator: f32,

    player_model: Option<Model>,
    ground_model: Option<Model>,
}

impl App {
    pub fn new(mode: GameMode) -> Result<Self> {
        Ok(Self {
            window: None,
            vulkan: None,
            assets: AssetManager::new(),
            physics: PhysicsWorld::new(GRAVITY),
            mode,
            player: Player::new(Vec3::new(0.0, 2.0, 5.0)),
            input: InputState::default(),
            last_frame: Instant::now(),
            accumulator: 0.0,
            player_model: None,
            ground_model: None,
        })
    }

    fn init_after_window(&mut self, window: Arc<Window>) -> Result<()> {
        let vulkan = VulkanContext::new(window.clone())
            .context("Vulkan initialization failed")?;
        self.vulkan = Some(vulkan);

        // Load assets
        match self.assets.load_model("assets/models/player.glb") {
            Ok(m) => self.player_model = Some(m),
            Err(e) => warn!("Could not load player model: {e}"),
        }
        match self.assets.load_model("assets/models/ground.obj") {
            Ok(m) => self.ground_model = Some(m),
            Err(e) => warn!("Could not load ground model: {e}"),
        }

        if let Some(vk) = &mut self.vulkan {
            if let Some(m) = &self.player_model {
                vk.upload_model(m)?;
            }
            if let Some(m) = &self.ground_model {
                vk.upload_model(m)?;
            }
        }

        self.physics.create_ground();
        self.physics.create_player_capsule(self.player.position);

        match &self.mode {
            GameMode::SinglePlayer { world } => {
                info!("Single-player world ready ({} entities)", world.entities.len());
                for ent in &world.entities {
                    self.physics.add_static_box(ent.position, ent.half_extents);
                }
            }
            GameMode::Multiplayer { .. } => {
                info!("Multiplayer mode – waiting for server data");
            }
        }

        Ok(())
    }

    fn update(&mut self, dt: f32) {
        self.player.update_movement(&self.input, dt);

        self.physics.step(dt);

        if let Some((pos, on_ground)) = self.physics.player_transform() {
            self.player.position = pos;
            self.player.on_ground = on_ground;
        } else {
            self.player.apply_simple_physics(GRAVITY, dt);
        }

        match &mut self.mode {
            GameMode::Multiplayer { net, remote_players } => {
                if let Err(e) = net.frame_tick() {
                    error!("Network tick failed: {e}");
                }

                net.send_transform(
                    self.player.position.x,
                    self.player.position.y,
                    self.player.position.z,
                    self.player.yaw,
                );

                let _ = remote_players; // updated via callbacks
            }
            GameMode::SinglePlayer { .. } => {}
        }
    }

    fn render(&mut self) -> Result<()> {
        let Some(vk) = self.vulkan.as_mut() else { return Ok(()); };
        let Some(window) = self.window.as_ref() else { return Ok(()); };

        let size = window.inner_size();
        let aspect = size.width as f32 / size.height.max(1) as f32;

        let view = self.player.view_matrix();
        let proj = Mat4::perspective_rh(45f32.to_radians(), aspect, 0.1, 500.0);
        let eye = self.player.eye_position();
        let light_pos = Vec3::new(10.0, 20.0, 10.0);

        vk.begin_frame()?;
        vk.update_camera_ubo(view, proj, eye);
        vk.update_light_ubo(light_pos, Vec3::ONE, 0.15, 0.5, 32.0);

        if let Some(model) = &self.ground_model {
            vk.draw_model(model, Mat4::IDENTITY)?;
        }

        match &self.mode {
            GameMode::Multiplayer { remote_players, .. } => {
                for rp in remote_players {
                    if let Some(model) = &self.player_model {
                        let model_mat = Mat4::from_rotation_translation(
                            Quat::from_rotation_y(rp.yaw),
                            rp.position,
                        );
                        vk.draw_model(model, model_mat)?;
                    }
                }
            }
            GameMode::SinglePlayer { world } => {
                for ent in &world.entities {
                    let model_mat = Mat4::from_translation(ent.position);
                    let _ = (ent, model_mat); // draw when you have meshes
                }
            }
        }

        vk.end_frame_and_present()?;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));

        match event_loop.create_window(attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                if let Err(e) = self.init_after_window(window.clone()) {
                    error!("Post-window init failed: {e:#}");
                    event_loop.exit();
                    return;
                }
                self.window = Some(window);
                info!("Window + systems initialized");
            }
            Err(e) => {
                error!("Failed to create window: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Close requested");
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if let Some(vk) = &mut self.vulkan {
                    if let Err(e) = vk.recreate_swapchain(size.width, size.height) {
                        error!("Swapchain recreation failed: {e}");
                    }
                }
            }

            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state,
                    ..
                },
                ..
            } => {
                let pressed = state == ElementState::Pressed;

                if code == KeyCode::Escape && pressed {
                    self.input.toggle_mouse_capture();
                    if let Some(w) = &self.window {
                        let mode = if self.input.mouse_captured {
                            CursorGrabMode::Confined
                        } else {
                            CursorGrabMode::None
                        };
                        let _ = w.set_cursor_grab(mode);
                        w.set_cursor_visible(!self.input.mouse_captured);
                    }
                } else {
                    self.input.handle_key(code, pressed);
                }
            }

            WindowEvent::CursorMoved { .. } if self.input.mouse_captured => {
                // TODO: relative mouse look → update player.yaw / player.pitch
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let frame_time = (now - self.last_frame).as_secs_f32().min(MAX_FRAME_TIME);
        self.last_frame = now;

        self.accumulator += frame_time;
        while self.accumulator >= FIXED_DT {
            self.update(FIXED_DT);
            self.accumulator -= FIXED_DT;
        }

        if let Err(e) = self.render() {
            error!("Render error: {e:#}");
        }

        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}