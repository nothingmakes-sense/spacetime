# Procedural voxel terrain

This folder is the terrain stack. Inventory / ResourceBits stay in
`src/items` and `assets/kaykit/resource_bits`. Terrain **only** consumes the
shared materials library defined here.

## Goal

A KayKit-looking block world the player can walk, mine, and build on, using
the same palette as ResourceBits so dropped bars / logs match the ground they
came from.

## Materials library

[`materials.rs`](materials.rs) is the contract:

| id | name   | source on `resource_bits_texture.png` | later use          |
|----|--------|----------------------------------------|--------------------|
| 0  | air    | —                                      | empty              |
| 1  | grass  | green moss tile                        | surface            |
| 2  | dirt   | warm brown                             | subsurface         |
| 3  | stone  | cool grey-blue rock                    | crust / caves      |
| 4  | wood   | plank brown                            | trees / builds     |
| 5  | iron   | brushed metal                          | ore veins          |
| 6  | copper | orange metal                           | ore veins          |
| 7  | gold   | gold trim                              | rare veins         |
| 8  | water  | deep teal                              | lakes (no collide) |

UVs are 128×128 cells on the 1024² atlas (`ATLAS_CELLS = 8`). When we greedy-
mesh a chunk we emit one quad per exposed face with that cell's UV.

The PNG lives at `assets/kaykit/resource_bits/resource_bits_texture.png`.
Do **not** duplicate it — inventory glTFs already reference the same file.

## Chunking

* Size: 16³ voxels (`CHUNK_SIZE`).
* Key: `IVec3` in chunk space.
* Storage: `u8` block id, 4096 bytes per chunk.
* Meshing v1 (now): culled cubes — skip a face if the neighbour is solid.
* Meshing v2: greedy merge on each axis (fewer verts, same atlas UVs).
* Upload: one `Model` per chunk via the existing Phong pipeline.

## Generation (v1)

Heightmap:

```
h(x,z) = BASE + A1*vnoise(x/48,z/48) + A2*vnoise(x/12,z/12)
```

* `y < h-3` → stone
* `y < h`   → dirt
* `y == h`  → grass
* iron / copper / gold speckles below `h-6` from a third noise octave
* water fill where `h < WATER_LEVEL`

Seed is a `u64` so multiplayer clients can rebuild the same world from the
module without shipping voxel bytes.

## Multiplayer

Do **not** replicate every block. Replicate:

1. `world_seed` on the SpacetimeDB `World` table (once).
2. `VoxelEdit { x, y, z, block }` for player breaks / places.

Clients generate from the seed, then apply the edit log. That keeps the
authoritative module tiny.

## Integration steps

1. **Now** — materials table + `Chunk::from_height` + culled mesh + unit test.
2. Upload one debug chunk behind the spawn when F3 is on (optional).
3. Replace the flat ground plane with a ring of chunks around the player
   (done — plane removed; collision is swept AABB vs voxels / props / actors).
4. Dig / place with the pickaxe / held block, writing `VoxelEdit`.
5. Greedy mesher + LOD for far chunks.

## Non-goals (this pass)

* Trees, caves, biomes
* Lighting / AO
* Streaming from disk
