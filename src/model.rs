use crate::save::{BitWriter, fix_header};
use anyhow::{Context, Result};
use libd2::core::character_class::CharacterClass;
use libd2::core::character_file::{CharacterFile, CharacterStat};
use libd2::core::character_progression::{
    BaseStats, ClassGrowth, experience_for_level, max_inventory_gold, max_stash_gold,
    skill_points_from_level, stat_points_from_level,
};
use libd2::core::quest::{
    self, SAVE_QUEST_SECTION_HEADER_AFTER_MARKER, SAVE_QUEST_SECTION_HEADER_BYTES,
    SAVE_QUEST_SECTION_MARKER, SAVE_QUEST_WORDS_PER_DIFFICULTY, VISIBLE_QUEST_INDICES,
    apply_progression_from_quests, initial_template_quests, quest_is_completed,
    set_quest_completed, sync_quest_progression,
};
use libd2::core::skills;
use libd2::core::waypoint::{
    self, LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY, LEGACY_WAYPOINT_SECTION_HEADER_AFTER_MARKER,
    LEGACY_WAYPOINT_SECTION_HEADER_BYTES, LEGACY_WAYPOINT_SECTION_MARKER, LEGACY_WAYPOINT_TRAILER,
    LEGACY_WAYPOINT_TRAILER_OFFSET, WAYPOINT_COUNT,
};
use std::path::Path;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, serde_derive::Serialize, serde_derive::Deserialize,
)]
pub enum GameVersion {
    #[default]
    Legacy, // 1.10 - 1.14d
    Resurrected, // 2.5+
    Warlock,     // Reign of the Warlock 3.0+
}

impl std::fmt::Display for GameVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Legacy => write!(f, "1.10 - 1.14d"),
            Self::Resurrected => write!(f, "Resurrected 2.5+"),
            Self::Warlock => write!(f, "Reign of the Warlock 3.0+"),
        }
    }
}

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

    // Quests for Normal, Nightmare, Hell.
    pub quests: [[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3],

    pub hardcore: bool,
    pub died: bool,

    // Waypoints for Normal, Nightmare, Hell.
    pub waypoints: [[bool; WAYPOINT_COUNT]; 3],

    pub game_version: GameVersion,
}

impl Savegame {
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
        let quests = quest::parse_legacy_quest_words(&raw_bytes)
            .unwrap_or([[0u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3]);
        let waypoints =
            waypoint::parse_legacy_waypoints(&raw_bytes).unwrap_or([[false; WAYPOINT_COUNT]; 3]);

        let mut game_version = GameVersion::Legacy;
        if header.version_raw >= 0x61 {
            game_version = GameVersion::Resurrected;
        }

        let mut savegame = Self {
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
            game_version,
        };
        savegame.clamp_gold();
        savegame.recalculate_remaining_points_from_allocations();

        Ok(savegame)
    }

    pub fn generate_template(class: CharacterClass) -> Self {
        let base = BaseStats::for_class(class);
        let name = class.to_string();

        let mut raw = vec![0u8; 0x2fd];
        raw[0..4].copy_from_slice(&0xaa55_aa55_u32.to_le_bytes()); // D2S_MAGIC
        raw[4..8].copy_from_slice(&0x60_u32.to_le_bytes()); // VERSION_OFFSET
        raw[0x28] = class as u8; // LEGACY_CLASS_OFFSET
        raw[0x2b] = 99; // LEGACY_LEVEL_OFFSET
        raw[0x24] = 0x20; // Status: Expansion (1 << 5)

        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(15);
        raw[0x14..0x14 + len].copy_from_slice(&name_bytes[..len]);

        raw[0x29] = 0x10;
        raw[0x2a] = 0x1e;
        raw[0x34..0x38].fill(0xff);

        // Quests
        raw[0x14b] = 1;
        raw[0x14f..0x14f + SAVE_QUEST_SECTION_MARKER.len()]
            .copy_from_slice(&SAVE_QUEST_SECTION_MARKER);
        raw[0x14f + SAVE_QUEST_SECTION_MARKER.len()..0x14f + SAVE_QUEST_SECTION_HEADER_BYTES]
            .copy_from_slice(&SAVE_QUEST_SECTION_HEADER_AFTER_MARKER);

        // Waypoints
        raw[0x279..0x279 + LEGACY_WAYPOINT_SECTION_MARKER.len()]
            .copy_from_slice(&LEGACY_WAYPOINT_SECTION_MARKER);
        raw[0x279 + LEGACY_WAYPOINT_SECTION_MARKER.len()
            ..0x279 + LEGACY_WAYPOINT_SECTION_HEADER_BYTES]
            .copy_from_slice(&LEGACY_WAYPOINT_SECTION_HEADER_AFTER_MARKER);
        for diff in 0..3 {
            let offset = 0x281 + diff * LEGACY_WAYPOINT_BYTES_PER_DIFFICULTY;
            raw[offset] = 0x02;
            raw[offset + 1] = 0x01;
        }
        raw[0x279 + LEGACY_WAYPOINT_TRAILER_OFFSET] = LEGACY_WAYPOINT_TRAILER;

        // NPC
        raw[0x2ca..0x2ca + 2].copy_from_slice(b"w4");
        raw[0x2ca + 2..0x2ca + 4].copy_from_slice(&[0x34, 0x00]); // 52 bytes len

        let quests = initial_template_quests();

        Self {
            name,
            class,
            level: 99,
            experience: experience_for_level(99),
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
            char_file: None,
            quests,
            hardcore: false,
            died: false,
            waypoints: [[false; WAYPOINT_COUNT]; 3],
            game_version: GameVersion::Legacy,
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
            .unwrap_or(stats_offset + 2);

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
        write_stat(CharacterStat::HitPoints, self.current_hp << 8);
        write_stat(CharacterStat::MaxHitPoints, self.max_hp << 8);
        write_stat(CharacterStat::Mana, self.current_mana << 8);
        write_stat(CharacterStat::MaxMana, self.max_mana << 8);
        write_stat(CharacterStat::Stamina, self.current_stamina << 8);
        write_stat(CharacterStat::MaxStamina, self.max_stamina << 8);
        write_stat(CharacterStat::Level, self.level);
        write_stat(CharacterStat::Experience, self.experience);
        write_stat(
            CharacterStat::Gold,
            self.gold.min(self.max_inventory_gold()),
        );
        write_stat(
            CharacterStat::StashedGold,
            self.stashed_gold.min(self.max_stash_gold()),
        );

        stats_writer.write_bits(0x1FF, 9);
        let encoded_stats = stats_writer.finish();

        let mut new_raw = Vec::new();
        new_raw.extend_from_slice(&self.raw_bytes[0..stats_offset]);
        new_raw.extend_from_slice(b"gf");
        new_raw.extend_from_slice(&encoded_stats);

        new_raw.extend_from_slice(b"if");
        new_raw.extend_from_slice(&self.skills);

        let post_skills_offset = skills_offset + 32;
        if post_skills_offset < self.raw_bytes.len() {
            new_raw.extend_from_slice(&self.raw_bytes[post_skills_offset..]);
        } else {
            new_raw.extend_from_slice(b"JM");
            new_raw.extend_from_slice(&0u16.to_le_bytes());
            new_raw.extend_from_slice(b"JM");
            new_raw.extend_from_slice(&0u16.to_le_bytes());
            new_raw.extend_from_slice(b"jf");
            new_raw.extend_from_slice(b"kf");
            new_raw.push(0);
        }

        if new_raw.len() >= 0x2b {
            new_raw[0x2b] = self.level as u8;
            new_raw[0x28] = self.class as u8;
            let mut status = new_raw[0x24];
            if self.hardcore {
                status |= 0x04;
            } else {
                status &= !0x04;
            }
            if self.died {
                status |= 0x08;
            } else {
                status &= !0x08;
            }
            status |= 0x20; // Always set Expansion bit for modern saves
            new_raw[0x24] = status;

            let name_bytes = self.name.as_bytes();
            let len = name_bytes.len().min(15);
            new_raw[0x14..0x14 + 16].fill(0);
            new_raw[0x14..0x14 + len].copy_from_slice(&name_bytes[..len]);
        }

        let mut quests = self.quests;
        for difficulty in &mut quests {
            sync_quest_progression(difficulty);
        }

        quest::write_legacy_quest_words(&mut new_raw, &quests);
        apply_progression_from_quests(&mut new_raw, &quests);

        waypoint::write_legacy_waypoints(&mut new_raw, &self.waypoints);

        fix_header(&mut new_raw);
        Ok(new_raw)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if path.exists() {
            let mut backup_path = path.to_path_buf();
            backup_path.set_extension("d2s.bak");
            std::fs::copy(path, &backup_path).context("Failed to create backup file")?;
        }
        let bytes = self.to_bytes()?;
        std::fs::write(path, bytes).context("Failed to write savegame file")?;
        Ok(())
    }

    pub fn total_allowed_stat_points(&self) -> u32 {
        stat_points_from_level(self.level) + quest::stat_points_from_quests(&self.quests)
    }

    pub fn total_allowed_skill_points(&self) -> u32 {
        skill_points_from_level(self.level) + quest::skill_points_from_quests(&self.quests)
    }

    fn spent_stat_points(&self) -> u32 {
        let base = BaseStats::for_class(self.class);
        self.strength.saturating_sub(base.str)
            + self.dexterity.saturating_sub(base.dex)
            + self.vitality.saturating_sub(base.vit)
            + self.energy.saturating_sub(base.eng)
    }

    fn spent_skill_points(&self) -> u32 {
        self.skills.iter().map(|&level| level as u32).sum()
    }

    fn normalize_point_totals(&mut self) {
        let allowed_stats = self.total_allowed_stat_points();
        let spent_stats = self.spent_stat_points();
        if spent_stats <= allowed_stats {
            self.stat_points_remaining = allowed_stats - spent_stats;
        } else {
            self.reset_stats();
        }

        let allowed_skills = self.total_allowed_skill_points();
        let spent_skills = self.spent_skill_points();
        if spent_skills <= allowed_skills {
            self.skill_points_remaining = allowed_skills - spent_skills;
        } else {
            self.reset_skills();
        }
    }

    fn recalculate_remaining_points_from_allocations(&mut self) {
        self.stat_points_remaining = self
            .total_allowed_stat_points()
            .saturating_sub(self.spent_stat_points());
        self.skill_points_remaining = self
            .total_allowed_skill_points()
            .saturating_sub(self.spent_skill_points());
    }

    pub fn reset_stats(&mut self) {
        let base = BaseStats::for_class(self.class);
        self.strength = base.str;
        self.dexterity = base.dex;
        self.vitality = base.vit;
        self.energy = base.eng;
        self.stat_points_remaining = self.total_allowed_stat_points();

        self.current_hp = base.hp;
        self.max_hp = base.hp;
        self.current_mana = base.mana;
        self.max_mana = base.mana;
        self.current_stamina = base.stamina;
        self.max_stamina = base.stamina;
    }

    pub fn set_level(&mut self, new_level: u32) {
        let old_level = self.level;
        self.level = new_level.clamp(1, 99);
        self.clamp_gold();
        self.experience = experience_for_level(self.level);
        if self.level != old_level {
            self.normalize_point_totals();
        }
    }

    pub fn increase_stat(&mut self, stat: CharacterStat, amount: u32) {
        let actual_amount = amount.min(self.stat_points_remaining);
        if actual_amount == 0 {
            return;
        }
        let growth = ClassGrowth::for_class(self.class);
        match stat {
            CharacterStat::Strength => self.strength += actual_amount,
            CharacterStat::Dexterity => self.dexterity += actual_amount,
            CharacterStat::Vitality => {
                self.vitality += actual_amount;
                self.current_hp += actual_amount * growth.whole_life_per_vitality();
                self.max_hp += actual_amount * growth.whole_life_per_vitality();
                self.current_stamina += actual_amount * growth.whole_stamina_per_vitality();
                self.max_stamina += actual_amount * growth.whole_stamina_per_vitality();
            }
            CharacterStat::Energy => {
                self.energy += actual_amount;
                self.current_mana += actual_amount * growth.whole_mana_per_energy();
                self.max_mana += actual_amount * growth.whole_mana_per_energy();
            }
            _ => return,
        }
        self.stat_points_remaining -= actual_amount;
    }

    pub fn decrease_stat(&mut self, stat: CharacterStat, amount: u32) {
        let base = BaseStats::for_class(self.class);
        let growth = ClassGrowth::for_class(self.class);
        match stat {
            CharacterStat::Strength => {
                let diff = self.strength.saturating_sub(base.str).min(amount);
                self.strength -= diff;
                self.stat_points_remaining += diff;
            }
            CharacterStat::Dexterity => {
                let diff = self.dexterity.saturating_sub(base.dex).min(amount);
                self.dexterity -= diff;
                self.stat_points_remaining += diff;
            }
            CharacterStat::Vitality => {
                let diff = self.vitality.saturating_sub(base.vit).min(amount);
                self.vitality -= diff;
                self.current_hp = self
                    .current_hp
                    .saturating_sub(diff * growth.whole_life_per_vitality())
                    .max(base.hp);
                self.max_hp = self
                    .max_hp
                    .saturating_sub(diff * growth.whole_life_per_vitality())
                    .max(base.hp);
                self.current_stamina = self
                    .current_stamina
                    .saturating_sub(diff * growth.whole_stamina_per_vitality())
                    .max(base.stamina);
                self.max_stamina = self
                    .max_stamina
                    .saturating_sub(diff * growth.whole_stamina_per_vitality())
                    .max(base.stamina);
                self.stat_points_remaining += diff;
            }
            CharacterStat::Energy => {
                let diff = self.energy.saturating_sub(base.eng).min(amount);
                self.energy -= diff;
                self.current_mana = self
                    .current_mana
                    .saturating_sub(diff * growth.whole_mana_per_energy())
                    .max(base.mana);
                self.max_mana = self
                    .max_mana
                    .saturating_sub(diff * growth.whole_mana_per_energy())
                    .max(base.mana);
                self.stat_points_remaining += diff;
            }
            _ => {}
        }
    }

    pub fn minimize_stat(&mut self, stat: CharacterStat) {
        self.decrease_stat(stat, u32::MAX);
    }

    pub fn maximize_stat(&mut self, stat: CharacterStat) {
        self.increase_stat(stat, self.stat_points_remaining);
    }

    pub fn base_resistance_bonus(&self) -> u32 {
        quest::base_resistance_bonus(&self.quests)
    }

    /// Returns whether the editor should present this save as currently dead.
    ///
    /// The softcore header died bit can be set on living characters. True
    /// softcore corpse state is stored in the later corpse item section, which
    /// d2sed does not parse yet.
    pub fn is_dead_for_display(&self) -> bool {
        self.hardcore && self.died
    }

    pub fn set_gold(&mut self, gold: u32) {
        self.gold = gold.min(self.max_inventory_gold());
    }

    pub fn set_stashed_gold(&mut self, stashed_gold: u32) {
        self.stashed_gold = stashed_gold.min(self.max_stash_gold());
    }

    pub fn max_inventory_gold(&self) -> u32 {
        max_inventory_gold(self.level)
    }

    pub fn max_stash_gold(&self) -> u32 {
        max_stash_gold(self.level)
    }

    fn clamp_gold(&mut self) {
        self.gold = self.gold.min(self.max_inventory_gold());
        self.stashed_gold = self.stashed_gold.min(self.max_stash_gold());
    }

    pub fn set_name(&mut self, new_name: String) {
        let name = new_name.chars().take(15).collect::<String>();
        self.name = name;
    }

    pub fn reset_skills(&mut self) {
        for level in &mut self.skills {
            *level = 0;
        }
        self.skill_points_remaining = self.total_allowed_skill_points();
    }

    pub fn toggle_all_waypoints(&mut self, difficulty: Option<usize>, state: bool) {
        match difficulty {
            Some(diff) if diff < 3 => {
                for wp in &mut self.waypoints[diff] {
                    *wp = state;
                }
            }
            None => {
                for diff in 0..3 {
                    for wp in &mut self.waypoints[diff] {
                        *wp = state;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn toggle_all_quests(&mut self, difficulty: Option<usize>, state: bool) {
        match difficulty {
            Some(diff) if diff < 3 => {
                for &idx in &VISIBLE_QUEST_INDICES {
                    let is_completed = quest_is_completed(self.quests[diff][idx]);
                    if is_completed != state {
                        self.toggle_quest(diff, idx);
                    }
                }
            }
            None => {
                for diff in 0..3 {
                    for &idx in &VISIBLE_QUEST_INDICES {
                        let is_completed = quest_is_completed(self.quests[diff][idx]);
                        if is_completed != state {
                            self.toggle_quest(diff, idx);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn can_increase_skill(&self, slot: usize) -> bool {
        skills::can_increase_skill(
            self.class,
            self.level,
            &self.skills,
            self.skill_points_remaining,
            slot,
        )
    }

    pub fn increase_skill(&mut self, slot: usize) {
        skills::increase_skill(
            self.class,
            self.level,
            &mut self.skills,
            &mut self.skill_points_remaining,
            slot,
        );
    }

    pub fn can_decrease_skill(&self, slot: usize) -> bool {
        skills::can_decrease_skill(self.class, &self.skills, slot)
    }

    pub fn decrease_skill(&mut self, slot: usize) {
        skills::decrease_skill(
            self.class,
            &mut self.skills,
            &mut self.skill_points_remaining,
            slot,
        );
    }
    pub fn toggle_quest(&mut self, difficulty: usize, quest_idx: usize) {
        if difficulty < 3 && quest_idx < SAVE_QUEST_WORDS_PER_DIFFICULTY {
            let current = self.quests[difficulty][quest_idx];
            if quest_is_completed(current) {
                set_quest_completed(&mut self.quests[difficulty][quest_idx], false);
                self.skill_points_remaining = self
                    .skill_points_remaining
                    .saturating_sub(quest::skill_points_reward_for_quest(quest_idx));
                self.stat_points_remaining = self
                    .stat_points_remaining
                    .saturating_sub(quest::stat_points_reward_for_quest(quest_idx));
            } else {
                set_quest_completed(&mut self.quests[difficulty][quest_idx], true);
                self.skill_points_remaining += quest::skill_points_reward_for_quest(quest_idx);
                self.stat_points_remaining += quest::stat_points_reward_for_quest(quest_idx);
            }
            sync_quest_progression(&mut self.quests[difficulty]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libd2::core::quest::{
        ACT_IV_COMPLETE, ACT_V_COMPLETE, ACT_V_INTRO, DIFFICULTY_COMPLETED_WORD,
        EVE_OF_DESTRUCTION, LEGACY_PROGRESSION_OFFSET, PRISON_OF_ICE, PROGRESSION_HELL_COMPLETED,
        PROGRESSION_NORMAL_UNLOCKED, QUEST_LOG_CLOSED, QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
        QUEST_REWARD_GRANTED, QUEST_REWARD_PENDING, TERRORS_END,
    };

    #[test]
    fn level_99_template_uses_exact_experience_breakpoint() {
        let save = Savegame::generate_template(CharacterClass::Amazon);

        assert_eq!(save.level, 99);
        assert_eq!(save.experience, experience_for_level(99));
    }

    #[test]
    fn paladin_holy_shield_adds_recursive_prerequisites() {
        let mut save = Savegame::generate_template(CharacterClass::Paladin);

        save.increase_skill(21);

        for slot in [1, 5, 11, 16, 21] {
            assert_eq!(
                save.skills[slot],
                1,
                "{} should receive one hard point",
                skills::skill_name(save.class, slot)
            );
        }
        assert_eq!(save.skill_points_remaining, 93);
    }

    #[test]
    fn advanced_skill_requires_enough_points_for_prerequisites() {
        let mut save = Savegame::generate_template(CharacterClass::Paladin);
        save.skill_points_remaining = 4;

        assert!(!save.can_increase_skill(21));
        save.increase_skill(21);

        assert_eq!(save.skills.iter().copied().sum::<u8>(), 0);
        assert_eq!(save.skill_points_remaining, 4);
    }

    #[test]
    fn last_prerequisite_point_cannot_be_removed_while_dependent_is_allocated() {
        let mut save = Savegame::generate_template(CharacterClass::Paladin);
        save.increase_skill(21);

        assert!(!save.can_decrease_skill(1));
        save.decrease_skill(1);

        assert_eq!(save.skills[1], 1);
        assert_eq!(save.skill_points_remaining, 93);
    }

    #[test]
    fn extra_prerequisite_points_can_be_removed_back_to_one() {
        let mut save = Savegame::generate_template(CharacterClass::Paladin);
        save.increase_skill(21);
        save.increase_skill(1);

        assert!(save.can_decrease_skill(1));
        save.decrease_skill(1);

        assert_eq!(save.skills[1], 1);
        assert_eq!(save.skill_points_remaining, 93);
    }

    #[test]
    fn level_min_and_max_recompute_remaining_points() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);
        save.increase_stat(CharacterStat::Strength, 10);
        save.increase_skill(0);

        save.set_level(1);

        let base = BaseStats::for_class(CharacterClass::Amazon);
        assert_eq!(save.level, 1);
        assert_eq!(save.experience, 0);
        assert_eq!(save.strength, base.str);
        assert_eq!(save.stat_points_remaining, 0);
        assert_eq!(save.skills.iter().copied().sum::<u8>(), 0);
        assert_eq!(save.skill_points_remaining, 0);

        save.set_level(99);

        assert_eq!(save.level, 99);
        assert_eq!(save.experience, experience_for_level(99));
        assert_eq!(save.stat_points_remaining, 490);
        assert_eq!(save.skill_points_remaining, 98);
    }

    #[test]
    fn stat_min_and_max_move_points_between_stat_and_pool() {
        let mut save = Savegame::generate_template(CharacterClass::Sorceress);
        let base = BaseStats::for_class(CharacterClass::Sorceress);
        save.stat_points_remaining = 20;

        save.maximize_stat(CharacterStat::Energy);

        assert_eq!(save.energy, base.eng + 20);
        assert_eq!(save.current_mana, base.mana + 40);
        assert_eq!(save.max_mana, base.mana + 40);
        assert_eq!(save.stat_points_remaining, 0);

        save.minimize_stat(CharacterStat::Energy);

        assert_eq!(save.energy, base.eng);
        assert_eq!(save.current_mana, base.mana);
        assert_eq!(save.max_mana, base.mana);
        assert_eq!(save.stat_points_remaining, 20);
    }

    #[test]
    fn point_normalization_preserves_loaded_allocations() {
        let mut save = Savegame::generate_template(CharacterClass::Paladin);
        save.strength += 25;
        save.dexterity += 10;
        save.skills[0] = 1;
        save.skills[1] = 3;
        save.stat_points_remaining = 999;
        save.skill_points_remaining = 999;

        save.normalize_point_totals();

        assert_eq!(
            save.stat_points_remaining,
            save.total_allowed_stat_points() - 35
        );
        assert_eq!(
            save.skill_points_remaining,
            save.total_allowed_skill_points() - 4
        );
        assert_eq!(
            save.strength,
            BaseStats::for_class(CharacterClass::Paladin).str + 25
        );
        assert_eq!(save.skills[1], 3);
    }

    #[test]
    fn loaded_over_budget_allocations_are_preserved_with_zero_remaining_points() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);
        save.set_level(1);
        save.strength += 25;
        save.skills[0] = 1;

        save.recalculate_remaining_points_from_allocations();

        assert_eq!(
            save.strength,
            BaseStats::for_class(CharacterClass::Amazon).str + 25
        );
        assert_eq!(save.skills[0], 1);
        assert_eq!(save.stat_points_remaining, 0);
        assert_eq!(save.skill_points_remaining, 0);
    }

    #[test]
    fn base_resistance_bonus_tracks_consumed_resistance_scrolls() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);

        assert_eq!(
            quest::consumed_resistance_scrolls(&save.quests),
            [false, false, false]
        );
        assert_eq!(save.base_resistance_bonus(), 0);

        save.toggle_quest(0, PRISON_OF_ICE);

        assert_eq!(
            quest::consumed_resistance_scrolls(&save.quests),
            [true, false, false]
        );
        assert_eq!(save.base_resistance_bonus(), 10);

        save.toggle_all_quests(None, true);

        assert_eq!(
            quest::consumed_resistance_scrolls(&save.quests),
            [true, true, true]
        );
        assert_eq!(save.base_resistance_bonus(), 30);
    }

    #[test]
    fn softcore_died_status_bit_is_not_display_dead() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);
        save.hardcore = false;
        save.died = true;

        assert!(!save.is_dead_for_display());
    }

    #[test]
    fn hardcore_died_status_bit_is_display_dead() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);
        save.hardcore = true;
        save.died = true;

        assert!(save.is_dead_for_display());
    }

    #[test]
    fn completing_reward_quest_grants_reward_without_leaving_it_pending() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);

        save.toggle_quest(0, 25);

        assert!(quest_is_completed(save.quests[0][25]));
        assert_eq!(save.quests[0][25] & QUEST_REWARD_PENDING, 0);
        assert_eq!(save.quests[0][25] & QUEST_LOG_CLOSED, QUEST_LOG_CLOSED);
        assert_eq!(save.skill_points_remaining, 100);
    }

    #[test]
    fn toggle_all_quests_sets_hidden_act_progression_words() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);

        save.toggle_all_quests(None, true);

        for diff in 0..3 {
            for &idx in &VISIBLE_QUEST_INDICES {
                assert!(
                    quest_is_completed(save.quests[diff][idx]),
                    "difficulty {diff} quest index {idx} should be completed"
                );
                assert_eq!(
                    save.quests[diff][idx] & QUEST_REWARD_PENDING,
                    0,
                    "difficulty {diff} quest index {idx} should not be reward-pending"
                );
                assert_eq!(
                    save.quests[diff][idx] & QUEST_LOG_CLOSED,
                    QUEST_LOG_CLOSED,
                    "difficulty {diff} quest index {idx} should be closed in quest history"
                );
            }

            assert_eq!(save.quests[diff][ACT_IV_COMPLETE], QUEST_REWARD_GRANTED);
            assert_eq!(save.quests[diff][ACT_V_INTRO], QUEST_REWARD_GRANTED);
            assert_eq!(save.quests[diff][ACT_V_COMPLETE], DIFFICULTY_COMPLETED_WORD);
            assert_eq!(
                save.quests[diff][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
                QUEST_PRISON_OF_ICE_SCROLL_CONSUMED
            );
        }
    }

    #[test]
    fn prison_of_ice_completion_marks_resistance_scroll_consumed() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);

        save.toggle_quest(0, PRISON_OF_ICE);

        assert_eq!(
            save.quests[0][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
            QUEST_PRISON_OF_ICE_SCROLL_CONSUMED
        );

        save.toggle_quest(0, PRISON_OF_ICE);

        assert_eq!(
            save.quests[0][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
            0
        );
    }

    #[test]
    fn to_bytes_sanitizes_old_pending_reward_bits_and_syncs_progression() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);
        save.quests[0][25] = QUEST_REWARD_GRANTED | QUEST_REWARD_PENDING | QUEST_LOG_CLOSED;
        save.quests[0][TERRORS_END] = QUEST_REWARD_GRANTED | QUEST_REWARD_PENDING;
        save.quests[0][EVE_OF_DESTRUCTION] = QUEST_REWARD_GRANTED | QUEST_REWARD_PENDING;

        let bytes = save.to_bytes().expect("template should serialize");
        let quests = quest_words(&bytes);

        assert_eq!(quests[0][25] & QUEST_REWARD_PENDING, 0);
        assert_eq!(quests[0][25] & QUEST_LOG_CLOSED, QUEST_LOG_CLOSED);
        assert_eq!(quests[0][TERRORS_END] & QUEST_REWARD_PENDING, 0);
        assert_eq!(quests[0][TERRORS_END] & QUEST_LOG_CLOSED, QUEST_LOG_CLOSED);
        assert_eq!(quests[0][EVE_OF_DESTRUCTION] & QUEST_REWARD_PENDING, 0);
        assert_eq!(
            quests[0][EVE_OF_DESTRUCTION] & QUEST_LOG_CLOSED,
            QUEST_LOG_CLOSED
        );
        assert_eq!(quests[0][ACT_IV_COMPLETE], QUEST_REWARD_GRANTED);
        assert_eq!(quests[0][ACT_V_INTRO], QUEST_REWARD_GRANTED);
        assert_eq!(quests[0][ACT_V_COMPLETE], DIFFICULTY_COMPLETED_WORD);
        assert_eq!(
            bytes[LEGACY_PROGRESSION_OFFSET],
            PROGRESSION_NORMAL_UNLOCKED
        );
    }

    #[test]
    fn to_bytes_sets_progression_for_completed_difficulties() {
        let mut save = Savegame::generate_template(CharacterClass::Amazon);

        save.toggle_all_quests(None, true);

        let bytes = save.to_bytes().expect("template should serialize");
        let quests = quest_words(&bytes);

        assert_eq!(bytes[LEGACY_PROGRESSION_OFFSET], PROGRESSION_HELL_COMPLETED);
        for difficulty in &quests {
            assert_eq!(difficulty[ACT_V_COMPLETE], DIFFICULTY_COMPLETED_WORD);
            assert_eq!(
                difficulty[PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED,
                QUEST_PRISON_OF_ICE_SCROLL_CONSUMED
            );
        }
    }

    #[test]
    fn gold_setters_clamp_to_legacy_caps() {
        let mut save = Savegame::generate_template(CharacterClass::Necromancer);

        save.set_gold(9_999_990);
        save.set_stashed_gold(9_999_999);

        assert_eq!(save.gold, 990_000);
        assert_eq!(save.stashed_gold, 2_500_000);

        save.set_level(1);

        assert_eq!(save.gold, 10_000);
        assert_eq!(save.stashed_gold, 50_000);
    }

    #[test]
    fn to_bytes_serializes_clamped_gold_values() {
        let mut save = Savegame::generate_template(CharacterClass::Necromancer);
        save.gold = 9_999_990;
        save.stashed_gold = 9_999_999;

        let bytes = save.to_bytes().expect("template should serialize");

        assert_eq!(stat_value(&bytes, CharacterStat::Gold), Some(990_000));
        assert_eq!(
            stat_value(&bytes, CharacterStat::StashedGold),
            Some(2_500_000)
        );
    }

    fn quest_words(bytes: &[u8]) -> [[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3] {
        quest::parse_legacy_quest_words(bytes).expect("quest header should exist")
    }

    fn stat_value(bytes: &[u8], target: CharacterStat) -> Option<u32> {
        let marker_offset = bytes.windows(2).position(|window| window == b"gf")?;
        let mut bit_offset = (marker_offset + 2) * 8;
        for _ in 0..64 {
            let id = read_bits(bytes, bit_offset, 9)?;
            bit_offset += 9;
            if id == 0x1ff {
                return None;
            }

            let stat = CharacterStat::from_id(id as u16)?;
            let value = read_bits(bytes, bit_offset, stat.bit_width() as usize)?;
            bit_offset += stat.bit_width() as usize;
            if stat == target {
                return Some(value);
            }
        }
        None
    }

    fn read_bits(bytes: &[u8], bit_offset: usize, count: usize) -> Option<u32> {
        if bit_offset + count > bytes.len() * 8 {
            return None;
        }

        let mut value = 0u32;
        for index in 0..count {
            let absolute_bit = bit_offset + index;
            let byte = *bytes.get(absolute_bit / 8)?;
            if byte & (1 << (absolute_bit % 8)) != 0 {
                value |= 1 << index;
            }
        }
        Some(value)
    }
}
