//! Shared "base class" for everything placed in the world.
//!
//! Rust has no inheritance. New assets **embed** [`Object`] and implement
//! [`GameObject`] — that is the toolkit (transform, tags, interact, draw).

use glam::{Mat4, Vec3};

use super::transform::Transform;
use crate::anim::AnimLibrary;
use crate::items::{ItemStore, ItemView};
use crate::vulkan::ModelHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectKind {
    Character,
    Prop,
    Chest,
    Station,
    Loot,
    Weapon,
    Trigger,
}

/// Fields every asset gets for free.
#[derive(Clone, Debug)]
pub struct Object {
    pub id: ObjectId,
    pub name: String,
    pub kind: ObjectKind,
    pub transform: Transform,
    pub tags: Vec<String>,
    pub interactable: bool,
    pub interact_radius: f32,
    pub active: bool,
}

impl Object {
    pub fn new(id: ObjectId, name: impl Into<String>, kind: ObjectKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            transform: Transform::default(),
            tags: Vec::new(),
            interactable: false,
            interact_radius: 2.2,
            active: true,
        }
    }

    pub fn with_translation(mut self, t: Vec3) -> Self {
        self.transform.translation = t;
        self
    }

    pub fn with_interact(mut self, radius: f32) -> Self {
        self.interactable = true;
        self.interact_radius = radius;
        self
    }

    pub fn in_range(&self, pos: Vec3) -> bool {
        self.interactable
            && self.active
            && self.transform.translation.distance(pos) <= self.interact_radius
    }
}

/// One mesh draw issued by an object this frame.
#[derive(Clone, Copy, Debug)]
pub struct DrawRequest {
    pub handle: ModelHandle,
    pub model: Mat4,
}

/// Per-frame services handed to every object.
pub struct TickCtx<'a> {
    pub dt: f32,
    pub player_pos: Vec3,
    pub player_yaw: f32,
    pub grounded: bool,
    pub moving: Vec3,
    pub sprinting: bool,
    pub jump: bool,
    pub sit_toggle: bool,
    pub attack: bool,
    pub interact: bool,
    pub library: &'a AnimLibrary,
    pub items: &'a mut dyn ItemStore,
    pub item_view: &'a ItemView,
}

/// Implement this on a new asset type, embed [`Object`] as `base`.
pub trait GameObject: Send {
    fn base(&self) -> &Object;
    fn base_mut(&mut self) -> &mut Object;

    fn tick(&mut self, ctx: &mut TickCtx);

    /// Return `true` if this object handled the interact.
    fn interact(&mut self, _ctx: &mut TickCtx) -> bool {
        false
    }

    /// CPU-skinned vertices that must be uploaded before draw (characters).
    fn skinned_upload(&self) -> Option<(ModelHandle, &[Vec<crate::assets::Vertex>])> {
        None
    }

    fn draws(&self) -> Vec<DrawRequest>;
}
