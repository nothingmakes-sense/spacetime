use std::collections::HashMap;

use crate::hud::ItemMeshes;
use crate::items::{ItemView, StationKind};
use crate::objects::{LootObject, StationMeshes, StationObject};
use crate::scene::{Object, ObjectKind, Scene};

/// Keeps scene nodes in lockstep with the authoritative [`ItemView`].
pub struct WorldSync {
    loot: HashMap<u64, crate::scene::ObjectId>,
    stations: HashMap<u64, crate::scene::ObjectId>,
}

impl WorldSync {
    pub fn new() -> Self {
        Self {
            loot: HashMap::new(),
            stations: HashMap::new(),
        }
    }

    pub fn apply(&mut self, scene: &mut Scene, view: &ItemView, meshes: &ItemMeshes) {
        let live: std::collections::HashSet<u64> = view.loot.iter().map(|l| l.id).collect();
        let stale: Vec<u64> = self
            .loot
            .keys()
            .copied()
            .filter(|id| !live.contains(id))
            .collect();
        for id in stale {
            if let Some(oid) = self.loot.remove(&id) {
                scene.remove(oid);
            }
        }
        for loot in &view.loot {
            let Some(handle) = meshes.visual(loot.stack.item, loot.stack.count) else {
                continue;
            };
            if let Some(&oid) = self.loot.get(&loot.id) {
                if let Some(node) = scene.nodes.iter_mut().find(|n| n.base().id == oid) {
                    node.apply_loot_visual(loot.stack, handle);
                }
                continue;
            }
            let oid = scene.alloc_id();
            scene.spawn(Box::new(LootObject::new(
                Object::new(oid, loot.stack.item.def().name, ObjectKind::Loot)
                    .with_translation(loot.pos),
                loot.id,
                loot.stack,
                handle,
            )));
            self.loot.insert(loot.id, oid);
        }

        for st in &view.stations {
            if self.stations.contains_key(&st.id) {
                continue;
            }
            let oid = scene.alloc_id();
            let meshes = StationMeshes {
                body: meshes.station_body(st.kind),
                lid: (st.kind == StationKind::Chest).then_some(meshes.chest_lid),
                ember: (st.kind == StationKind::Furnace).then_some(meshes.ember),
            };
            scene.spawn(Box::new(StationObject::new(
                Object::new(oid, st.kind.name(), ObjectKind::Station).with_translation(st.pos),
                st.id,
                st.kind,
                meshes,
            )));
            self.stations.insert(st.id, oid);
        }
    }
}

impl Default for WorldSync {
    fn default() -> Self {
        Self::new()
    }
}
