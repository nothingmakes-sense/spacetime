//! Game loop: window events → input → items → physics → scene → Vulkan.
//!
//! **In:** winit events, [`GameMode`] (local store or SpacetimeDB connection).
//! **Out:** frames via [`VulkanContext`], reducer calls through [`ItemStore`].
//! Physics/items tick at [`FIXED_DT`]; locomotion uses the real frame `dt`
//! so the chase camera and the player mesh never disagree.

use anyhow::{Context, Result};
use glam::{Mat4, Vec3};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use crate::anim::AnimLibrary;
use crate::assets::{
    chest_parts, furnace_parts, ground_plane, unit_box, workbench_model, AdventurerClass,
    AssetManager, ANIM_GENERAL, ANIM_MOVEMENT, GROUND_HALF_EXTENT,
};
use crate::config::*;
use crate::game_mode::GameMode;
use crate::hud::{self, DebugSnap, ItemMeshes};
use crate::input::InputState;
#[allow(unused_imports)]
use crate::items::{
    selected_recipe, ItemStore, ItemUi, WorldSync, BAG_SLOTS, HOTBAR, STATION_RANGE,
};
use crate::objects::{AttachedItem, CharacterObject, PropObject};
use crate::physics::PhysicsWorld;
use crate::player::{character_model_matrix, Player};
use crate::scene::{Object, Scene, TickCtx};
use crate::vulkan::VulkanContext;
use crate::module_bindings::PlayerTableAccess;
use spacetimedb_sdk::Table;

pub struct App {
    window: Option<Arc<Window>>,
    vulkan: Option<VulkanContext>,

    assets: AssetManager,
    physics: PhysicsWorld,
    mode: GameMode,
    player: Player,
    input: InputState,
    anim_lib: AnimLibrary,
    scene: Scene,
    item_ui: ItemUi,
    item_meshes: Option<ItemMeshes>,
    world_sync: WorldSync,

    last_frame: Instant,
    accumulator: f32,
    fps: f32,

    ground: Option<crate::vulkan::ModelHandle>,
    remote_mesh: Option<crate::vulkan::ModelHandle>,
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
            anim_lib: AnimLibrary::new(),
            scene: Scene::new(),
            item_ui: ItemUi::default(),
            item_meshes: None,
            world_sync: WorldSync::new(),
            last_frame: Instant::now(),
            accumulator: 0.0,
            fps: 60.0,
            ground: None,
            remote_mesh: None,
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

        if let Err(e) = self.anim_lib.load_file(crate::assets::resolve_asset(ANIM_GENERAL)) {
            warn!("general anim pack: {e:#}");
        }
        if let Err(e) = self.anim_lib.load_file(crate::assets::resolve_asset(ANIM_MOVEMENT)) {
            warn!("movement anim pack: {e:#}");
        }
        info!("animation library: {} clips", self.anim_lib.len());

        self.ground = Some(vulkan.upload_model(&ground_plane(GROUND_HALF_EXTENT, 12.0))?);

        let crate_mesh = vulkan.upload_model(&unit_box([0.55, 0.38, 0.22, 1.0]))?;
        let (chest_body_cpu, chest_lid_cpu) = chest_parts();
        let (furnace_cpu, ember_cpu) = furnace_parts();
        let chest_body = vulkan.upload_model(&chest_body_cpu)?;
        let chest_lid = vulkan.upload_model(&chest_lid_cpu)?;
        let furnace = vulkan.upload_model(&furnace_cpu)?;
        let ember = vulkan.upload_model(&ember_cpu)?;
        let workbench = vulkan.upload_model(&workbench_model())?;

        self.item_meshes = Some(ItemMeshes::upload(
            &mut vulkan,
            chest_body,
            chest_lid,
            furnace,
            ember,
            workbench,
        )?);

        self.spawn_character(
            &mut vulkan,
            LOCAL_CLASS,
            self.player.position,
            0.0,
            true,
        )?;

        let npc_spawns = [
            (AdventurerClass::Mage, Vec3::new(-6.0, 0.0, -4.0), 0.6),
            (AdventurerClass::Ranger, Vec3::new(6.0, 0.0, -4.0), -0.6),
            (AdventurerClass::Barbarian, Vec3::new(0.0, 0.0, -8.0), 3.14),
            (AdventurerClass::Rogue, Vec3::new(4.0, 0.0, 2.0), 2.2),
        ];
        for (class, pos, yaw) in npc_spawns {
            if let Err(e) = self.spawn_character(&mut vulkan, class, pos, yaw, false) {
                warn!("NPC {}: {e:#}", class.display_name());
            }
        }

        if let GameMode::SinglePlayer { world, .. } = &self.mode {
            for ent in &world.entities {
                let id = self.scene.alloc_id();
                let mut obj = Object::new(id, "crate", crate::scene::ObjectKind::Prop)
                    .with_translation(ent.position - Vec3::new(0.0, ent.half_extents.y, 0.0));
                obj.transform.scale = ent.half_extents * 2.0;
                self.scene.spawn(Box::new(PropObject::new(obj, crate_mesh)));
                self.physics.add_static_box(ent.position, ent.half_extents);
            }
            info!("Single-player world ready ({} props)", world.entities.len());
        }

        // Blockers for the default stations (same layout in both modes).
        for (kind, x, y, z, _) in crate::items::DEFAULT_STATIONS {
            let (hx, hy, hz) = kind.half_extents();
            self.physics
                .add_static_box(Vec3::new(*x, *y + hy, *z), Vec3::new(hx, hy, hz));
        }

        {
            let view = self.mode.items().view();
            if let Some(meshes) = &self.item_meshes {
                self.world_sync.apply(&mut self.scene, &view, meshes);
            }
        }

        self.physics.create_ground();
        self.physics.create_player_capsule(self.player.position);
        self.vulkan = Some(vulkan);

        info!(
            "Controls: WASD · Shift run · hold Space jump · C/F sit · Q/LMB swing · E use · Tab bag · G drop · R craft · T/Y transfer · Esc mouse"
        );
        Ok(())
    }

    fn spawn_character(
        &mut self,
        vulkan: &mut VulkanContext,
        class: AdventurerClass,
        pos: Vec3,
        yaw: f32,
        is_local: bool,
    ) -> Result<()> {
        let rig = self
            .assets
            .load_rigged_class(class)
            .with_context(|| format!("rig {}", class.glb_path()))?;
        let gpu = vulkan.upload_model(&rig.as_model())?;
        if is_local {
            self.remote_mesh = Some(gpu);
        }

        let weapon = match self.assets.load_model(class.default_weapon()) {
            Ok(m) => Some(AttachedItem {
                handle: vulkan.upload_model(&m)?,
                socket: "handslot.r".into(),
            }),
            Err(e) => {
                warn!("weapon for {}: {e}", class.display_name());
                None
            }
        };

        let id = self.scene.alloc_id();
        let name = if is_local {
            format!("Player ({})", class.display_name())
        } else {
            class.display_name().to_string()
        };
        let obj = Object::new(id, name, crate::scene::ObjectKind::Character).with_translation(pos);
        self.scene.spawn(Box::new(CharacterObject::new(
            obj, class, rig, gpu, weapon, yaw, is_local,
        )));
        Ok(())
    }

    fn handle_item_input(&mut self, edges: &crate::input::InputEdges) {
        let pos = self.player.position;
        let yaw = self.player.yaw;

        if edges.inventory {
            self.item_ui.toggle_bag();
            if self.item_ui.bag_open {
                // Free the cursor so the player can click slots. WASD stays live.
                self.input.mouse_captured = false;
                self.set_cursor_captured(false);
            } else {
                self.item_ui.cancel_drag();
                self.mode.items().close_station();
                self.item_ui.on_station_closed();
            }
        }

        if edges.debug {
            self.item_ui.debug = !self.item_ui.debug;
        }

        // Click-to-move: LMB whole stack, RMB one item. Works on the hotbar
        // even when the bag is closed, as long as the cursor is free.
        if (edges.lmb || edges.rmb) && (self.item_ui.bag_open || !self.input.mouse_captured) {
            if let Some(w) = &self.window {
                let size = w.inner_size();
                self.item_ui.set_mouse_pixels(
                    self.input.mouse_x,
                    self.input.mouse_y,
                    size.width as f32,
                    size.height as f32,
                );
            }
            let view = self.mode.items().view();
            if let Some(slot) = hud::hit_slot(&view, self.item_ui.bag_open, self.item_ui.mouse_ndc.0, self.item_ui.mouse_ndc.1)
            {
                self.item_ui.click_slot(self.mode.items(), slot, edges.rmb);
            } else if edges.lmb && !self.item_ui.held.is_empty() {
                self.item_ui.cancel_drag();
            }
        }

        if edges.hotbar.is_some() || edges.wheel != 0 {
            let cur = self.mode.items().view().selected;
            if let Some(h) = edges.hotbar {
                self.mode.items().select(h);
            } else if edges.wheel != 0 {
                self.mode
                    .items()
                    .select(ItemUi::next_hotbar(cur, edges.wheel));
            }
        }

        if self.item_ui.bag_open
            && (edges.cursor_left
                || edges.cursor_right
                || edges.cursor_up
                || edges.cursor_down)
        {
            let dx = edges.cursor_right as i32 - edges.cursor_left as i32;
            let dy = edges.cursor_down as i32 - edges.cursor_up as i32;
            let view = self.mode.items().view();
            if self.item_ui.focus_station {
                self.item_ui.move_cursor(&view, dx, dy);
            } else {
                let s = view.selected.min(BAG_SLOTS - 1);
                let col = (s % HOTBAR) as i32 + dx;
                let row = (s / HOTBAR) as i32 + dy;
                let rows = (BAG_SLOTS / HOTBAR) as i32;
                let nc = col.rem_euclid(HOTBAR as i32) as usize;
                let nr = row.clamp(0, rows - 1) as usize;
                self.mode.items().select(nr * HOTBAR + nc);
            }
        }

        if edges.drop {
            self.mode.items().drop_selected(pos, yaw);
        }
        if edges.recipe_prev {
            self.mode.items().cycle_recipe(-1);
        }
        if edges.recipe_next {
            self.mode.items().cycle_recipe(1);
        }
        if edges.craft {
            if let Some(r) = selected_recipe(&self.mode.items().view()) {
                self.mode.items().craft(r.id);
            }
        }
        if edges.transfer {
            self.mode.items().transfer_selected(true);
        }
        if edges.take {
            self.mode.items().transfer_selected(false);
        }

        // Walk-over pickup.
        self.mode.items().pickup_nearest(pos);

        if self.mode.items().view().open_station.is_some() {
            if !self.item_ui.bag_open {
                self.input.mouse_captured = false;
                self.set_cursor_captured(false);
            }
            self.item_ui.on_station_opened();
        }
    }

    fn close_all_ui(&mut self) {
        self.item_ui.cancel_drag();
        self.item_ui.bag_open = false;
        self.mode.items().close_station();
        self.item_ui.on_station_closed();
    }

    /// Kinematic step. Called every render frame with the real `dt` so the
    /// body and chase camera advance together (no 60 Hz pose snap).
    fn drive_player(&mut self, dt: f32) {
        self.player.update_movement(&self.input, dt);
        self.physics.set_wish_horizontal(
            self.player.velocity.x,
            self.player.velocity.z,
            self.input.jump && !self.player.sitting,
        );
        self.physics.step(dt);
        if let Some((pos, on_ground)) = self.physics.player_transform() {
            self.player.position = pos;
            self.player.on_ground = on_ground;
            self.player.velocity.y = self.physics.player_velocity().y;
        }
    }

    fn step_locomotion(&mut self, dt: f32) {
        self.drive_player(dt);
        for n in &mut self.scene.nodes {
            n.sync_local(self.player.position, self.player.yaw);
        }
    }

    fn update(&mut self, dt: f32) {
        let edges = self.input.consume_edges();
        if edges.sit {
            self.player.sitting = !self.player.sitting;
        }

        self.handle_item_input(&edges);

        {
            let view = self.mode.items().view();
            if let Some(st) = view.open_station_view() {
                if self.player.position.distance(st.pos) > STATION_RANGE {
                    drop(view);
                    self.close_all_ui();
                }
            }
        }

        self.mode.items().tick(dt);

        let item_view = self.mode.items().view();
        if let Some(meshes) = &self.item_meshes {
            self.world_sync.apply(&mut self.scene, &item_view, meshes);
        }

        {
            let mut ctx = TickCtx {
                dt,
                player_pos: self.player.position,
                player_yaw: self.player.yaw,
                grounded: self.player.on_ground,
                moving: self.player.velocity,
                sprinting: self.input.sprint,
                jump: self.input.jump,
                sit_toggle: edges.sit,
                attack: edges.attack,
                interact: edges.interact,
                library: &self.anim_lib,
                items: self.mode.items(),
                item_view: &item_view,
            };
            self.scene.tick(&mut ctx);
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
                let me = net.local_identity;
                remote_players.clear();
                for p in net.conn.db.player().iter() {
                    if me == Some(p.identity) {
                        continue;
                    }
                    remote_players.push(crate::game_mode::RemotePlayer {
                        identity: p.identity,
                        name: p.name,
                        position: Vec3::new(p.x, p.y, p.z),
                        yaw: p.rot_y,
                    });
                }
            }
            GameMode::SinglePlayer { .. } => {}
        }

        if let Some(w) = &self.window {
            let prompt = self.scene.nearest_prompt(self.player.position);
            let line = hud::status_line(&self.mode.items().view(), &self.item_ui, prompt.as_deref());
            w.set_title(&format!("{WINDOW_TITLE}  ·  {line}"));
        }
    }

    fn render(&mut self) -> Result<()> {
        let Some(vk) = self.vulkan.as_mut() else {
            return Ok(());
        };
        let Some(window) = self.window.as_ref() else {
            return Ok(());
        };

        for node in &self.scene.nodes {
            if let Some((handle, parts)) = node.skinned_upload() {
                vk.update_model_vertices(handle, parts);
            }
        }

        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let aspect = size.width as f32 / size.height.max(1) as f32;
        let (view, eye) = self.player.chase_view_matrix();
        let proj = Mat4::perspective_rh(55f32.to_radians(), aspect, 0.1, 200.0);

        vk.begin_frame()?;
        vk.update_camera_ubo(view, proj, eye);
        vk.update_light_ubo(Vec3::new(12.0, 22.0, 8.0), Vec3::new(1.0, 0.96, 0.88), 0.22, 0.35, 28.0);

        if let Some(g) = self.ground {
            vk.draw_model(g, Mat4::IDENTITY)?;
        }

        for node in &self.scene.nodes {
            for draw in node.draws() {
                vk.draw_model(draw.handle, draw.model)?;
            }
        }

        if let GameMode::Multiplayer { remote_players, .. } = &self.mode {
            if let Some(mesh) = self.remote_mesh {
                for rp in remote_players {
                    vk.draw_model(mesh, character_model_matrix(rp.position, rp.yaw))?;
                }
            }
        }

        if let Some(meshes) = &self.item_meshes {
            let item_view = self.mode.items().view();
            self.item_ui.set_mouse_pixels(
                self.input.mouse_x,
                self.input.mouse_y,
                size.width as f32,
                size.height as f32,
            );
            let debug = self.item_ui.debug.then_some(DebugSnap {
                fps: self.fps,
                pos: self.player.position,
                vel: self.player.velocity,
                yaw: self.player.yaw,
                pitch: self.player.pitch,
                grounded: self.player.on_ground,
                sitting: self.player.sitting,
                bag_open: self.item_ui.bag_open,
                selected: item_view.selected,
                held: self.item_ui.held,
                station: item_view.open_station_view().map(|s| s.kind.name()),
                loot: item_view.loot.len(),
                multiplayer: matches!(self.mode, GameMode::Multiplayer { .. }),
            });
            vk.begin_overlay();
            for draw in hud::hud_draws(meshes, &item_view, &self.item_ui, debug.as_ref()) {
                vk.draw_model(draw.handle, draw.model)?;
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
                info!("Window ready — click to look, Esc releases the mouse");
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
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.input.add_wheel(y);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.set_mouse(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => {
                        if self.item_ui.bag_open {
                            self.input.set_lmb(pressed);
                        } else if pressed && !self.input.mouse_captured {
                            self.input.mouse_captured = true;
                            self.set_cursor_captured(true);
                        } else {
                            self.input.set_attack(pressed);
                        }
                    }
                    MouseButton::Right => self.input.set_rmb(pressed),
                    _ => {}
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
                    if self.item_ui.bag_open || self.mode.items().view().open_station.is_some() {
                        self.close_all_ui();
                    } else {
                        self.input.toggle_mouse_capture();
                        self.set_cursor_captured(self.input.mouse_captured);
                    }
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
            if self.input.mouse_captured && !self.item_ui.bag_open {
                self.player.apply_look(delta.0, delta.1);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let frame_time = (now - self.last_frame).as_secs_f32().min(MAX_FRAME_TIME);
        self.last_frame = now;

        self.fps = if frame_time > 1e-4 {
            self.fps * 0.9 + (1.0 / frame_time) * 0.1
        } else {
            self.fps
        };

        self.step_locomotion(frame_time);

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
