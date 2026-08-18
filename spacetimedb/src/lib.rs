use spacetimedb::{Identity, ReducerContext, Table, Timestamp};

#[spacetimedb::table(accessor = player, public)]
#[derive(Clone, Debug)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rot_y: f32,
    pub last_update: Timestamp,
}

#[spacetimedb::reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    // IMPORTANT: sender() is a method in SpacetimeDB 2.x
    let sender = ctx.sender();

    if ctx.db.player().identity().find(&sender).is_none() {
        ctx.db.player().insert(Player {
            identity: sender,
            name: format!("Player-{}", &sender.to_hex()[..8]),
            x: 0.0,
            y: 1.0,
            z: 0.0,
            rot_y: 0.0,
            last_update: ctx.timestamp,
        });
    }
}

#[spacetimedb::reducer]
pub fn update_transform(ctx: &ReducerContext, x: f32, y: f32, z: f32, rot_y: f32) {
    let sender = ctx.sender();

    if let Some(mut p) = ctx.db.player().identity().find(&sender) {
        p.x = x;
        p.y = y;
        p.z = z;
        p.rot_y = rot_y;
        p.last_update = ctx.timestamp;
        ctx.db.player().identity().update(p);
    }
}

#[spacetimedb::reducer]
pub fn set_name(ctx: &ReducerContext, name: String) {
    let sender = ctx.sender();

    if let Some(mut p) = ctx.db.player().identity().find(&sender) {
        p.name = name;
        ctx.db.player().identity().update(p);
    }
}