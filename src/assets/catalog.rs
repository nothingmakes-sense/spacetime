//! Paths into the KayKit Adventurers 2.0 pack shipped under `assets/kaykit/`.

pub const CHARACTERS_DIR: &str = "assets/kaykit/characters";
pub const WEAPONS_DIR: &str = "assets/kaykit/weapons";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdventurerClass {
    Knight,
    Mage,
    Ranger,
    Rogue,
    RogueHooded,
    Barbarian,
}

impl AdventurerClass {
    pub const ALL: [AdventurerClass; 6] = [
        Self::Knight,
        Self::Mage,
        Self::Ranger,
        Self::Rogue,
        Self::RogueHooded,
        Self::Barbarian,
    ];

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Knight => "Knight.glb",
            Self::Mage => "Mage.glb",
            Self::Ranger => "Ranger.glb",
            Self::Rogue => "Rogue.glb",
            Self::RogueHooded => "Rogue_Hooded.glb",
            Self::Barbarian => "Barbarian.glb",
        }
    }

    pub fn glb_path(self) -> String {
        format!("{CHARACTERS_DIR}/{}", self.file_name())
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Knight => "Knight",
            Self::Mage => "Mage",
            Self::Ranger => "Ranger",
            Self::Rogue => "Rogue",
            Self::RogueHooded => "Hooded Rogue",
            Self::Barbarian => "Barbarian",
        }
    }

    pub fn default_weapon(self) -> String {
        let file = match self {
            Self::Knight => "sword_1handed.gltf",
            Self::Mage => "staff.gltf",
            Self::Ranger => "bow.gltf",
            Self::Rogue | Self::RogueHooded => "dagger.gltf",
            Self::Barbarian => "axe_1handed.gltf",
        };
        format!("{WEAPONS_DIR}/{file}")
    }
}

pub const GROUND_HALF_EXTENT: f32 = 24.0;
