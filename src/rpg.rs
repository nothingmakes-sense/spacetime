//! Client-side hero sheet: attributes, skills, equipment, placed builds.
//!
//! Authoritative stacks stay in [`crate::items`]. This module is the RPG
//! layer the inventory tabs (STATS / SKILLS / BUILD) read and write.

use crate::items::{EquipKind, ItemId, Stack};

pub const EQUIP_SLOTS: usize = 6;
pub const STAT_CAP: u8 = 20;
pub const SKILL_CAP: u8 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatId {
    Str = 0,
    Dex = 1,
    Int = 2,
    Vit = 3,
    End = 4,
}

impl StatId {
    pub const ALL: [StatId; 5] = [Self::Str, Self::Dex, Self::Int, Self::Vit, Self::End];

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Str),
            1 => Some(Self::Dex),
            2 => Some(Self::Int),
            3 => Some(Self::Vit),
            4 => Some(Self::End),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Str => "STR",
            Self::Dex => "DEX",
            Self::Int => "INT",
            Self::Vit => "VIT",
            Self::End => "END",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Str => "STRENGTH",
            Self::Dex => "DEXTERITY",
            Self::Int => "INTELLECT",
            Self::Vit => "VITALITY",
            Self::End => "ENDURANCE",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Str => "MELEE DAMAGE AND CARRY",
            Self::Dex => "MOVE SPEED AND AIM",
            Self::Int => "CRAFT YIELD AND XP",
            Self::Vit => "HIT POINTS",
            Self::End => "STAMINA POOL",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillId {
    Mining = 0,
    Woodcutting = 1,
    Combat = 2,
    Crafting = 3,
    Building = 4,
}

impl SkillId {
    pub const ALL: [SkillId; 5] = [
        Self::Mining,
        Self::Woodcutting,
        Self::Combat,
        Self::Crafting,
        Self::Building,
    ];

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Mining),
            1 => Some(Self::Woodcutting),
            2 => Some(Self::Combat),
            3 => Some(Self::Crafting),
            4 => Some(Self::Building),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Mining => "MINING",
            Self::Woodcutting => "WOODCUTTING",
            Self::Combat => "COMBAT",
            Self::Crafting => "CRAFTING",
            Self::Building => "BUILDING",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Mining => "BREAK STONE AND ORE",
            Self::Woodcutting => "FELL TREES AND HAUL LOGS",
            Self::Combat => "SWING WEAPONS",
            Self::Crafting => "RECIPES AT HAND AND BENCH",
            Self::Building => "PLACE BLOCKS IN THE WORLD",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Skill {
    pub id: SkillId,
    pub level: u8,
    pub xp: u32,
}

impl Skill {
    pub fn new(id: SkillId) -> Self {
        Self {
            id,
            level: 1,
            xp: 0,
        }
    }

    pub fn next(self) -> u32 {
        40 + self.level as u32 * 25
    }

    pub fn ratio(self) -> f32 {
        let n = self.next().max(1) as f32;
        (self.xp as f32 / n).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Debug)]
pub struct Hero {
    pub str: u8,
    pub dex: u8,
    pub int: u8,
    pub vit: u8,
    pub end: u8,
    pub unspent: u8,
    pub hp: f32,
    pub skills: [Skill; 5],
    pub level: u8,
    pub xp: u32,
}

impl Hero {
    pub fn new() -> Self {
        let mut h = Self {
            str: 5,
            dex: 5,
            int: 5,
            vit: 5,
            end: 5,
            unspent: 8,
            hp: 100.0,
            skills: [
                Skill::new(SkillId::Mining),
                Skill::new(SkillId::Woodcutting),
                Skill::new(SkillId::Combat),
                Skill::new(SkillId::Crafting),
                Skill::new(SkillId::Building),
            ],
            level: 1,
            xp: 0,
        };
        h.hp = h.max_hp();
        h
    }

    pub fn stat(&self, id: StatId) -> u8 {
        match id {
            StatId::Str => self.str,
            StatId::Dex => self.dex,
            StatId::Int => self.int,
            StatId::Vit => self.vit,
            StatId::End => self.end,
        }
    }

    pub fn max_hp(&self) -> f32 {
        80.0 + self.vit as f32 * 8.0
    }

    pub fn max_stam(&self) -> f32 {
        40.0 + self.end as f32 * 7.0
    }

    pub fn melee(&self) -> f32 {
        4.0 + self.str as f32 * 1.15
    }

    pub fn carry(&self) -> u16 {
        40 + self.str as u16 * 4
    }

    pub fn speed_mult(&self) -> f32 {
        1.0 + (self.dex as f32 - 5.0) * 0.035
    }

    pub fn next_level(&self) -> u32 {
        80 + self.level as u32 * 40
    }

    pub fn spend(&mut self, id: StatId) -> bool {
        if self.unspent == 0 {
            return false;
        }
        let slot = match id {
            StatId::Str => &mut self.str,
            StatId::Dex => &mut self.dex,
            StatId::Int => &mut self.int,
            StatId::Vit => &mut self.vit,
            StatId::End => &mut self.end,
        };
        if *slot >= STAT_CAP {
            return false;
        }
        *slot += 1;
        self.unspent -= 1;
        if matches!(id, StatId::Vit) {
            self.hp += 8.0;
        }
        true
    }

    pub fn add_skill_xp(&mut self, id: SkillId, amount: u32) {
        let amount = if self.int > 5 {
            amount + (self.int as u32 - 5)
        } else {
            amount
        };
        let s = &mut self.skills[id as usize];
        s.xp += amount;
        while s.level < SKILL_CAP && s.xp >= s.next() {
            s.xp -= s.next();
            s.level += 1;
            if s.level % 5 == 0 {
                self.unspent = self.unspent.saturating_add(1);
            }
        }
        self.xp += (amount / 3).max(1);
        while self.level < SKILL_CAP && self.xp >= self.next_level() {
            self.xp -= self.next_level();
            self.level += 1;
            self.unspent = self.unspent.saturating_add(1);
        }
    }

    pub fn heal(&mut self, amount: f32) {
        self.hp = (self.hp + amount).min(self.max_hp());
    }
}

impl Default for Hero {
    fn default() -> Self {
        Self::new()
    }
}

pub fn equip_kind(slot: usize) -> EquipKind {
    match slot {
        0 => EquipKind::Head,
        1 => EquipKind::Chest,
        2 => EquipKind::Hands,
        3 => EquipKind::Legs,
        4 => EquipKind::Weapon,
        5 => EquipKind::Offhand,
        _ => EquipKind::None,
    }
}

pub fn equip_label(slot: usize) -> &'static str {
    match slot {
        0 => "HEAD",
        1 => "CHEST",
        2 => "HANDS",
        3 => "LEGS",
        4 => "WEAPON",
        5 => "OFFHAND",
        _ => "",
    }
}

pub fn empty_equip() -> [Stack; EQUIP_SLOTS] {
    [Stack::empty(); EQUIP_SLOTS]
}

/// Which skill harvesting this item trains.
pub fn harvest_skill(item: ItemId) -> Option<(SkillId, u32)> {
    match item {
        ItemId::STONE | ItemId::ORE | ItemId::COAL | ItemId::STONE_BRICK | ItemId::COPPER_NUGGET
        | ItemId::SILVER_NUGGET | ItemId::GOLD_NUGGET => Some((SkillId::Mining, 4)),
        ItemId::WOOD | ItemId::STICK | ItemId::WOOD_PLANK => Some((SkillId::Woodcutting, 4)),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct BuildPiece {
    pub id: u64,
    pub item: ItemId,
    pub pos: glam::Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spending_caps_and_grants_hp() {
        let mut h = Hero::new();
        let hp = h.max_hp();
        assert!(h.spend(StatId::Vit));
        assert_eq!(h.vit, 6);
        assert!(h.max_hp() > hp);
        h.unspent = 0;
        assert!(!h.spend(StatId::Str));
    }

    #[test]
    fn skill_xp_levels_up() {
        let mut h = Hero::new();
        h.add_skill_xp(SkillId::Mining, 10_000);
        assert!(h.skills[0].level > 1);
        assert!(h.level >= 1);
    }
}
