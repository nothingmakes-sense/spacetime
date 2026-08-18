use rapier3d::prelude::*;

let mut rigid_body_set = RigidBodySet::new();
let mut collider_set = ColliderSet::new();
// … gravity, integration parameters, physics pipeline …

let body = rigid_body_set.insert(
    RigidBodyBuilder::dynamic()
        .translation(vector![0.0, 5.0, 0.0])
        .build()
);
collider_set.insert_with_parent(
    ColliderBuilder::cuboid(0.5, 0.5, 0.5).build(),
    body,
    &mut rigid_body_set,
);