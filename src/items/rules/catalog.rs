//! Stable wire IDs (`u16`). Never renumber — SpacetimeDB rows persist them.
//!
//! Visuals come from KayKit ResourceBits. [`ItemId::visual_mesh`] picks a
//! glTF stem from [`ItemDef::tiers`] so a lone plank and a stack of 32 look
//! different (and upgrade when nearby drops auto-merge).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ItemId(pub u16);

impl ItemId {
    pub const EMPTY: Self = Self(0);
    pub const WOOD: Self = Self(1);
    pub const STONE: Self = Self(2);
    pub const ORE: Self = Self(3);
    pub const COAL: Self = Self(4);
    pub const IRON: Self = Self(5);
    pub const STICK: Self = Self(6);
    pub const RAW_MEAT: Self = Self(7);
    pub const COOKED_MEAT: Self = Self(8);
    pub const COPPER_NUGGET: Self = Self(9);
    pub const COPPER_BAR: Self = Self(10);
    pub const SILVER_NUGGET: Self = Self(11);
    pub const SILVER_BAR: Self = Self(12);
    pub const GOLD_NUGGET: Self = Self(13);
    pub const GOLD_BAR: Self = Self(14);
    pub const STONE_BRICK: Self = Self(15);
    pub const WOOD_PLANK: Self = Self(16);
    pub const COG: Self = Self(17);
    pub const TEXTILE: Self = Self(18);
    pub const FUEL_CAN: Self = Self(19);
    pub const PICKAXE: Self = Self(20);
    pub const AXE: Self = Self(21);
    pub const SWORD: Self = Self(22);

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn def(self) -> ItemDef {
        CATALOG.iter().copied().find(|d| d.id == self).unwrap_or(ItemDef {
            id: Self::EMPTY,
            name: "empty",
            stack: 1,
            color: [0.15, 0.15, 0.16, 1.0],
            tool: false,
            fuel: 0,
            mesh: "",
            tiers: &[],
        })
    }

    /// ResourceBits glTF stem for this stack size. Empty string = colored gem.
    pub fn visual_mesh(self, count: u16) -> &'static str {
        let def = self.def();
        let mut best = def.mesh;
        for (min, stem) in def.tiers {
            if count >= *min {
                best = stem;
            }
        }
        best
    }
}

/// One catalog row. `tiers` must be sorted by rising `min_count`.
#[derive(Clone, Copy, Debug)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: &'static str,
    pub stack: u16,
    pub color: [f32; 4],
    pub tool: bool,
    /// Furnace fuel units granted when this item is consumed as fuel. 0 = not fuel.
    pub fuel: u32,
    /// Default ResourceBits mesh (count == 1).
    pub mesh: &'static str,
    /// `(min_count, gltf_stem)` — swap the world/HUD mesh as the stack grows.
    pub tiers: &'static [(u16, &'static str)],
}

const LOG_TIERS: &[(u16, &str)] = &[
    (1, "Wood_Log_A"),
    (4, "Wood_Log_B"),
    (12, "Wood_Log_Stack"),
];
const PLANK_TIERS: &[(u16, &str)] = &[
    (1, "Wood_Plank_A"),
    (6, "Wood_Planks_Stack_Small"),
    (16, "Wood_Planks_Stack_Medium"),
    (32, "Wood_Planks_Stack_Large"),
];
const STONE_TIERS: &[(u16, &str)] = &[
    (1, "Stone_Chunks_Small"),
    (8, "Stone_Chunks_Large"),
];
const BRICK_TIERS: &[(u16, &str)] = &[
    (1, "Stone_Brick"),
    (6, "Stone_Bricks_Stack_Small"),
    (16, "Stone_Bricks_Stack_Medium"),
    (32, "Stone_Bricks_Stack_Large"),
];
const IRON_ORE_TIERS: &[(u16, &str)] = &[
    (1, "Iron_Nugget_Small"),
    (4, "Iron_Nugget_Medium"),
    (10, "Iron_Nuggets"),
];
const IRON_BAR_TIERS: &[(u16, &str)] = &[
    (1, "Iron_Bar"),
    (4, "Iron_Bars"),
    (8, "Iron_Bars_Stack_Small"),
    (16, "Iron_Bars_Stack_Medium"),
    (32, "Iron_Bars_Stack_Large"),
];
const COPPER_NUGGET_TIERS: &[(u16, &str)] = &[
    (1, "Copper_Nugget_Small"),
    (4, "Copper_Nugget_Medium"),
    (10, "Copper_Nuggets"),
];
const COPPER_BAR_TIERS: &[(u16, &str)] = &[
    (1, "Copper_Bar"),
    (4, "Copper_Bars"),
    (8, "Copper_Bars_Stack_Small"),
    (16, "Copper_Bars_Stack_Medium"),
    (32, "Copper_Bars_Stack_Large"),
];
const SILVER_NUGGET_TIERS: &[(u16, &str)] = &[
    (1, "Silver_Nugget_Small"),
    (4, "Silver_Nugget_Medium"),
    (10, "Silver_Nuggets"),
];
const SILVER_BAR_TIERS: &[(u16, &str)] = &[
    (1, "Silver_Bar"),
    (4, "Silver_Bars"),
    (8, "Silver_Bars_Stack_Small"),
    (16, "Silver_Bars_Stack_Medium"),
    (32, "Silver_Bars_Stack_Large"),
];
const GOLD_NUGGET_TIERS: &[(u16, &str)] = &[
    (1, "Gold_Nugget_Small"),
    (4, "Gold_Nugget_Medium"),
    (10, "Gold_Nuggets"),
];
const GOLD_BAR_TIERS: &[(u16, &str)] = &[
    (1, "Gold_Bar"),
    (4, "Gold_Bars"),
    (8, "Gold_Bars_Stack_Small"),
    (16, "Gold_Bars_Stack_Medium"),
    (32, "Gold_Bars_Stack_Large"),
];
const COG_TIERS: &[(u16, &str)] = &[
    (1, "Parts_Cog"),
    (6, "Parts_Pile_Small"),
    (16, "Parts_Pile_Medium"),
    (32, "Parts_Pile_Large"),
];
const TEXTILE_TIERS: &[(u16, &str)] = &[
    (1, "Textiles_A"),
    (8, "Textiles_Stack_Small"),
    (24, "Textiles_Stack_Large"),
];
const FUEL_TIERS: &[(u16, &str)] = &[(1, "Fuel_A_Jerrycan"), (8, "Fuel_A_Barrel")];
const COAL_TIERS: &[(u16, &str)] = &[(1, "Fuel_C_Jerrycan"), (8, "Fuel_C_Barrel")];

pub const CATALOG: &[ItemDef] = &[
    ItemDef { id: ItemId::WOOD, name: "Wood", stack: 64, color: [0.55, 0.34, 0.16, 1.0], tool: false, fuel: 40, mesh: "Wood_Log_A", tiers: LOG_TIERS },
    ItemDef { id: ItemId::STONE, name: "Stone", stack: 64, color: [0.55, 0.55, 0.52, 1.0], tool: false, fuel: 0, mesh: "Stone_Chunks_Small", tiers: STONE_TIERS },
    ItemDef { id: ItemId::ORE, name: "Iron ore", stack: 64, color: [0.55, 0.32, 0.22, 1.0], tool: false, fuel: 0, mesh: "Iron_Nugget_Small", tiers: IRON_ORE_TIERS },
    ItemDef { id: ItemId::COAL, name: "Coal", stack: 64, color: [0.12, 0.12, 0.13, 1.0], tool: false, fuel: 160, mesh: "Fuel_C_Jerrycan", tiers: COAL_TIERS },
    ItemDef { id: ItemId::IRON, name: "Iron ingot", stack: 64, color: [0.72, 0.74, 0.78, 1.0], tool: false, fuel: 0, mesh: "Iron_Bar", tiers: IRON_BAR_TIERS },
    ItemDef { id: ItemId::STICK, name: "Stick", stack: 64, color: [0.45, 0.30, 0.14, 1.0], tool: false, fuel: 20, mesh: "Wood_Plank_C", tiers: &[(1, "Wood_Plank_C")] },
    ItemDef { id: ItemId::RAW_MEAT, name: "Raw meat", stack: 16, color: [0.72, 0.28, 0.28, 1.0], tool: false, fuel: 0, mesh: "", tiers: &[] },
    ItemDef { id: ItemId::COOKED_MEAT, name: "Cooked meat", stack: 16, color: [0.45, 0.22, 0.10, 1.0], tool: false, fuel: 0, mesh: "", tiers: &[] },
    ItemDef { id: ItemId::COPPER_NUGGET, name: "Copper nugget", stack: 64, color: [0.80, 0.45, 0.22, 1.0], tool: false, fuel: 0, mesh: "Copper_Nugget_Small", tiers: COPPER_NUGGET_TIERS },
    ItemDef { id: ItemId::COPPER_BAR, name: "Copper bar", stack: 64, color: [0.85, 0.50, 0.25, 1.0], tool: false, fuel: 0, mesh: "Copper_Bar", tiers: COPPER_BAR_TIERS },
    ItemDef { id: ItemId::SILVER_NUGGET, name: "Silver nugget", stack: 64, color: [0.78, 0.80, 0.84, 1.0], tool: false, fuel: 0, mesh: "Silver_Nugget_Small", tiers: SILVER_NUGGET_TIERS },
    ItemDef { id: ItemId::SILVER_BAR, name: "Silver bar", stack: 64, color: [0.82, 0.84, 0.88, 1.0], tool: false, fuel: 0, mesh: "Silver_Bar", tiers: SILVER_BAR_TIERS },
    ItemDef { id: ItemId::GOLD_NUGGET, name: "Gold nugget", stack: 64, color: [0.90, 0.72, 0.20, 1.0], tool: false, fuel: 0, mesh: "Gold_Nugget_Small", tiers: GOLD_NUGGET_TIERS },
    ItemDef { id: ItemId::GOLD_BAR, name: "Gold bar", stack: 64, color: [0.95, 0.78, 0.18, 1.0], tool: false, fuel: 0, mesh: "Gold_Bar", tiers: GOLD_BAR_TIERS },
    ItemDef { id: ItemId::STONE_BRICK, name: "Stone brick", stack: 64, color: [0.50, 0.48, 0.44, 1.0], tool: false, fuel: 0, mesh: "Stone_Brick", tiers: BRICK_TIERS },
    ItemDef { id: ItemId::WOOD_PLANK, name: "Wood plank", stack: 64, color: [0.62, 0.42, 0.22, 1.0], tool: false, fuel: 30, mesh: "Wood_Plank_A", tiers: PLANK_TIERS },
    ItemDef { id: ItemId::COG, name: "Cog", stack: 64, color: [0.60, 0.58, 0.52, 1.0], tool: false, fuel: 0, mesh: "Parts_Cog", tiers: COG_TIERS },
    ItemDef { id: ItemId::TEXTILE, name: "Textile", stack: 64, color: [0.70, 0.62, 0.48, 1.0], tool: false, fuel: 10, mesh: "Textiles_A", tiers: TEXTILE_TIERS },
    ItemDef { id: ItemId::FUEL_CAN, name: "Fuel can", stack: 16, color: [0.75, 0.18, 0.14, 1.0], tool: false, fuel: 240, mesh: "Fuel_A_Jerrycan", tiers: FUEL_TIERS },
    ItemDef { id: ItemId::PICKAXE, name: "Pickaxe", stack: 1, color: [0.40, 0.42, 0.48, 1.0], tool: true, fuel: 0, mesh: "", tiers: &[] },
    ItemDef { id: ItemId::AXE, name: "Axe", stack: 1, color: [0.50, 0.38, 0.22, 1.0], tool: true, fuel: 0, mesh: "", tiers: &[] },
    ItemDef { id: ItemId::SWORD, name: "Sword", stack: 1, color: [0.80, 0.82, 0.88, 1.0], tool: true, fuel: 0, mesh: "", tiers: &[] },
];

pub const BAG_SLOTS: usize = 36;
pub const HOTBAR: usize = 9;
pub const CHEST_SLOTS: usize = 18;
pub const FURNACE_SLOTS: usize = 3;
pub const PICKUP_RANGE: f32 = 1.8;
pub const STATION_RANGE: f32 = 2.6;
/// World drops of the same item within this radius collapse into one stack
/// and the mesh upgrades via [`ItemId::visual_mesh`].
pub const STACK_MERGE_RANGE: f32 = 1.45;

pub const RESOURCE_BITS_DIR: &str = "assets/kaykit/resource_bits";
