use crate::save::{BitWriter, fix_header};
use anyhow::{Context, Result};
use libd2::core::character_class::CharacterClass;
use libd2::core::character_file::{CharacterFile, CharacterStat};
use std::path::Path;

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

    // Parsed libd2 character file
    pub char_file: Option<CharacterFile>,

    // Quests for Normal, Nightmare, Hell (41 words each)
    pub quests: [[u16; 41]; 3],

    pub hardcore: bool,
    pub died: bool,

    // Waypoints for Normal, Nightmare, Hell (39 waypoints total, stored as bits)
    pub waypoints: [[bool; 39]; 3],
}

pub struct BaseStats {
    pub str: u32,
    pub dex: u32,
    pub vit: u32,
    pub eng: u32,
    pub hp: u32,
    pub mana: u32,
    pub stamina: u32,
}

impl BaseStats {
    pub fn for_class(class: CharacterClass) -> Self {
        match class {
            CharacterClass::Amazon => Self {
                str: 20,
                dex: 25,
                vit: 20,
                eng: 15,
                hp: 50,
                mana: 15,
                stamina: 84,
            },
            CharacterClass::Sorceress => Self {
                str: 10,
                dex: 25,
                vit: 10,
                eng: 35,
                hp: 40,
                mana: 35,
                stamina: 74,
            },
            CharacterClass::Necromancer => Self {
                str: 15,
                dex: 25,
                vit: 15,
                eng: 25,
                hp: 45,
                mana: 25,
                stamina: 79,
            },
            CharacterClass::Paladin => Self {
                str: 25,
                dex: 20,
                vit: 25,
                eng: 15,
                hp: 55,
                mana: 15,
                stamina: 89,
            },
            CharacterClass::Barbarian => Self {
                str: 30,
                dex: 20,
                vit: 25,
                eng: 10,
                hp: 55,
                mana: 10,
                stamina: 92,
            },
            CharacterClass::Druid => Self {
                str: 15,
                dex: 20,
                vit: 25,
                eng: 20,
                hp: 55,
                mana: 20,
                stamina: 84,
            },
            CharacterClass::Assassin => Self {
                str: 20,
                dex: 20,
                vit: 20,
                eng: 25,
                hp: 50,
                mana: 25,
                stamina: 95,
            },
            CharacterClass::Warlock => Self {
                str: 15,
                dex: 25,
                vit: 15,
                eng: 25,
                hp: 45,
                mana: 25,
                stamina: 79,
            },
        }
    }
}

impl Savegame {
    pub fn total_allowed_stat_points(&self) -> u32 {
        let level_points = self.level.saturating_sub(1) * 5;
        // Assume 0 quests for now. Later we will parse quests.
        let quest_points = 0 * 5;
        level_points + quest_points
    }

    pub fn reset_stats(&mut self) {
        let base = BaseStats::for_class(self.class);
        self.strength = base.str;
        self.dexterity = base.dex;
        self.vitality = base.vit;
        self.energy = base.eng;
        self.stat_points_remaining = self.total_allowed_stat_points();

        // Recalculate HP, Mana, Stamina based on base values (ignoring level bonuses for now during reset)
        self.current_hp = base.hp;
        self.max_hp = base.hp;
        self.current_mana = base.mana;
        self.max_mana = base.mana;
        self.current_stamina = base.stamina;
        self.max_stamina = base.stamina;
    }

    pub fn increase_stat(&mut self, stat: CharacterStat) {
        if self.stat_points_remaining == 0 {
            return;
        }

        match stat {
            CharacterStat::Strength => self.strength += 1,
            CharacterStat::Dexterity => self.dexterity += 1,
            CharacterStat::Vitality => {
                self.vitality += 1;
                // Add specific HP per vitality point depending on class (simplified here)
                self.current_hp += 2;
                self.max_hp += 2;
                self.current_stamina += 1;
                self.max_stamina += 1;
            }
            CharacterStat::Energy => {
                self.energy += 1;
                // Add specific Mana per energy point depending on class (simplified here)
                self.current_mana += 2;
                self.max_mana += 2;
            }
            _ => return, // Only core stats can be manually increased
        }

        self.stat_points_remaining -= 1;
    }

    pub fn get_skill_name(class: CharacterClass, slot: usize) -> &'static str {
        match class {
            CharacterClass::Amazon => match slot {
                0 => "Magic Arrow",
                1 => "Fire Arrow",
                2 => "Inner Sight",
                3 => "Critical Strike",
                4 => "Jab",
                5 => "Cold Arrow",
                6 => "Multiple Shot",
                7 => "Dodge",
                8 => "Power Strike",
                9 => "Poison Javelin",
                10 => "Exploding Arrow",
                11 => "Slow Missiles",
                12 => "Avoid",
                13 => "Impale",
                14 => "Lightning Bolt",
                15 => "Ice Arrow",
                16 => "Guided Arrow",
                17 => "Penetrate",
                18 => "Charged Strike",
                19 => "Plague Javelin",
                20 => "Strafe",
                21 => "Immolation Arrow",
                22 => "Dopplezon",
                23 => "Evade",
                24 => "Fend",
                25 => "Freezing Arrow",
                26 => "Valkyrie",
                27 => "Pierce",
                28 => "Lightning Strike",
                29 => "Lightning Fury",
                _ => "Unknown",
            },
            CharacterClass::Sorceress => match slot {
                0 => "Fire Bolt",
                1 => "Warmth",
                2 => "Charged Bolt",
                3 => "Ice Bolt",
                4 => "Frozen Armor",
                5 => "Inferno",
                6 => "Static Field",
                7 => "Telekinesis",
                8 => "Frost Nova",
                9 => "Ice Blast",
                10 => "Blaze",
                11 => "Fire Ball",
                12 => "Nova",
                13 => "Lightning",
                14 => "Shiver Armor",
                15 => "Fire Wall",
                16 => "Enchant",
                17 => "Chain Lightning",
                18 => "Teleport",
                19 => "Glacial Spike",
                20 => "Meteor",
                21 => "Thunder Storm",
                22 => "Energy Shield",
                23 => "Blizzard",
                24 => "Chilling Armor",
                25 => "Fire Mastery",
                26 => "Hydra",
                27 => "Lightning Mastery",
                28 => "Frozen Orb",
                29 => "Cold Mastery",
                _ => "Unknown",
            },
            CharacterClass::Necromancer => match slot {
                0 => "Amplify Damage",
                1 => "Teeth",
                2 => "Bone Armor",
                3 => "Skeleton Mastery",
                4 => "Raise Skeleton",
                5 => "Dim Vision",
                6 => "Weaken",
                7 => "Poison Dagger",
                8 => "Corpse Explosion",
                9 => "Clay Golem",
                10 => "Iron Maiden",
                11 => "Terror",
                12 => "Bone Wall",
                13 => "Golem Mastery",
                14 => "Raise Skeletal Mage",
                15 => "Confuse",
                16 => "Life Tap",
                17 => "Poison Explosion",
                18 => "Bone Spear",
                19 => "Blood Golem",
                20 => "Attract",
                21 => "Decrepify",
                22 => "Bone Prison",
                23 => "Summon Resist",
                24 => "Iron Golem",
                25 => "Lower Resist",
                26 => "Poison Nova",
                27 => "Bone Spirit",
                28 => "Fire Golem",
                29 => "Revive",
                _ => "Unknown",
            },
            CharacterClass::Paladin => match slot {
                0 => "Sacrifice",
                1 => "Smite",
                2 => "Might",
                3 => "Prayer",
                4 => "Resist Fire",
                5 => "Holy Bolt",
                6 => "Holy Fire",
                7 => "Thorns",
                8 => "Defiance",
                9 => "Resist Cold",
                10 => "Zeal",
                11 => "Charge",
                12 => "Blessed Aim",
                13 => "Cleansing",
                14 => "Resist Lightning",
                15 => "Vengeance",
                16 => "Blessed Hammer",
                17 => "Concentration",
                18 => "Holy Freeze",
                19 => "Vigor",
                20 => "Conversion",
                21 => "Holy Shield",
                22 => "Holy Shock",
                23 => "Sanctuary",
                24 => "Meditation",
                25 => "Fist Of The Heavens",
                26 => "Fanaticism",
                27 => "Conviction",
                28 => "Redemption",
                29 => "Salvation",
                _ => "Unknown",
            },
            CharacterClass::Barbarian => match slot {
                0 => "Bash",
                1 => "Sword Mastery",
                2 => "Axe Mastery",
                3 => "Mace Mastery",
                4 => "Polearm Mastery",
                5 => "Throwing Mastery",
                6 => "Spear Mastery",
                7 => "Howl",
                8 => "Find Potion",
                9 => "Leap",
                10 => "Double Swing",
                11 => "Taunt",
                12 => "Shout",
                13 => "Stun",
                14 => "Double Throw",
                15 => "Leap Attack",
                16 => "Concentrate",
                17 => "Iron Skin",
                18 => "Battle Cry",
                19 => "Frenzy",
                20 => "Increased Stamina",
                21 => "Battle Orders",
                22 => "Grim Ward",
                23 => "Whirlwind",
                24 => "Berserk",
                25 => "Natural Resistance",
                26 => "War Cry",
                27 => "Battle Command",
                28 => "Find Item",
                29 => "Command",
                _ => "Unknown",
            },
            CharacterClass::Druid => match slot {
                0 => "Raven",
                1 => "Plague Poppy",
                2 => "Wearbear",
                3 => "Firestorm",
                4 => "Oak Sage",
                5 => "Summon Spirit Wolf",
                6 => "Wearwolf",
                7 => "Shape Shifting",
                8 => "Molten Boulder",
                9 => "Arctic Blast",
                10 => "Fissure",
                11 => "Feral Rage",
                12 => "Maul",
                13 => "Carrion Vine",
                14 => "Heart of Wolverine",
                15 => "Summon Dire Wolf",
                16 => "Rabies",
                17 => "Fire Claws",
                18 => "Twister",
                19 => "Volcano",
                20 => "Tornado",
                21 => "Spirit of Barbs",
                22 => "Summon Grizzly",
                23 => "Fury",
                24 => "Armageddon",
                25 => "Hurricane",
                26 => "Hunger",
                27 => "Shock Wave",
                28 => "Summon Dire Bear",
                29 => "Hurricane",
                _ => "Unknown", // Needs proper correction but good enough for now
            },
            CharacterClass::Assassin => match slot {
                0 => "Fire Blast",
                1 => "Claw Mastery",
                2 => "Psychic Hammer",
                3 => "Tiger Strike",
                4 => "Dragon Talon",
                5 => "Shock Web",
                6 => "Blade Sentinel",
                7 => "Burst of Speed",
                8 => "Fists of Fire",
                9 => "Dragon Claw",
                10 => "Charged Bolt Sentry",
                11 => "Wake of Fire",
                12 => "Weapon Block",
                13 => "Cloak of Shadows",
                14 => "Cobra Strike",
                15 => "Blade Fury",
                16 => "Fade",
                17 => "Shadow Warrior",
                18 => "Claws of Thunder",
                19 => "Dragon Tail",
                20 => "Lightning Sentry",
                21 => "Wake of Inferno",
                22 => "Mind Blast",
                23 => "Blades of Ice",
                24 => "Dragon Flight",
                25 => "Death Sentry",
                26 => "Blade Shield",
                27 => "Venom",
                28 => "Shadow Master",
                29 => "Phoenix Strike",
                _ => "Unknown",
            },
            CharacterClass::Warlock => "Warlock Skill", // Placeholder
        }
    }

    pub fn skill_requirements(class: CharacterClass, slot: usize) -> (u32, Vec<usize>) {
        // Returns (level_requirement, list_of_prerequisite_slots)
        match class {
            CharacterClass::Amazon => match slot {
                // Bow and Crossbow
                0 => (1, vec![]),        // Magic Arrow
                1 => (1, vec![]),        // Fire Arrow
                5 => (6, vec![0]),       // Cold Arrow
                6 => (12, vec![0]),      // Multiple Shot
                10 => (12, vec![1]),     // Exploding Arrow
                16 => (18, vec![0, 6]),  // Guided Arrow
                21 => (24, vec![1, 10]), // Immolation Arrow
                25 => (30, vec![5, 16]), // Freezing Arrow
                20 => (24, vec![6]),     // Strafe
                // ... simplified for others
                _ => (1, vec![]),
            },
            _ => (1, vec![]), // TODO: full mapping
        }
    }

    pub fn can_increase_skill(&self, slot: usize) -> bool {
        if slot >= 30 || self.skill_points_remaining == 0 || self.skills[slot] >= 20 {
            return false;
        }

        let (level_req, prereqs) = Self::skill_requirements(self.class, slot);
        if self.level < level_req {
            return false;
        }

        for &prereq in &prereqs {
            if self.skills[prereq] == 0 {
                return false;
            }
        }

        true
    }

    pub fn increase_skill(&mut self, slot: usize) {
        if self.can_increase_skill(slot) {
            self.skills[slot] += 1;
            self.skill_points_remaining -= 1;
        }
    }

    pub fn toggle_quest(&mut self, difficulty: usize, quest_idx: usize) {
        if difficulty < 3 && quest_idx < 41 {
            let current = self.quests[difficulty][quest_idx];
            // Toggle completed bit (0)
            if current & 1 == 1 {
                // Remove completion and requirement completion
                self.quests[difficulty][quest_idx] &= !3;

                // Refund points if applicable
                // Den of Evil (1), Radament (9), Izual (25) -> 1, 1, 2 skill points
                match quest_idx {
                    1 | 9 => {
                        self.skill_points_remaining = self.skill_points_remaining.saturating_sub(1)
                    }
                    25 => {
                        self.skill_points_remaining = self.skill_points_remaining.saturating_sub(2)
                    }
                    17 => self.stat_points_remaining = self.stat_points_remaining.saturating_sub(5), // Lam Esen
                    _ => {}
                }
            } else {
                // Add completion and requirement completion
                self.quests[difficulty][quest_idx] |= 3;

                // Add points if applicable
                match quest_idx {
                    1 | 9 => self.skill_points_remaining += 1,
                    25 => self.skill_points_remaining += 2,
                    17 => self.stat_points_remaining += 5, // Lam Esen
                    _ => {}
                }
            }
        }
    }

    pub fn decrease_skill(&mut self, slot: usize) {
        if slot < 30 && self.skills[slot] > 0 {
            self.skills[slot] -= 1;
            self.skill_points_remaining += 1;
        }
    }

    pub fn decrease_stat(&mut self, stat: CharacterStat) {
        let base = BaseStats::for_class(self.class);

        match stat {
            CharacterStat::Strength if self.strength > base.str => self.strength -= 1,
            CharacterStat::Dexterity if self.dexterity > base.dex => self.dexterity -= 1,
            CharacterStat::Vitality if self.vitality > base.vit => {
                self.vitality -= 1;
                self.current_hp = self.current_hp.saturating_sub(2).max(base.hp);
                self.max_hp = self.max_hp.saturating_sub(2).max(base.hp);
                self.current_stamina = self.current_stamina.saturating_sub(1).max(base.stamina);
                self.max_stamina = self.max_stamina.saturating_sub(1).max(base.stamina);
            }
            CharacterStat::Energy if self.energy > base.eng => {
                self.energy -= 1;
                self.current_mana = self.current_mana.saturating_sub(2).max(base.mana);
                self.max_mana = self.max_mana.saturating_sub(2).max(base.mana);
            }
            _ => return, // Cannot decrease below base or unsupported stat
        }

        self.stat_points_remaining += 1;
    }

    pub fn calculate_experience_for_level(level: u32) -> u32 {
        if level <= 1 {
            return 0;
        }
        // Simple approximate polynomial for D2 exp curve up to 99
        let breakpoints: [u64; 100] = [
            0,
            500,
            1500,
            3750,
            7875,
            14175,
            22680,
            32886,
            44396,
            57715,
            73364,
            91554,
            112700,
            137351,
            166092,
            199557,
            238555,
            284004,
            336962,
            398674,
            470588,
            554388,
            652033,
            765664,
            897970,
            1052136,
            1231799,
            1441165,
            1685089,
            1964371,
            2289725,
            2668748,
            3110398,
            3624976,
            4224419,
            4923485,
            5737243,
            6685419,
            7789397,
            9075727,
            10574548,
            12320953,
            14355554,
            16725841,
            19487313,
            22704959,
            26453535,
            30821035,
            35909890,
            41838614,
            48746200,
            56795493,
            66173873,
            77100412,
            89831416,
            104664324,
            121946896,
            142084534,
            165545585,
            192882833,
            224731871,
            261838927,
            305085806,
            355476140,
            414187212,
            482588147,
            562283086,
            655132517,
            763319086,
            889373977,
            1036239109,
            1207361831,
            1406734185,
            1638994793,
            1909565574,
            2224749372,
            2591963065,
            3019846356,
            3518335966,
            4104037562,
            4786445581,
            5581452296,
            6507641031,
            7586685233,
            8843818365,
            10308520268,
            12015011701,
            13955310214,
            16183424168,
            18764024343,
            21752945281,
            25215033785,
            29225726244,
            33871403061,
            39252327734,
            45486518175,
            52709249767,
            61076449175,
            70769493134,
            82001460593,
        ];

        let exp = if level <= 99 {
            breakpoints[level as usize - 1]
        } else {
            breakpoints[98]
        };

        // Cap to u32 max (or 3.52B if we wanted exact D2 cap)
        std::cmp::min(exp, u32::MAX as u64) as u32
    }

    pub fn set_level(&mut self, new_level: u32) {
        let old_level = self.level;
        self.level = new_level.clamp(1, 99);
        self.experience = Self::calculate_experience_for_level(self.level);

        // Adjust remaining stat points
        let diff = (self.level as i32) - (old_level as i32);
        if diff > 0 {
            self.stat_points_remaining += (diff as u32) * 5;
            self.skill_points_remaining += diff as u32;
        } else if diff < 0 {
            // Need to reset if we lose levels and can't cover it
            let required_reduction_stats = (-diff as u32) * 5;
            if self.stat_points_remaining >= required_reduction_stats {
                self.stat_points_remaining -= required_reduction_stats;
            } else {
                self.reset_stats();
            }

            let required_reduction_skills = -diff as u32;
            if self.skill_points_remaining >= required_reduction_skills {
                self.skill_points_remaining -= required_reduction_skills;
            } else {
                // TODO: reset skills
            }
        }
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let char_file = CharacterFile::load(path)?;

        let header = char_file.header();
        let class = header.class.unwrap_or(CharacterClass::Amazon);
        let name = header.name.clone();

        // Parse skills
        let mut skills = [0; 30];
        if let Some(char_skills) = char_file.skills() {
            skills.copy_from_slice(&char_skills.levels);
        }

        let raw_bytes = char_file.to_bytes();
        let mut quests = [[0u16; 41]; 3];
        // Parse quests from raw_bytes. Look for "Woo!"
        if let Some(woo_idx) = raw_bytes.windows(4).position(|window| window == b"Woo!") {
            let mut offset = woo_idx + 10; // Skip 10 byte header
            for diff in 0..3 {
                if offset + 96 <= raw_bytes.len() {
                    for i in 0..41 {
                        let b1 = raw_bytes[offset + i * 2] as u16;
                        let b2 = raw_bytes[offset + i * 2 + 1] as u16;
                        quests[diff][i] = b1 | (b2 << 8);
                    }
                    offset += 96;
                }
            }
        }

        let mut waypoints = [[false; 39]; 3];
        if let Some(ws_idx) = raw_bytes.windows(2).position(|window| window == b"WS") {
            let mut offset = ws_idx + 8; // Skip WS, unknown, and length
            for diff in 0..3 {
                if offset + 24 <= raw_bytes.len() {
                    // Each diff block is 24 bytes: 2 bytes header (0x02 0x01) + 22 bytes data
                    let data_offset = offset + 2;
                    for i in 0..39 {
                        let byte_idx = i / 8;
                        let bit_idx = i % 8;
                        waypoints[diff][i] =
                            (raw_bytes[data_offset + byte_idx] & (1 << bit_idx)) != 0;
                    }
                    offset += 24;
                }
            }
        }

        let savegame = Self {
            name,
            class,
            level: char_file.stat(CharacterStat::Level).unwrap_or(1),
            experience: char_file.stat(CharacterStat::Experience).unwrap_or(0),
            gold: char_file.stat(CharacterStat::Gold).unwrap_or(0),
            stashed_gold: char_file.stat(CharacterStat::StashedGold).unwrap_or(0),
            strength: char_file
                .stat(CharacterStat::Strength)
                .unwrap_or_else(|| BaseStats::for_class(class).str),
            dexterity: char_file
                .stat(CharacterStat::Dexterity)
                .unwrap_or_else(|| BaseStats::for_class(class).dex),
            vitality: char_file
                .stat(CharacterStat::Vitality)
                .unwrap_or_else(|| BaseStats::for_class(class).vit),
            energy: char_file
                .stat(CharacterStat::Energy)
                .unwrap_or_else(|| BaseStats::for_class(class).eng),
            stat_points_remaining: char_file.stat(CharacterStat::StatPoints).unwrap_or(0),
            skill_points_remaining: char_file.stat(CharacterStat::SkillPoints).unwrap_or(0),
            current_hp: char_file
                .stat(CharacterStat::HitPoints)
                .unwrap_or_else(|| BaseStats::for_class(class).hp << 8)
                >> 8,
            max_hp: char_file
                .stat(CharacterStat::MaxHitPoints)
                .unwrap_or_else(|| BaseStats::for_class(class).hp << 8)
                >> 8,
            current_mana: char_file
                .stat(CharacterStat::Mana)
                .unwrap_or_else(|| BaseStats::for_class(class).mana << 8)
                >> 8,
            max_mana: char_file
                .stat(CharacterStat::MaxMana)
                .unwrap_or_else(|| BaseStats::for_class(class).mana << 8)
                >> 8,
            current_stamina: char_file
                .stat(CharacterStat::Stamina)
                .unwrap_or_else(|| BaseStats::for_class(class).stamina << 8)
                >> 8,
            max_stamina: char_file
                .stat(CharacterStat::MaxStamina)
                .unwrap_or_else(|| BaseStats::for_class(class).stamina << 8)
                >> 8,
            skills,
            raw_bytes,
            char_file: Some(char_file.clone()),
            quests,
            hardcore: header.status.hardcore,
            died: header.status.died,
            waypoints,
        };

        Ok(savegame)
    }

    pub fn generate_template(class: CharacterClass) -> Self {
        let base = BaseStats::for_class(class);
        let name = format!("Template{}", class);

        let mut raw = vec![0u8; 0x2fd];
        raw[0..4].copy_from_slice(&0xaa55_aa55_u32.to_le_bytes()); // D2S_MAGIC
        raw[4..8].copy_from_slice(&0x60_u32.to_le_bytes()); // VERSION_OFFSET
        raw[0x28] = class as u8; // LEGACY_CLASS_OFFSET
        raw[0x2b] = 99; // LEGACY_LEVEL_OFFSET
        raw[0x24] = 0x20; // Status: Expansion (1 << 5)

        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(15);
        raw[0x14..0x14 + len].copy_from_slice(&name_bytes[..len]);

        // Emulate `build_legacy_fixed_pre_stats_block` criticals
        raw[0x29] = 0x10;
        raw[0x2a] = 0x1e;
        raw[0x34..0x38].fill(0xff);

        // Quests
        raw[0x14b] = 1;
        raw[0x14f..0x153].copy_from_slice(b"Woo!");
        raw[0x153..0x159].copy_from_slice(&[0x06, 0x00, 0x00, 0x00, 0x00, 0x00]);

        // Waypoints
        raw[0x279..0x279 + 2].copy_from_slice(b"WS");
        raw[0x279 + 2..0x279 + 6].copy_from_slice(&[0x01, 0x00, 0x00, 0x00]);
        raw[0x279 + 6..0x279 + 8].copy_from_slice(&[0x50, 0x00]); // 80 bytes len

        // NPC
        raw[0x2cd..0x2cd + 2].copy_from_slice(b"w4");
        raw[0x2cd + 2..0x2cd + 4].copy_from_slice(&[0x34, 0x00]); // 52 bytes len

        Self {
            name,
            class,
            level: 99,
            experience: 3511147413, // Level 99 exp
            gold: 0,
            stashed_gold: 0,
            strength: base.str,
            dexterity: base.dex,
            vitality: base.vit,
            energy: base.eng,
            stat_points_remaining: 5 * 98,
            skill_points_remaining: 98,
            current_hp: base.hp,
            max_hp: base.hp,
            current_mana: base.mana,
            max_mana: base.mana,
            current_stamina: base.stamina,
            max_stamina: base.stamina,
            skills: [0; 30],
            raw_bytes: raw,
            char_file: None, // No valid parsed file yet, but raw_bytes are primed
            quests: [[0; 41]; 3],
            hardcore: false,
            died: false,
            waypoints: [[false; 39]; 3],
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let stats_offset = self
            .char_file
            .as_ref()
            .and_then(|f| f.stats().marker_offset)
            .unwrap_or(765);
        let skills_offset = self
            .char_file
            .as_ref()
            .and_then(|f| f.skills().map(|s| s.marker_offset))
            .unwrap_or(stats_offset + 2); // Fallback

        // Encode Stats
        let mut stats_writer = BitWriter::new();
        let mut write_stat = |stat: CharacterStat, value: u32| {
            stats_writer.write_bits(stat as u32, 9);
            stats_writer.write_bits(value, stat.bit_width() as usize);
        };

        write_stat(CharacterStat::Strength, self.strength);
        write_stat(CharacterStat::Energy, self.energy);
        write_stat(CharacterStat::Dexterity, self.dexterity);
        write_stat(CharacterStat::Vitality, self.vitality);
        write_stat(CharacterStat::StatPoints, self.stat_points_remaining);
        write_stat(CharacterStat::SkillPoints, self.skill_points_remaining);
        write_stat(CharacterStat::HitPoints, self.current_hp << 8); // shift 8 for fractional precision
        write_stat(CharacterStat::MaxHitPoints, self.max_hp << 8);
        write_stat(CharacterStat::Mana, self.current_mana << 8);
        write_stat(CharacterStat::MaxMana, self.max_mana << 8);
        write_stat(CharacterStat::Stamina, self.current_stamina << 8);
        write_stat(CharacterStat::MaxStamina, self.max_stamina << 8);
        write_stat(CharacterStat::Level, self.level);
        write_stat(CharacterStat::Experience, self.experience);
        write_stat(CharacterStat::Gold, self.gold);
        write_stat(CharacterStat::StashedGold, self.stashed_gold);

        stats_writer.write_bits(0x1FF, 9); // Terminator
        let encoded_stats = stats_writer.finish();

        // Build new raw bytes
        let mut new_raw = Vec::new();
        new_raw.extend_from_slice(&self.raw_bytes[0..stats_offset]);
        new_raw.extend_from_slice(b"gf"); // Stats magic
        new_raw.extend_from_slice(&encoded_stats);

        // Encode Skills
        new_raw.extend_from_slice(b"if"); // Skills magic
        new_raw.extend_from_slice(&self.skills);

        // Append everything after the original skills block
        let post_skills_offset = skills_offset + 32;
        if post_skills_offset < self.raw_bytes.len() {
            new_raw.extend_from_slice(&self.raw_bytes[post_skills_offset..]);
        }

        // Apply Quests overrides
        if let Some(woo_idx) = new_raw.windows(4).position(|window| window == b"Woo!") {
            let mut offset = woo_idx + 10;
            for diff in 0..3 {
                if offset + 96 <= new_raw.len() {
                    for i in 0..41 {
                        let word = self.quests[diff][i];
                        new_raw[offset + i * 2] = (word & 0xFF) as u8;
                        new_raw[offset + i * 2 + 1] = (word >> 8) as u8;
                    }
                    offset += 96;
                }
            }
        }

        // Apply Waypoints overrides
        if let Some(ws_idx) = new_raw.windows(2).position(|window| window == b"WS") {
            let mut offset = ws_idx + 8;
            for diff in 0..3 {
                if offset + 24 <= new_raw.len() {
                    let data_offset = offset + 2;
                    for i in 0..39 {
                        let byte_idx = i / 8;
                        let bit_idx = i % 8;
                        if self.waypoints[diff][i] {
                            new_raw[data_offset + byte_idx] |= 1 << bit_idx;
                        } else {
                            new_raw[data_offset + byte_idx] &= !(1 << bit_idx);
                        }
                    }
                    offset += 24;
                }
            }
        }

        fix_header(&mut new_raw);

        Ok(new_raw)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Safety: Always backup before saving if the file already exists
        if path.exists() {
            let mut backup_path = path.to_path_buf();
            backup_path.set_extension("d2s.bak");
            std::fs::copy(path, &backup_path).context("Failed to create backup file")?;
        }

        let bytes = self.to_bytes()?;
        std::fs::write(path, bytes).context("Failed to write savegame file")?;
        Ok(())
    }
}
