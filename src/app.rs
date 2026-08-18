use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec3};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use crate::assets::{ground_plane, unit_box, AdventurerClass, AssetManager, GROUND_HALF_EXTENT};
use crate::config::*;
use crate::game_mode::GameMode;
use crate::input::InputState;
use crate::physics::PhysicsWorld;
use crate::player::{character_model_matrix, Player};
use crate::vulkan::{ModelHandle, VulkanContext};

struct LoadedModels {
    ground: ModelHandle,
    player: ModelHandle,
    weapon: ModelHandle,
    crate_prop: ModelHandle,
    /// Rest-pose `handslot.r` so the sword sits in the knight's grip.
    player_hand_r: Mat4,
}

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

    models: Option<LoadedModels>,
    npc_classes: Vec<(AdventurerClass, ModelHandle, Vec3, f32)>,
}

impl App {
    pub fn new(mode: GameMode) -> Result<Self> {
        Ok(Self {
            window: None,
            vulkan: None,
            assets: AssetManager::new(),
            physics: PhysicsWorld::new(GRAVITY),
            mode,
            player: Player::new(Vec3::new(0.0, 0.0, 6.0)),
            input: InputState::default(),
            last_frame: Instant::now(),
            accumulator: 0.0,
            models: None,
            npc_classes: Vec::new(),
        })
    }

    fn set_cursor_captured(&self, captured: bool) {
        if let Some(w) = &self.window {
            let mode = if captured {
                CursorGrabMode::Confined
            } else {
                CursorGrabMode::None
            };
            let _ = w.set_cursor_grab(mode);
            w.set_cursor_visible(!captured);
        }
    }

    fn init_after_window(&mut self, window: Arc<Window>) -> Result<()> {
        let mut vulkan = VulkanContext::new(window.clone()).context("Vulkan initialization failed")?;

        let ground = vulkan.upload_model(&ground_plane(GROUND_HALF_EXTENT, 12.0))?;

        let player_cpu = self
            .assets
            .load_adventurer(LOCAL_CLASS)
            .with_context(|| format!("load {}", LOCAL_CLASS.glb_path()))?;
        let player_hand_r = player_cpu.socket("handslot.r").unwrap_or_else(|| {
            warn!("no handslot.r on {}; using fallback offset", LOCAL_CLASS.display_name());
            Mat4::from_translation(Vec3::new(0.35, 0.85, 0.25))
        });
        let player = vulkan.upload_model(&player_cpu)?;

        let weapon = match self.assets.load_model(LOCAL_CLASS.default_weapon()) {
            Ok(m) => vulkan.upload_model(&m)?,
            Err(e) => {
                warn!("weapon load failed ({e}); using crate prop");
                vulkan.upload_model(&unit_box([0.7, 0.7, 0.7, 1.0]))?
            }
        };

        let crate_prop = vulkan.upload_model(&unit_box([0.55, 0.38, 0.22, 1.0]))?;

        // Extra adventurers so the whole pack is visible in single-player.
        let npc_spawns = [
            (AdventurerClass::Mage, Vec3::new(-6.0, 0.0, -4.0), 0.6),
            (AdventurerClass::Ranger, Vec3::new(6.0, 0.0, -4.0), -0.6),
            (AdventurerClass::Barbarian, Vec3::new(0.0, 0.0, -8.0), 3.14),
            (AdventurerClass::Rogue, Vec3::new(4.0, 0.0, 2.0), 2.2),
        ];
        for (class, pos, yaw) in npc_spawns {
            match self.assets.load_adventurer(class) {
                Ok(m) => match vulkan.upload_model(&m) {
                    Ok(h) => self.npc_classes.push((class, h, pos, yaw)),
                    Err(e) => warn!("upload {} failed: {e}", class.display_name()),
                },
                Err(e) => warn!("load {} failed: {e}", class.glb_path()),
            }
        }

        self.models = Some(LoadedModels {
            ground,
            player,
            weapon,
            crate_prop,
            player_hand_r,
        });
        self.vulkan = Some(vulkan);

        self.physics.create_ground();
        self.physics.create_player_capsule(self.player.position);

        match &self.mode {
            GameMode::SinglePlayer { world } => {
                info!("Single-player world ready ({} props)", world.entities.len());
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
        self.physics.set_wish_horizontal(
            self.player.velocity.x,
            self.player.velocity.z,
            self.input.jump && self.player.on_ground,
        );
        self.physics.step(dt);

        if let Some((pos, on_ground)) = self.physics.player_transform() {
            self.player.position = pos;
            self.player.on_ground = on_ground;
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
                let _ = remote_players;
            }
            GameMode::SinglePlayer { .. } => {}
        }
    }

    fn render(&mut self) -> Result<()> {
        let Some(vk) = self.vulkan.as_mut() else {
            return Ok(());
        };
        let Some(window) = self.window.as_ref() else {
            return Ok(());
        };
        let Some(models) = self.models.as_ref() else {
            return Ok(());
        };

        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let aspect = size.width as f32 / size.height.max(1) as f32;

        let (view, eye) = self.player.chase_view_matrix();
        let proj = Mat4::perspective_rh(55f32.to_radians(), aspect, 0.1, 200.0);
        let light_pos = Vec3::new(12.0, 22.0, 8.0);

        vk.begin_frame()?;
        vk.update_camera_ubo(view, proj, eye);
        vk.update_light_ubo(light_pos, Vec3::new(1.0, 0.96, 0.88), 0.22, 0.35, 28.0);

        vk.draw_model(models.ground, Mat4::IDENTITY)?;
        vk.draw_model(models.player, self.player.model_matrix())?;
        vk.draw_model(
            models.weapon,
            self.player.model_matrix() * models.player_hand_r,
        )?;

        match &self.mode {
            GameMode::Multiplayer { remote_players, .. } => {
                for rp in remote_players {
                    vk.draw_model(models.player, character_model_matrix(rp.position, rp.yaw))?;
                }
            }
            GameMode::SinglePlayer { world } => {
                for ent in &world.entities {
                    let s = ent.half_extents * 2.0;
                    let model_mat = Mat4::from_scale_rotation_translation(
                        s,
                        Quat::IDENTITY,
                        ent.position - Vec3::new(0.0, ent.half_extents.y, 0.0),
                    );
                    vk.draw_model(models.crate_prop, model_mat)?;
                }
            }
        }

        for (_class, handle, pos, yaw) in &self.npc_classes {
            vk.draw_model(*handle, character_model_matrix(*pos, *yaw))?;
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
                info!("Window + renderer initialized — click to look, Esc releases the mouse");
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
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.input.mouse_captured {
                    self.input.mouse_captured = true;
                    self.set_cursor_captured(true);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if code == KeyCode::Escape && pressed {
                    self.input.toggle_mouse_capture();
                    self.set_cursor_captured(self.input.mouse_captured);
                } else {
                    self.input.handle_key(code, pressed);
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.input.mouse_captured {
                self.player.apply_look(delta.0, delta.1);
            }
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
