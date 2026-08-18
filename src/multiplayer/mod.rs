use anyhow::Result;
use log::{error, info};

// These traits are required for the methods we call
use spacetimedb_sdk::{DbContext, Table, TableWithPrimaryKey};

use crate::module_bindings::*;

pub struct Multiplayer {
    pub conn: DbConnection,
    pub local_identity: Option<spacetimedb_sdk::Identity>,
}

impl Multiplayer {
    pub fn connect(uri: &str, db_name: &str) -> Result<Self> {
        let conn = DbConnection::builder()
            .with_uri(uri)
            .with_database_name(db_name)
            .on_connect(|_conn, identity, _token| {
                info!("Connected as {}", identity.to_hex());
            })
            .on_connect_error(|_, err| {
                error!("Connect error: {err}");
            })
            .build()?;

        Ok(Self {
            conn,
            local_identity: None,
        })
    }

    pub fn is_connected(&self) -> bool {
        // You can make this more accurate later
        true
    }

    pub fn subscribe_players(&self) {
        // IMPORTANT: register callbacks BEFORE add_query / subscribe
        self.conn
            .subscription_builder()
            .on_applied(|ctx| {
                info!(
                    "Players subscription applied, count = {}",
                    ctx.db.player().count()
                );
            })
            .add_query(|q| q.from.player())
            .subscribe();
    }

    pub fn register_callbacks(&self) {
        // on_insert is always available
        self.conn.db.player().on_insert(|_ctx, player| {
            info!(
                "Player joined: {} at ({}, {}, {})",
                player.name, player.x, player.y, player.z
            );
        });

        // on_update requires the TableWithPrimaryKey trait (imported above)
        self.conn.db.player().on_update(|_ctx, _old, new| {
            info!(
                "Player updated: {} → ({}, {}, {})",
                new.name, new.x, new.y, new.z
            );
        });
    }

    pub fn send_transform(&self, x: f32, y: f32, z: f32, rot_y: f32) {
        self.conn.reducers.update_transform(x, y, z, rot_y);
    }

    pub fn frame_tick(&mut self) -> Result<()> {
        self.conn.frame_tick()?;
        Ok(())
    }
}