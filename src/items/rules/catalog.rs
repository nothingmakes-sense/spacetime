//! Stable wire IDs (`u16`). Never renumber — SpacetimeDB rows persist them.

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
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ItemDef {
    pub id: ItemId,
    pub name: &'static str,
    pub stack: u16,
    pub color: [f32; 4],
    pub tool: bool,
    /// Furnace fuel units granted when this item is consumed as fuel. 0 = not fuel.
    pub fuel: u32,
}

pub const CATALOG: &[ItemDef] = &[
    ItemDef { id: ItemId::WOOD, name: "Wood", stack: 64, color: [0.55, 0.34, 0.16, 1.0], tool: false, fuel: 40 },
    ItemDef { id: ItemId::STONE, name: "Stone", stack: 64, color: [0.55, 0.55, 0.52, 1.0], tool: false, fuel: 0 },
    ItemDef { id: ItemId::ORE, name: "Iron ore", stack: 64, color: [0.55, 0.32, 0.22, 1.0], tool: false, fuel: 0 },
    ItemDef { id: ItemId::COAL, name: "Coal", stack: 64, color: [0.12, 0.12, 0.13, 1.0], tool: false, fuel: 160 },
    ItemDef { id: ItemId::IRON, name: "Iron ingot", stack: 64, color: [0.72, 0.74, 0.78, 1.0], tool: false, fuel: 0 },
    ItemDef { id: ItemId::STICK, name: "Stick", stack: 64, color: [0.45, 0.30, 0.14, 1.0], tool: false, fuel: 20 },
    ItemDef { id: ItemId::RAW_MEAT, name: "Raw meat", stack: 16, color: [0.72, 0.28, 0.28, 1.0], tool: false, fuel: 0 },
    ItemDef { id: ItemId::COOKED_MEAT, name: "Cooked meat", stack: 16, color: [0.45, 0.22, 0.10, 1.0], tool: false, fuel: 0 },
    ItemDef { id: ItemId::PICKAXE, name: "Pickaxe", stack: 1, color: [0.40, 0.42, 0.48, 1.0], tool: true, fuel: 0 },
    ItemDef { id: ItemId::AXE, name: "Axe", stack: 1, color: [0.50, 0.38, 0.22, 1.0], tool: true, fuel: 0 },
    ItemDef { id: ItemId::SWORD, name: "Sword", stack: 1, color: [0.80, 0.82, 0.88, 1.0], tool: true, fuel: 0 },
];

pub const BAG_SLOTS: usize = 36;
pub const HOTBAR: usize = 9;
pub const CHEST_SLOTS: usize = 18;
pub const FURNACE_SLOTS: usize = 3;
pub const PICKUP_RANGE: f32 = 1.8;
pub const STATION_RANGE: f32 = 2.6;
