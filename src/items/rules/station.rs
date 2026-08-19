use super::catalog::{CHEST_SLOTS, FURNACE_SLOTS};
use super::recipe::CraftStation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StationKind {
    Chest = 0,
    Furnace = 1,
    Workbench = 2,
}

impl StationKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Furnace,
            2 => Self::Workbench,
            _ => Self::Chest,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn slots(self) -> usize {
        match self {
            Self::Chest => CHEST_SLOTS,
            Self::Furnace => FURNACE_SLOTS,
            Self::Workbench => 0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Chest => "Chest",
            Self::Furnace => "Furnace",
            Self::Workbench => "Workbench",
        }
    }

    pub fn craft_station(self) -> Option<CraftStation> {
        match self {
            Self::Workbench => Some(CraftStation::Workbench),
            Self::Furnace => Some(CraftStation::Furnace),
            Self::Chest => None,
        }
    }

    /// Half-extents used for the physics blocker (and a reasonable interact volume).
    pub fn half_extents(self) -> (f32, f32, f32) {
        match self {
            Self::Chest => (0.42, 0.28, 0.30),
            Self::Furnace => (0.38, 0.55, 0.38),
            Self::Workbench => (0.70, 0.40, 0.40),
        }
    }
}
