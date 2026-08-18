//! Optional Rapier3D sketch. Not compiled — `PhysicsWorld` in `mod.rs`
//! is the gameplay integration point. Wire this in when you want real
//! rigid-body collision instead of the capsule-on-plane fallback.

#![allow(dead_code, unused_variables)]

use rapier3d::prelude::*;

pub fn example_scene() {
    let mut rigid_body_set = RigidBodySet::new();
    let mut collider_set = ColliderSet::new();

    let body = rigid_body_set.insert(
        RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 5.0, 0.0])
            .build(),
    );
    collider_set.insert_with_parent(
        ColliderBuilder::cuboid(0.5, 0.5, 0.5).build(),
        body,
        &mut rigid_body_set,
    );
}
