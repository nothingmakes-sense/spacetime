//! Scene graph toolkit. New props / characters / interactables live here.

mod object;
mod transform;

pub use object::{DrawRequest, GameObject, Object, ObjectId, ObjectKind, TickCtx};
pub use transform::Transform;

use glam::Vec3;

/// Owns every spawned [`GameObject`]. IDs are stable for the session.
pub struct Scene {
    next_id: u32,
    pub nodes: Vec<Box<dyn GameObject>>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            nodes: Vec::new(),
        }
    }

    pub fn alloc_id(&mut self) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn spawn(&mut self, node: Box<dyn GameObject>) {
        self.nodes.push(node);
    }

    pub fn remove(&mut self, id: ObjectId) {
        self.nodes.retain(|n| n.base().id != id);
    }

    pub fn contains(&self, id: ObjectId) -> bool {
        self.nodes.iter().any(|n| n.base().id == id)
    }

    pub fn tick(&mut self, ctx: &mut TickCtx) {
        for n in &mut self.nodes {
            n.tick(ctx);
        }
        if ctx.interact {
            let pos = ctx.player_pos;
            let mut best: Option<(usize, f32)> = None;
            for (i, n) in self.nodes.iter().enumerate() {
                let b = n.base();
                if b.in_range(pos) {
                    let d = b.transform.translation.distance(pos);
                    if best.map(|(_, bd)| d < bd).unwrap_or(true) {
                        best = Some((i, d));
                    }
                }
            }
            if let Some((i, _)) = best {
                let _ = self.nodes[i].interact(ctx);
            }
        }
    }

    pub fn nearest_prompt(&self, pos: Vec3) -> Option<String> {
        self.nodes
            .iter()
            .filter(|n| n.base().in_range(pos))
            .min_by(|a, b| {
                let da = a.base().transform.translation.distance(pos);
                let db = b.base().transform.translation.distance(pos);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|n| format!("[E] {}", n.base().name))
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}
