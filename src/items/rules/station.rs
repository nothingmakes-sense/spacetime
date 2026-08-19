//! Station kinds and per-slot rules.
//!
//! `StationKind` is the wire value stored in SpacetimeDB (`u8`).
//! [`SlotRole`] decides whether a dragged stack may land in a given slot
//! (furnace fuel vs input vs output).

use super::catalog::{CHEST_SLOTS, FURNACE_SLOTS};
use super::recipe::{smelt_of, CraftStation};
use super::stack::Stack;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StationKind {
    Chest = 0,
    Furnace = 1,
    Workbench = 2,
}

/// What a single station (or bag) slot is allowed to hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotRole {
    /// Player bag / chest — any stack.
    Any,
    /// Furnace input — only items that have a smelt recipe.
    SmeltInput,
    /// Furnace fuel — items with `ItemDef::fuel > 0`.
    Fuel,
    /// Furnace output — take only; placing is rejected.
    Output,
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

    /// Role of station slot `i`. Bag slots are always [`SlotRole::Any`].
    pub fn slot_role(self, i: usize) -> SlotRole {
        match self {
            Self::Chest => SlotRole::Any,
            Self::Workbench => SlotRole::Any,
            Self::Furnace => match i {
                0 => SlotRole::SmeltInput,
                1 => SlotRole::Fuel,
                _ => SlotRole::Output,
            },
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

impl SlotRole {
    /// `true` if `stack` may be inserted here. Empty stacks are always ok.
    pub fn accepts(self, stack: Stack) -> bool {
        if stack.is_empty() {
            return true;
        }
        match self {
            Self::Any => true,
            Self::SmeltInput => smelt_of(stack.item).is_some(),
            Self::Fuel => stack.item.def().fuel > 0,
            Self::Output => false,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Any => "",
            Self::SmeltInput => "in",
            Self::Fuel => "fuel",
            Self::Output => "out",
        }
    }
}
