use crate::multiplayer::Multiplayer;
use crate::world::LocalWorld;
use glam::Vec3;

pub struct RemotePlayer {
    pub identity: spacetimedb_sdk::Identity, // replace with generated type if needed
    pub name: String,
    pub position: Vec3,
    pub yaw: f32,
}

pub enum GameMode {
    SinglePlayer {
        world: LocalWorld,
    },
    Multiplayer {
        net: Multiplayer,
        remote_players: Vec<RemotePlayer>,
    },
}