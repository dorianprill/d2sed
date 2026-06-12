use libd2::core::character_class::CharacterClass;
use std::collections::HashMap;

/// The central application state model representing a loaded `.d2s` savegame.
#[derive(Debug, Clone)]
pub struct Savegame {
    pub name: String,
    pub class: CharacterClass,
    pub level: u32,
    pub experience: u32,
    pub gold: u32,
    pub stashed_gold: u32,

    pub strength: u32,
    pub dexterity: u32,
    pub vitality: u32,
    pub energy: u32,

    pub stat_points_remaining: u32,
    pub skill_points_remaining: u32,

    pub current_hp: u32,
    pub max_hp: u32,
    pub current_mana: u32,
    pub max_mana: u32,
    pub current_stamina: u32,
    pub max_stamina: u32,

    // Array of 30 skill levels
    pub skills: [u8; 30],

    // Raw file bytes for sections we don't fully parse/modify yet
    pub raw_bytes: Vec<u8>,
}

impl Savegame {
    pub fn generate_template(class: CharacterClass) -> Self {
        // Implement blank level 99 character generation
        Self {
            name: format!("Template{}", class),
            class,
            level: 99,
            experience: 3511147413, // Level 99 exp
            gold: 0,
            stashed_gold: 0,
            strength: 15,
            dexterity: 15,
            vitality: 10,
            energy: 10,
            stat_points_remaining: 5 * 98,
            skill_points_remaining: 98,
            current_hp: 40,
            max_hp: 40,
            current_mana: 10,
            max_mana: 10,
            current_stamina: 100,
            max_stamina: 100,
            skills: [0; 30],
            raw_bytes: Vec::new(),
        }
    }
}
