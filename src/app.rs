//! Game loop: window events → input → items → physics → scene → Vulkan.
//!
//! **In:** winit events, [`GameMode`] (local store or SpacetimeDB connection).
//! **Out:** frames via [`VulkanContext`], reducer calls through [`ItemStore`].
//! Physics/items tick at [`FIXED_DT`]; locomotion uses the real frame `dt`
//! so the chase camera and the player mesh never disagree.

use anyhow::{Context, Result};
use glam::{IVec3, Mat4, Vec3};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

use crate::anim::AnimLibrary;
use crate::assets::{
    chest_parts, furnace_parts, material_lib, unit_box, workbench_model, AdventurerClass,
    AssetManager, ANIM_GENERAL, ANIM_MOVEMENT,
};
use crate::config::*;
use crate::game_mode::GameMode;
use crate::hud::{self, DebugSnap, HudHit, ItemMeshes};
use crate::input::InputState;
#[allow(unused_imports)]
use crate::items::{
    selected_recipe, InvTab, ItemStore, ItemUi, WorldSync, BAG_SLOTS, HOTBAR, STATION_RANGE,
};
use crate::objects::{AttachedItem, CharacterObject, PropObject};
use crate::pause::{pause_draws, PauseMenu, PausePage, PauseResult};
use crate::physics::PhysicsWorld;
use crate::player::{character_model_matrix, look_dir, Player};
use crate::rpg::SkillId;
use crate::scene::{Object, ObjectKind, Scene, TickCtx};
use crate::settings::Settings;
use crate::vulkan::VulkanContext;
use crate::voxel::{solid_at, stand_y, Chunk, CHUNK_SIZE, WORLD_SEED};
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
    settings: Settings,
    pause: PauseMenu,
    /// Loaded voxel chunks. Meshes go in `voxel_draws`; occupancy is queried
    /// by physics via [`solid_at`].
    chunks: Vec<Chunk>,
    voxel_draws: Vec<(crate::vulkan::ModelHandle, glam::Mat4)>,
    exit_requested: bool,

    last_frame: Instant,
    accumulator: f32,
    fps: f32,

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
            player: Player::new(Vec3::new(0.5, stand_y(0.5, 6.0, WORLD_SEED), 6.0)),
            input: InputState::default(),
            anim_lib: AnimLibrary::new(),
            scene: Scene::new(),
            item_ui: ItemUi::default(),
            item_meshes: None,
            world_sync: WorldSync::new(),
            settings: Settings::load(),
            pause: PauseMenu::default(),
            chunks: Vec::new(),
            voxel_draws: Vec::new(),
            exit_requested: false,
            last_frame: Instant::now(),
            accumulator: 0.0,
            fps: 60.0,
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

    fn apply_display(&mut self) {
        let Some(w) = self.window.clone() else {
            return;
        };
        Self::apply_display_to(&self.settings, &w);
    }

    fn apply_display_to(settings: &Settings, w: &Window) {
        if settings.fullscreen {
            w.set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            w.set_fullscreen(None);
            let _ = w.request_inner_size(PhysicalSize::new(settings.width, settings.height));
        }
    }

    fn apply_graphics(&mut self) {
        if let Some(vk) = &mut self.vulkan {
            vk.set_vsync(self.settings.vsync);
        }
    }

    fn open_pause(&mut self) {
        self.close_all_ui();
        self.pause.open = true;
        self.pause.page = PausePage::Root;
        self.pause.waiting = None;
        self.input.mouse_captured = false;
        self.set_cursor_captured(false);
    }

    fn close_pause(&mut self) {
        self.pause.close();
        self.input.mouse_captured = true;
        self.set_cursor_captured(true);
    }

    fn init_after_window(&mut self, window: Arc<Window>) -> Result<()> {
        Self::apply_display_to(&self.settings, &window);

        let mut vulkan = VulkanContext::new(window.clone(), self.settings.vsync)
            .context("Vulkan initialization failed")?;

        if let Err(e) = self.anim_lib.load_file(crate::assets::resolve_asset(ANIM_GENERAL)) {
            warn!("general anim pack: {e:#}");
        }
        if let Err(e) = self.anim_lib.load_file(crate::assets::resolve_asset(ANIM_MOVEMENT)) {
            warn!("movement anim pack: {e:#}");
        }
        info!("animation library: {} clips", self.anim_lib.len());

        let mats = material_lib::MatCache::load();
        // Test-plane ground is gone — terrain is the voxel ring below.

        let wood_px = mats.or_solid("wood", [0x8a, 0x5a, 0x2b, 0xff]);
        let brick_px = mats.or_solid("brick", [0x8a, 0x3a, 0x28, 0xff]);
        let crate_mesh = vulkan.upload_model(&material_lib::textured_box(
            1.0,
            1.0,
            1.0,
            wood_px.clone(),
            "crate",
        ))?;
        let chest_body = vulkan.upload_model(&material_lib::textured_box(
            0.80,
            0.45,
            0.55,
            wood_px.clone(),
            "chest_body",
        ))?;
        let mut lid = material_lib::textured_box(0.82, 0.06, 0.57, wood_px.clone(), "chest_lid");
        for mesh in &mut lid.meshes {
            for v in &mut mesh.vertices {
                v.position[1] += 0.45;
            }
        }
        let chest_lid = vulkan.upload_model(&lid)?;
        let furnace = vulkan.upload_model(&material_lib::textured_box(
            0.72,
            1.05,
            0.72,
            brick_px,
            "furnace",
        ))?;
        let ember_cpu = {
            let (_, ember) = furnace_parts();
            ember
        };
        let ember = vulkan.upload_model(&ember_cpu)?;
        let thatch = mats.or_solid("planks", [0x7a, 0x52, 0x28, 0xff]);
        let workbench = vulkan.upload_model(&material_lib::textured_box(
            1.35,
            0.78,
            0.75,
            thatch,
            "workbench",
        ))?;
        let _ = (chest_parts, unit_box, workbench_model);

        self.item_meshes = Some(ItemMeshes::upload(
            &mut vulkan,
            chest_body,
            chest_lid,
            furnace,
            ember,
            workbench,
            &mats,
        )?);

        // Load a walkable ring of chunks at their true world Y (no -surface
        // offset — that existed only to sit the mesh on the deleted y=0 plane).
        //
        // **Takes:** `WORLD_SEED` + Material-LIB albedos.
        // **Gives:** `self.chunks` for [`solid_at`] and `self.voxel_draws` for Vulkan.
        // **Goes to:** physics occupancy and the terrain pass in `render`.
        self.chunks.clear();
        for cy in 0..=1 {
            for cz in -2..=2 {
                for cx in -2..=2 {
                    let chunk = Chunk::from_height(IVec3::new(cx, cy, cz), WORLD_SEED);
                    let model = chunk.mesh_textured(|b| mats.block(b).cloned());
                    match vulkan.upload_model(&model) {
                        Ok(h) => {
                            let t = Mat4::from_translation(Vec3::new(
                                cx as f32 * CHUNK_SIZE as f32,
                                cy as f32 * CHUNK_SIZE as f32,
                                cz as f32 * CHUNK_SIZE as f32,
                            ));
                            self.voxel_draws.push((h, t));
                            self.chunks.push(chunk);
                        }
                        Err(e) => warn!("voxel chunk ({cx},{cy},{cz}): {e:#}"),
                    }
                }
            }
        }
        info!(
            "voxel terrain: {} chunks at world Y (Material-LIB albedo)",
            self.voxel_draws.len()
        );

        self.spawn_character(
            &mut vulkan,
            LOCAL_CLASS,
            self.player.position,
            0.0,
            true,
        )?;

        let npc_spawns = [
            (AdventurerClass::Mage, -6.0, -4.0, 0.6),
            (AdventurerClass::Ranger, 6.0, -4.0, -0.6),
            (AdventurerClass::Barbarian, 0.0, -8.0, 3.14),
            (AdventurerClass::Rogue, 4.0, 2.0, 2.2),
        ];
        for (class, x, z, yaw) in npc_spawns {
            let pos = Vec3::new(x, stand_y(x, z, WORLD_SEED), z);
            if let Err(e) = self.spawn_character(&mut vulkan, class, pos, yaw, false) {
                warn!("NPC {}: {e:#}", class.display_name());
            }
        }

        if let GameMode::SinglePlayer { world, .. } = &self.mode {
            for ent in &world.entities {
                let feet = stand_y(ent.position.x, ent.position.z, WORLD_SEED);
                let pos = Vec3::new(ent.position.x, feet + ent.half_extents.y, ent.position.z);
                let id = self.scene.alloc_id();
                let mut obj = Object::new(id, "crate", crate::scene::ObjectKind::Prop)
                    .with_translation(pos - Vec3::new(0.0, ent.half_extents.y, 0.0));
                obj.transform.scale = ent.half_extents * 2.0;
                self.scene.spawn(Box::new(PropObject::new(obj, crate_mesh)));
                self.physics.add_static_box(pos, ent.half_extents);
            }
            info!("Single-player world ready ({} props)", world.entities.len());
        }

        for (kind, x, y, z, _) in crate::items::DEFAULT_STATIONS {
            let (hx, hy, hz) = kind.half_extents();
            let gy = if *y == 0.0 {
                stand_y(*x, *z, WORLD_SEED)
            } else {
                *y
            };
            self.physics
                .add_static_box(Vec3::new(*x, gy + hy, *z), Vec3::new(hx, hy, hz));
        }

        {
            let view = self.mode.items().view();
            if let Some(meshes) = &self.item_meshes {
                self.world_sync.apply(&mut self.scene, &view, meshes);
            }
        }

        self.physics.create_player_capsule(self.player.position);
        self.vulkan = Some(vulkan);

        info!(
            "Controls: WASD · Shift run · hold Space jump · Tab bag · Esc pause · E use · RMB place/eat"
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

    fn sync_mouse_ndc(&mut self) {
        if let Some(w) = &self.window {
            let size = w.inner_size();
            self.item_ui.set_mouse_pixels(
                self.input.mouse_x,
                self.input.mouse_y,
                size.width as f32,
                size.height as f32,
            );
        }
    }

    fn handle_pause_input(&mut self, edges: &crate::input::InputEdges) {
        self.sync_mouse_ndc();
        let (mx, my) = self.item_ui.mouse_ndc;
        self.pause.hover = self.pause.hit(&self.settings, mx, my);
        if !edges.lmb {
            return;
        }
        let Some(hit) = self.pause.hover else {
            return;
        };
        match self.pause.click(&mut self.settings, hit) {
            PauseResult::None => {}
            PauseResult::Resume => {
                self.input.mouse_captured = true;
                self.set_cursor_captured(true);
            }
            PauseResult::Exit => {
                self.exit_requested = true;
            }
            PauseResult::ApplyDisplay => self.apply_display(),
            PauseResult::ApplyGraphics => self.apply_graphics(),
        }
    }

    fn handle_item_input(&mut self, edges: &crate::input::InputEdges) {
        let pos = self.player.position;
        let yaw = self.player.yaw;

        if edges.inventory {
            self.item_ui.toggle_bag();
            if self.item_ui.bag_open {
                self.input.mouse_captured = false;
                self.set_cursor_captured(false);
            } else {
                self.item_ui.cancel_drag();
                self.mode.items().close_station();
                self.item_ui.on_station_closed();
                self.input.mouse_captured = true;
                self.set_cursor_captured(true);
            }
        }

        if edges.debug {
            self.item_ui.debug = !self.item_ui.debug;
        }

        let cursor_free = self.item_ui.bag_open || !self.input.mouse_captured;
        if (edges.lmb || edges.rmb) && cursor_free {
            self.sync_mouse_ndc();
            let view = self.mode.items().view();
            match hud::hit(&view, &self.item_ui, self.item_ui.mouse_ndc.0, self.item_ui.mouse_ndc.1) {
                Some(HudHit::Tab(t)) if edges.lmb => {
                    self.item_ui.tab = t;
                }
                Some(HudHit::StatPlus(i)) if edges.lmb => {
                    self.mode.items().spend_stat(i);
                }
                Some(HudHit::Craft) if edges.lmb => {
                    if let Some(r) = selected_recipe(&view) {
                        self.mode.items().craft(r.id);
                    }
                }
                Some(HudHit::Recipe(i)) if edges.lmb => {
                    self.mode.items().set_recipe_index(i as usize);
                }
                Some(HudHit::SelectBag(i)) if edges.lmb => {
                    self.mode.items().select(i);
                }
                Some(HudHit::Slot(slot)) => {
                    self.item_ui.click_slot(self.mode.items(), slot, edges.rmb);
                }
                _ => {
                    if edges.lmb && !self.item_ui.held.is_empty() {
                        self.item_ui.cancel_drag();
                    }
                }
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

        // World RMB: eat or place a block.
        if edges.rmb && self.input.mouse_captured && !self.item_ui.bag_open {
            let sel = self.mode.items().view().selected_stack();
            if sel.item.is_food() {
                self.mode.items().consume_selected();
            } else if sel.item.def().place != 0 {
                // Snap the block to the surface under the aim point.
                let dir = look_dir(yaw, self.player.pitch);
                let p = pos + Vec3::Y * 1.2 + dir * 2.6;
                let gx = p.x.floor() + 0.5;
                let gz = p.z.floor() + 0.5;
                let gy = stand_y(gx, gz, WORLD_SEED);
                self.mode.items().place_build(Vec3::new(gx, gy, gz));
            }
        }

        if edges.attack && !self.item_ui.bag_open {
            let sel = self.mode.items().view().selected_stack();
            let removed = if sel.item.def().place != 0 || sel.item.def().tool {
                self.mode.items().remove_nearest_build(pos, 3.2)
            } else {
                false
            };
            if !removed {
                self.mode.items().add_skill_xp(SkillId::Combat as u8, 3);
            }
        }

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
    ///
    /// **Takes:** frame `dt`, current input (via `player.update_movement`),
    /// hero DEX speed, live item/scene/remote positions, and `self.chunks`.
    /// **Gives:** updated `player.position` / `on_ground` / vertical velocity.
    /// **Goes to:** character `sync_local` and the chase camera.
    fn drive_player(&mut self, dt: f32) {
        self.player.update_movement(&self.input, dt);
        let spd = self.mode.items().view().hero.speed_mult();
        self.player.velocity.x *= spd;
        self.player.velocity.z *= spd;
        self.rebuild_colliders();
        self.physics.set_wish_horizontal(
            self.player.velocity.x,
            self.player.velocity.z,
            self.input.jump && !self.player.sitting,
        );
        let chunks = &self.chunks;
        self.physics
            .step(dt, |x, y, z| solid_at(chunks, WORLD_SEED, x, y, z));
        if let Some((pos, on_ground)) = self.physics.player_transform() {
            self.player.position = pos;
            self.player.on_ground = on_ground;
            self.player.velocity.y = self.physics.player_velocity().y;
        }
    }

    /// Rebuild static + actor colliders from the live world.
    ///
    /// **Takes:** [`ItemView`] stations / builds / loot, single-player crates
    /// already registered at init, scene characters (NPCs), and remote
    /// multiplayer players.
    /// **Gives:** a fresh collider list on [`PhysicsWorld`].
    /// **Source:** `mode.items().view()`, `scene.nodes`, `remote_players`.
    /// **Goes to:** [`PhysicsWorld::step`] this frame.
    fn rebuild_colliders(&mut self) {
        self.physics.clear_colliders();
        let view = self.mode.items().view();
        for st in &view.stations {
            let (hx, hy, hz) = st.kind.half_extents();
            self.physics
                .add_static_box(st.pos + Vec3::Y * hy, Vec3::new(hx, hy, hz));
        }
        for piece in &view.builds {
            // `textured_box` origin is bottom-centre — collider centre is +0.5 Y.
            self.physics
                .add_static_box(piece.pos + Vec3::Y * 0.5, Vec3::splat(0.5));
        }
        for loot in &view.loot {
            self.physics
                .add_static_box(loot.pos + Vec3::Y * 0.16, Vec3::splat(0.18));
        }
        drop(view);

        if let GameMode::SinglePlayer { world, .. } = &self.mode {
            for ent in &world.entities {
                let feet = stand_y(ent.position.x, ent.position.z, WORLD_SEED);
                let center = Vec3::new(ent.position.x, feet + ent.half_extents.y, ent.position.z);
                self.physics.add_static_box(center, ent.half_extents);
            }
        }

        let local = self.player.position;
        for n in &self.scene.nodes {
            let b = n.base();
            if b.kind != ObjectKind::Character {
                continue;
            }
            let p = b.transform.translation;
            if p.distance(local) < 0.4 {
                continue;
            }
            self.physics
                .add_actor_capsule(p, PLAYER_RADIUS, PLAYER_HEIGHT);
        }

        if let GameMode::Multiplayer { remote_players, .. } = &self.mode {
            for rp in remote_players {
                if rp.position.distance(local) < 0.4 {
                    continue;
                }
                self.physics
                    .add_actor_capsule(rp.position, PLAYER_RADIUS, PLAYER_HEIGHT);
            }
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

        if self.pause.open {
            self.handle_pause_input(&edges);
            return;
        }

        if edges.sit {
            self.player.sitting = !self.player.sitting;
        }

        self.handle_item_input(&edges);

        {
            let too_far = {
                let view = self.mode.items().view();
                view.open_station_view()
                    .map(|st| self.player.position.distance(st.pos) > STATION_RANGE)
                    .unwrap_or(false)
            };
            if too_far {
                self.close_all_ui();
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
                attack: edges.attack && !self.item_ui.bag_open,
                interact: edges.interact && !self.item_ui.bag_open,
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
        let (amb, spec, shine) = self.settings.quality.light();
        let bright = self.settings.brightness_mul();
        vk.update_light_ubo(
            Vec3::new(12.0, 22.0, 8.0),
            Vec3::new(1.0, 0.96, 0.88) * bright,
            amb,
            spec,
            shine,
        );

        for &(handle, model) in &self.voxel_draws {
            vk.draw_model(handle, model)?;
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
            self.item_ui.set_mouse_pixels(
                self.input.mouse_x,
                self.input.mouse_y,
                size.width as f32,
                size.height as f32,
            );
            let item_view = self.mode.items().view();
            self.item_ui.hover = hud::hit_slot(
                &item_view,
                &self.item_ui,
                self.item_ui.mouse_ndc.0,
                self.item_ui.mouse_ndc.1,
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
            if !self.pause.open {
                for draw in hud::hud_draws(meshes, &item_view, &self.item_ui, debug.as_ref()) {
                    vk.draw_model(draw.handle, draw.model)?;
                }
            }
            if self.pause.open {
                self.pause.hover = self.pause.hit(
                    &self.settings,
                    self.item_ui.mouse_ndc.0,
                    self.item_ui.mouse_ndc.1,
                );
                for draw in pause_draws(meshes, &self.pause, &self.settings, self.pause.hover) {
                    vk.draw_model(draw.handle, draw.model)?;
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
            .with_inner_size(PhysicalSize::new(self.settings.width, self.settings.height));

        match event_loop.create_window(attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                if let Err(e) = self.init_after_window(window.clone()) {
                    error!("Post-window init failed: {e:#}");
                    event_loop.exit();
                    return;
                }
                self.window = Some(window);
                info!("Window ready — click to look, Esc opens pause");
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
                if self.pause.open {
                    return;
                }
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
                        if self.pause.open || self.item_ui.bag_open {
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
                    if self.pause.waiting.is_some() {
                        self.pause.waiting = None;
                    } else if self.pause.open {
                        if self.pause.page != PausePage::Root {
                            self.pause.page = PausePage::Root;
                            self.pause.waiting = None;
                        } else {
                            self.close_pause();
                        }
                    } else if self.item_ui.bag_open
                        || self.mode.items().view().open_station.is_some()
                    {
                        self.close_all_ui();
                        self.input.mouse_captured = true;
                        self.set_cursor_captured(true);
                    } else {
                        self.open_pause();
                    }
                    return;
                }

                if self.pause.open {
                    if pressed && self.pause.waiting.is_some() {
                        self.pause.capture_rebind(&mut self.settings, code);
                    } else if !pressed {
                        self.input.handle_key(&self.settings, code, false);
                    }
                    return;
                }

                self.input.handle_key(&self.settings, code, pressed);
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
            if self.input.mouse_captured && !self.item_ui.bag_open && !self.pause.open {
                self.player
                    .apply_look(delta.0, delta.1, self.settings.mouse_sensitivity());
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            info!("Exit from pause menu");
            event_loop.exit();
            return;
        }

        let now = Instant::now();
        let frame_time = (now - self.last_frame).as_secs_f32().min(MAX_FRAME_TIME);
        self.last_frame = now;

        self.fps = if frame_time > 1e-4 {
            self.fps * 0.9 + (1.0 / frame_time) * 0.1
        } else {
            self.fps
        };

        if !self.pause.open {
            self.step_locomotion(frame_time);
            self.accumulator += frame_time;
            while self.accumulator >= FIXED_DT {
                self.update(FIXED_DT);
                self.accumulator -= FIXED_DT;
            }
        } else {
            // Still consume click edges so the menu stays responsive.
            self.accumulator = 0.0;
            self.update(FIXED_DT);
        }

        if let Err(e) = self.render() {
            error!("Render error: {e:#}");
        }

        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
