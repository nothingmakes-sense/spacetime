use crate::items::LocalStore;
use crate::multiplayer::Multiplayer;
use crate::world::LocalWorld;
use glam::Vec3;

pub struct RemotePlayer {
    pub identity: spacetimedb_sdk::Identity,
    pub name: String,
    pub position: Vec3,
    pub yaw: f32,
}

pub enum GameMode {
    SinglePlayer {
        world: LocalWorld,
        store: LocalStore,
    },
    Multiplayer {
        net: Multiplayer,
        remote_players: Vec<RemotePlayer>,
    },
}

impl GameMode {
    pub fn items(&mut self) -> &mut dyn crate::items::ItemStore {
        match self {
            GameMode::SinglePlayer { store, .. } => store,
            GameMode::Multiplayer { net, .. } => net,
        }
    }
}
