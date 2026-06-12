use crate::save::{BitWriter, fix_header};
use anyhow::{Context, Result, anyhow};
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

    pub fn set_level(&mut self, new_level: u32) {
        let old_level = self.level;
        self.level = new_level.clamp(1, 99);

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
            raw_bytes: char_file.to_bytes(),
            char_file: Some(char_file.clone()),
        };

        Ok(savegame)
    }

    pub fn generate_template(class: CharacterClass) -> Self {
        let base = BaseStats::for_class(class);
        Self {
            name: format!("Template{}", class),
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
            raw_bytes: Vec::new(),
            char_file: None, // No valid file
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let char_file = self.char_file.as_ref().ok_or_else(|| {
            anyhow!("Cannot save a template that has no base file yet (TODO: Synthesize full file)")
        })?;

        let stats_offset = char_file.stats().marker_offset.unwrap_or(765);
        let skills_offset = char_file
            .skills()
            .map(|s| s.marker_offset)
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
