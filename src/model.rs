use crate::save::BitWriter;
use anyhow::{Context, Result};
use libd2::core::character_class::CharacterClass;
use libd2::core::character_file::{CharacterFile, CharacterProgression, CharacterStat};
use libd2::core::character_progression::{
    BaseStats, ClassGrowth, experience_for_level, max_inventory_gold, max_stash_gold,
    skill_points_from_level, stat_points_from_level,
};
use libd2::core::quest::{
    self, SAVE_QUEST_WORDS_PER_DIFFICULTY, VISIBLE_QUEST_INDICES, initial_template_quests,
    progression_from_quests, quest_is_completed, set_quest_completed, sync_quest_progression,
};
use libd2::core::skills;
use libd2::core::version::{CharacterStatus, ExpansionMode};
use libd2::core::waypoint::{self, WAYPOINT_COUNT};
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
        let quests = quest::parse_legacy_quest_words(&raw_bytes, 0)
            .unwrap_or([[0u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3]);
        let waypoints =
            waypoint::parse_legacy_waypoints(&raw_bytes, 0).unwrap_or([[false; WAYPOINT_COUNT]; 3]);

        let mut game_version = GameVersion::Legacy;
        if header.version_raw >= 0x61 {
            if header.expansion_mode == ExpansionMode::RotW || class == CharacterClass::Warlock {
                game_version = GameVersion::Warlock;
            } else {
                game_version = GameVersion::Resurrected;
            }
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
        let mut save = Self::generate_blank_template(class);

        // Upgrade to level 99
        save.set_level(99);
        save.toggle_all_quests(None, true);
        save.toggle_all_waypoints(None, true);

        save.normalize_point_totals();
        save.recalculate_vitals();

        save
    }

    pub fn generate_blank_template(class: CharacterClass) -> Self {
        let base = BaseStats::for_class(class);
        let name = class.to_string();

        let char_file = CharacterFile::default_rotw(class, &name)
            .expect("libd2 should generate valid RotW blank character");

        let quests = initial_template_quests();

        let game_version = GameVersion::Warlock; // Default templates to RotW

        Self {
            name,
            class,
            level: 1,
            experience: 0,
            gold: 0,
            stashed_gold: 0,
            strength: base.str,
            dexterity: base.dex,
            vitality: base.vit,
            energy: base.eng,
            stat_points_remaining: 0,
            skill_points_remaining: 0,
            current_hp: base.hp,
            max_hp: base.hp,
            current_mana: base.mana,
            max_mana: base.mana,
            current_stamina: base.stamina,
            max_stamina: base.stamina,
            skills: [0; 30],
            char_file: Some(char_file),
            quests,
            hardcore: false,
            died: false,
            waypoints: [[false; WAYPOINT_COUNT]; 3],
            game_version,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut char_file = self.char_file.clone().unwrap();

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

        let mut quests = self.quests;
        for difficulty in &mut quests {
            sync_quest_progression(difficulty);
        }

        let status = CharacterStatus {
            hardcore: self.hardcore,
            died: self.died,
            expansion: if char_file.header().layout.uses_v105_mode_marker() {
                false
            } else {
                char_file.header().status.expansion
            },
            ladder: char_file.header().status.ladder,
        };

        char_file.set_header_fields(
            &self.name,
            status,
            self.class,
            self.level as u8,
            Some(CharacterProgression::from_v105_byte(
                progression_from_quests(&quests),
            )),
        )?;

        // Ensure the raw version byte is updated if upgraded
        let current_version_raw = char_file.header().version_raw;
        let new_version_raw: u32 = match self.game_version {
            GameVersion::Legacy => 0x60,
            GameVersion::Resurrected => {
                if current_version_raw >= 0x61 {
                    current_version_raw
                } else {
                    0x62
                }
            }
            GameVersion::Warlock => {
                if current_version_raw >= 0x69 {
                    current_version_raw
                } else {
                    0x69
                }
            }
        };

        if current_version_raw != new_version_raw {
            let mut raw = char_file.into_raw_bytes();
            raw[0x04..0x08].copy_from_slice(&new_version_raw.to_le_bytes());
            crate::save::fix_header(&mut raw);
            char_file = CharacterFile::parse(raw)?;
        }

        if char_file.header().layout.uses_v105_mode_marker() {
            let expansion_mode = match self.game_version {
                GameVersion::Warlock => ExpansionMode::RotW,
                GameVersion::Resurrected => ExpansionMode::Expansion,
                GameVersion::Legacy => char_file.header().expansion_mode,
            };
            char_file.set_expansion_mode(expansion_mode)?;
        }

        char_file.replace_stats_and_skills(&encoded_stats, &self.skills)?;
        char_file.replace_quests(&quests)?;
        char_file.replace_waypoints(&self.waypoints)?;

        Ok(char_file.to_bytes())
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

    pub fn recalculate_remaining_points_from_allocations(&mut self) {
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
            self.recalculate_vitals();
        }
    }

    pub fn increase_stat(&mut self, stat: CharacterStat, amount: u32) {
        let actual_amount = amount.min(self.stat_points_remaining);
        if actual_amount == 0 {
            return;
        }
        match stat {
            CharacterStat::Strength => self.strength += actual_amount,
            CharacterStat::Dexterity => self.dexterity += actual_amount,
            CharacterStat::Vitality => {
                self.vitality += actual_amount;
            }
            CharacterStat::Energy => {
                self.energy += actual_amount;
            }
            _ => return,
        }
        self.stat_points_remaining -= actual_amount;
        self.recalculate_vitals();
    }

    pub fn decrease_stat(&mut self, stat: CharacterStat, amount: u32) {
        let base = BaseStats::for_class(self.class);
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
                self.stat_points_remaining += diff;
            }
            CharacterStat::Energy => {
                let diff = self.energy.saturating_sub(base.eng).min(amount);
                self.energy -= diff;
                self.stat_points_remaining += diff;
            }
            _ => {}
        }
        self.recalculate_vitals();
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

    pub fn set_gold(&mut self, gold: u32) {
        self.gold = gold.min(self.max_inventory_gold());
    }

    pub fn set_stashed_gold(&mut self, stashed_gold: u32) {
        self.stashed_gold = stashed_gold.min(self.max_stash_gold());
    }

    pub fn base_life(&self) -> u32 {
        let base = BaseStats::for_class(self.class);
        let growth = ClassGrowth::for_class(self.class);
        let level_bonus = growth.life_for_level(self.level);
        let vit_bonus = growth.life_for_vitality(self.vitality, base.vit);

        // Quest index 20 is Act 3 Quest 1 (The Golden Bird).
        let golden_bird_bonus = (0..3)
            .filter(|&diff| libd2::core::quest::quest_is_completed(self.quests[diff][20]))
            .count() as u32
            * 20;

        base.hp + level_bonus + vit_bonus + golden_bird_bonus
    }

    pub fn base_mana(&self) -> u32 {
        let base = BaseStats::for_class(self.class);
        let growth = ClassGrowth::for_class(self.class);
        let level_bonus = growth.mana_for_level(self.level);
        let energy_bonus = growth.mana_for_energy(self.energy, base.eng);

        base.mana + level_bonus + energy_bonus
    }

    pub fn base_stamina(&self) -> u32 {
        let base = BaseStats::for_class(self.class);
        let growth = ClassGrowth::for_class(self.class);
        let level_bonus = growth.stamina_for_level(self.level);
        let vit_bonus = growth.stamina_for_vitality(self.vitality, base.vit);

        base.stamina + level_bonus + vit_bonus
    }

    pub fn recalculate_vitals(&mut self) {
        let hp_diff = self.max_hp.saturating_sub(self.current_hp);
        let mana_diff = self.max_mana.saturating_sub(self.current_mana);
        let stamina_diff = self.max_stamina.saturating_sub(self.current_stamina);

        self.max_hp = self.base_life();
        self.max_mana = self.base_mana();
        self.max_stamina = self.base_stamina();

        self.current_hp = self.max_hp.saturating_sub(hp_diff);
        self.current_mana = self.max_mana.saturating_sub(mana_diff);
        self.current_stamina = self.max_stamina.saturating_sub(stamina_diff);
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
            self.recalculate_vitals();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libd2::core::quest::{
        ACT_IV_COMPLETE, ACT_V_COMPLETE, ACT_V_INTRO, DIFFICULTY_COMPLETED_WORD,
        EVE_OF_DESTRUCTION, PRISON_OF_ICE, PROGRESSION_HELL_COMPLETED, PROGRESSION_NORMAL_UNLOCKED,
        QUEST_LOG_CLOSED, QUEST_PRISON_OF_ICE_SCROLL_CONSUMED, QUEST_REWARD_GRANTED,
        QUEST_REWARD_PENDING, TERRORS_END,
    };

    #[test]
    fn level_99_template_uses_exact_experience_breakpoint() {
        let save = Savegame::generate_template(CharacterClass::Amazon);

        assert_eq!(save.level, 99);
        assert_eq!(save.experience, experience_for_level(99));
    }

    #[test]
    fn paladin_holy_shield_adds_recursive_prerequisites() {
        let mut save = Savegame::generate_blank_template(CharacterClass::Paladin);
        save.set_level(99);

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
        let mut save = Savegame::generate_blank_template(CharacterClass::Paladin);
        save.set_level(99);
        save.skill_points_remaining = 4;

        assert!(!save.can_increase_skill(21));
        save.increase_skill(21);

        assert_eq!(save.skills.iter().copied().sum::<u8>(), 0);
        assert_eq!(save.skill_points_remaining, 4);
    }

    #[test]
    fn last_prerequisite_point_cannot_be_removed_while_dependent_is_allocated() {
        let mut save = Savegame::generate_blank_template(CharacterClass::Paladin);
        save.set_level(99);
        save.increase_skill(21);

        assert!(!save.can_decrease_skill(1));
        save.decrease_skill(1);

        assert_eq!(save.skills[1], 1);
        assert_eq!(save.skill_points_remaining, 93);
    }

    #[test]
    fn extra_prerequisite_points_can_be_removed_back_to_one() {
        let mut save = Savegame::generate_blank_template(CharacterClass::Paladin);
        save.set_level(99);
        save.increase_skill(21);
        save.increase_skill(1);

        assert!(save.can_decrease_skill(1));
        save.decrease_skill(1);

        assert_eq!(save.skills[1], 1);
        assert_eq!(save.skill_points_remaining, 93);
    }

    #[test]
    fn level_min_and_max_recompute_remaining_points() {
        let mut save = Savegame::generate_blank_template(CharacterClass::Amazon);
        save.game_version = GameVersion::Legacy;
        save.set_level(99);
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
        let mut save = Savegame::generate_blank_template(CharacterClass::Sorceress);
        save.set_level(99);
        let base_mana = save.base_mana();
        save.stat_points_remaining = 20;

        save.maximize_stat(CharacterStat::Energy);

        assert_eq!(save.current_mana, base_mana + 40);
        assert_eq!(save.max_mana, base_mana + 40);
        assert_eq!(save.stat_points_remaining, 0);

        save.minimize_stat(CharacterStat::Energy);

        assert_eq!(save.current_mana, base_mana);
        assert_eq!(save.max_mana, base_mana);
        assert_eq!(save.stat_points_remaining, 20);
    }

    #[test]
    fn point_normalization_preserves_loaded_allocations() {
        let mut save = Savegame::generate_blank_template(CharacterClass::Paladin);
        save.set_level(99);
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
        let mut save = Savegame::generate_blank_template(CharacterClass::Amazon);
        save.game_version = GameVersion::Legacy;
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
        let mut save = Savegame::generate_blank_template(CharacterClass::Amazon);
        save.game_version = GameVersion::Legacy;

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
    fn completing_reward_quest_grants_reward_without_leaving_it_pending() {
        let mut save = Savegame::generate_blank_template(CharacterClass::Amazon);
        save.game_version = GameVersion::Legacy;
        save.set_level(99);

        save.toggle_quest(0, 25);

        assert!(quest_is_completed(save.quests[0][25]));
        assert_eq!(save.quests[0][25] & QUEST_REWARD_PENDING, 0);
        assert_eq!(save.quests[0][25] & QUEST_LOG_CLOSED, QUEST_LOG_CLOSED);
        assert_eq!(save.skill_points_remaining, 100);
    }

    #[test]
    fn toggle_all_quests_sets_hidden_act_progression_words() {
        let mut save = Savegame::generate_blank_template(CharacterClass::Amazon);
        save.game_version = GameVersion::Legacy;

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
        let mut save = Savegame::generate_blank_template(CharacterClass::Amazon);
        save.game_version = GameVersion::Legacy;

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
        let mut save = Savegame::generate_blank_template(CharacterClass::Amazon);
        save.game_version = GameVersion::Legacy;
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
            bytes[0x15], // D2R_V105_PROGRESSION_OFFSET
            PROGRESSION_NORMAL_UNLOCKED
        );
    }

    #[test]
    fn to_bytes_sets_progression_for_completed_difficulties() {
        let mut save = Savegame::generate_blank_template(CharacterClass::Amazon);
        save.game_version = GameVersion::Legacy;

        save.toggle_all_quests(None, true);

        let bytes = save.to_bytes().expect("template should serialize");
        let quests = quest_words(&bytes);

        assert_eq!(bytes[0x15], PROGRESSION_HELL_COMPLETED);
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
        let mut save = Savegame::generate_blank_template(CharacterClass::Necromancer);
        save.set_level(99);

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
        let mut save = Savegame::generate_blank_template(CharacterClass::Necromancer);
        save.set_level(99);
        save.gold = 9_999_990;
        save.stashed_gold = 9_999_999;

        let bytes = save.to_bytes().expect("template should serialize");

        assert_eq!(stat_value(&bytes, CharacterStat::Gold), Some(990_000));
        assert_eq!(
            stat_value(&bytes, CharacterStat::StashedGold),
            Some(2_500_000)
        );
    }

    #[test]
    fn rotw_template_serializes_native_v105_signatures() {
        let save = Savegame::generate_template(CharacterClass::Amazon);

        let bytes = save.to_bytes().expect("template should serialize");
        let parsed = CharacterFile::parse(bytes.clone()).expect("serialized template parses");

        assert_eq!(parsed.header().expansion_mode, ExpansionMode::RotW);
        assert_eq!(bytes[0x14], 0x00);
        assert_eq!(bytes[0x19..0x1b], [0x10, 0x1e]);
        assert_eq!(bytes[0x24..0x28], [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(bytes[0x00f8], ExpansionMode::V105_ROTW_MARKER);
        assert_eq!(
            bytes[0x02bd..0x02c5],
            [0x57, 0x53, 0x01, 0x00, 0x00, 0x00, 0x50, 0x00]
        );
        assert_eq!(bytes[0x030d..0x0311], [0x01, 0x77, 0x34, 0x00]);
        assert!(bytes.ends_with(&[
            0x4a, 0x4d, 0x00, 0x00, 0x6a, 0x66, 0x6b, 0x66, 0x00, 0x01, 0x00, 0x6c, 0x66, 0x00,
            0x00,
        ]));
    }

    #[test]
    fn sorceress_template_uses_fresh_rotw_strength() {
        let save = Savegame::generate_blank_template(CharacterClass::Sorceress);

        assert_eq!(save.strength, 10);

        let bytes = save.to_bytes().expect("template should serialize");
        assert_eq!(stat_value(&bytes, CharacterStat::Strength), Some(10));
    }

    fn quest_words(bytes: &[u8]) -> [[u16; SAVE_QUEST_WORDS_PER_DIFFICULTY]; 3] {
        quest::parse_legacy_quest_words(bytes, 0).expect("quest header should exist")
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
