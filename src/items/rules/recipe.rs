use super::catalog::ItemId;
use super::stack::Stack;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CraftStation {
    Hand,
    Workbench,
    Furnace,
}

#[derive(Clone, Copy, Debug)]
pub struct Ingredient {
    pub item: ItemId,
    pub count: u16,
}

impl From<Ingredient> for Stack {
    fn from(i: Ingredient) -> Self {
        Stack::new(i.item, i.count)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub id: u16,
    pub name: &'static str,
    pub station: CraftStation,
    pub inputs: &'static [Ingredient],
    pub output: Ingredient,
    /// Furnace only — 50 ms ticks of fuel+time.
    pub smelt_ticks: u32,
}

pub const RECIPES: &[Recipe] = &[
    Recipe {
        id: 1,
        name: "Stick x2",
        station: CraftStation::Hand,
        inputs: &[Ingredient { item: ItemId::WOOD, count: 1 }],
        output: Ingredient { item: ItemId::STICK, count: 2 },
        smelt_ticks: 0,
    },
    Recipe {
        id: 2,
        name: "Wooden pickaxe",
        station: CraftStation::Workbench,
        inputs: &[
            Ingredient { item: ItemId::STONE, count: 3 },
            Ingredient { item: ItemId::STICK, count: 2 },
        ],
        output: Ingredient { item: ItemId::PICKAXE, count: 1 },
        smelt_ticks: 0,
    },
    Recipe {
        id: 3,
        name: "Axe",
        station: CraftStation::Workbench,
        inputs: &[
            Ingredient { item: ItemId::STONE, count: 3 },
            Ingredient { item: ItemId::STICK, count: 2 },
        ],
        output: Ingredient { item: ItemId::AXE, count: 1 },
        smelt_ticks: 0,
    },
    Recipe {
        id: 4,
        name: "Iron sword",
        station: CraftStation::Workbench,
        inputs: &[
            Ingredient { item: ItemId::IRON, count: 2 },
            Ingredient { item: ItemId::STICK, count: 1 },
        ],
        output: Ingredient { item: ItemId::SWORD, count: 1 },
        smelt_ticks: 0,
    },
    Recipe {
        id: 10,
        name: "Smelt iron",
        station: CraftStation::Furnace,
        inputs: &[Ingredient { item: ItemId::ORE, count: 1 }],
        output: Ingredient { item: ItemId::IRON, count: 1 },
        smelt_ticks: 80,
    },
    Recipe {
        id: 11,
        name: "Cook meat",
        station: CraftStation::Furnace,
        inputs: &[Ingredient { item: ItemId::RAW_MEAT, count: 1 }],
        output: Ingredient { item: ItemId::COOKED_MEAT, count: 1 },
        smelt_ticks: 50,
    },
];

pub fn recipe_by_id(id: u16) -> Option<&'static Recipe> {
    RECIPES.iter().find(|r| r.id == id)
}

pub fn recipes_for(station: CraftStation) -> impl Iterator<Item = &'static Recipe> {
    RECIPES.iter().filter(move |r| r.station == station)
}

pub fn smelt_of(input: ItemId) -> Option<&'static Recipe> {
    RECIPES
        .iter()
        .find(|r| r.station == CraftStation::Furnace && r.inputs.first().is_some_and(|i| i.item == input))
}
