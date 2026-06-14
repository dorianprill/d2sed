use crate::save::{BitWriter, fix_header};
use anyhow::{Context, Result};
use libd2::core::character_class::CharacterClass;
use libd2::core::character_file::{CharacterFile, CharacterStat};
use std::path::Path;

const QUEST_REWARD_GRANTED: u16 = 0x0001;
const QUEST_REWARD_PENDING: u16 = 0x0002;
const QUEST_PRISON_OF_ICE_SCROLL_CONSUMED: u16 = 0x0080;
const QUEST_LOG_CLOSED: u16 = 0x1000;
const QUEST_COMPLETION_MASK: u16 = QUEST_REWARD_GRANTED | QUEST_REWARD_PENDING | QUEST_LOG_CLOSED;
const QUEST_CLOSED_COMPLETE: u16 = QUEST_REWARD_GRANTED | QUEST_LOG_CLOSED;
const DIFFICULTY_COMPLETED_WORD: u16 = 0x8001;
const LEGACY_PROGRESSION_OFFSET: usize = 0x25;
const PROGRESSION_NORMAL_UNLOCKED: u8 = 0x05;
const PROGRESSION_NIGHTMARE_UNLOCKED: u8 = 0x0a;
const PROGRESSION_HELL_COMPLETED: u8 = 0x0f;
const GOLD_MAX_ENCODED: u32 = (1 << 25) - 1;

const ACT_I_COMPLETE: usize = 7;
const ACT_II_INTRO: usize = 8;
const ACT_II_COMPLETE: usize = 15;
const ACT_III_INTRO: usize = 16;
const ACT_III_COMPLETE: usize = 23;
const ACT_IV_INTRO: usize = 24;
const ACT_IV_COMPLETE: usize = 28;
const ACT_V_INTRO: usize = 32;
const ACT_V_COMPLETE: usize = 41;

const SISTERS_TO_THE_SLAUGHTER: usize = 6;
const THE_SEVEN_TOMBS: usize = 14;
const THE_GUARDIAN: usize = 22;
const TERRORS_END: usize = 26;
const PRISON_OF_ICE: usize = 37;
const EVE_OF_DESTRUCTION: usize = 40;

const VISIBLE_QUEST_INDICES: [usize; 27] = [
    1, 2, 4, 5, 3, 6, 9, 10, 11, 12, 13, 14, 20, 19, 18, 17, 21, 22, 25, 26, 27, 35, 36, 37, 38,
    39, 40,
];

const EXPERIENCE_BY_LEVEL: [u32; 99] = [
    0,
    500,
    1_500,
    3_750,
    7_875,
    14_175,
    22_680,
    32_886,
    44_396,
    57_715,
    72_144,
    90_180,
    112_725,
    140_906,
    176_132,
    220_165,
    275_207,
    344_008,
    430_010,
    537_513,
    671_891,
    839_864,
    1_049_830,
    1_312_287,
    1_640_359,
    2_050_449,
    2_563_061,
    3_203_826,
    3_902_260,
    4_663_553,
    5_493_363,
    6_397_855,
    7_383_752,
    8_458_379,
    9_629_723,
    10_906_488,
    12_298_162,
    13_815_086,
    15_468_534,
    17_270_791,
    19_235_252,
    21_376_515,
    23_710_491,
    26_254_525,
    29_027_522,
    32_050_088,
    35_344_686,
    38_935_798,
    42_850_109,
    47_116_709,
    51_767_302,
    56_836_449,
    62_361_819,
    68_384_473,
    74_949_165,
    82_104_680,
    89_904_191,
    98_405_658,
    107_672_256,
    117_772_849,
    128_782_495,
    140_783_010,
    153_863_570,
    168_121_381,
    183_662_396,
    200_602_101,
    219_066_380,
    239_192_444,
    261_129_853,
    285_041_630,
    311_105_466,
    339_515_048,
    370_481_492,
    404_234_916,
    441_026_148,
    481_128_591,
    524_840_254,
    572_485_967,
    624_419_793,
    681_027_665,
    742_730_244,
    809_986_056,
    883_294_891,
    963_201_521,
    1_050_299_747,
    1_145_236_814,
    1_248_718_217,
    1_361_512_946,
    1_484_459_201,
    1_618_470_619,
    1_764_543_065,
    1_923_762_030,
    2_097_310_703,
    2_286_478_756,
    2_492_671_933,
    2_717_422_497,
    2_962_400_612,
    3_229_426_756,
    3_520_485_254,
];

/// A display group for one class skill tab/tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillCategory {
    pub name: &'static str,
    pub slots: &'static [usize],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkillRequirement {
    level: u32,
    prereqs: &'static [usize],
}

const AMAZON_JAVELIN_AND_SPEAR: [usize; 10] = [4, 8, 9, 13, 14, 18, 19, 24, 28, 29];
const AMAZON_PASSIVE_AND_MAGIC: [usize; 10] = [2, 3, 7, 11, 12, 17, 22, 23, 26, 27];
const AMAZON_BOW_AND_CROSSBOW: [usize; 10] = [0, 1, 5, 6, 10, 15, 16, 20, 21, 25];
const AMAZON_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Bow and Crossbow",
        slots: &AMAZON_BOW_AND_CROSSBOW,
    },
    SkillCategory {
        name: "Passive and Magic",
        slots: &AMAZON_PASSIVE_AND_MAGIC,
    },
    SkillCategory {
        name: "Javelin and Spear",
        slots: &AMAZON_JAVELIN_AND_SPEAR,
    },
];

const SORCERESS_FIRE: [usize; 10] = [0, 1, 5, 10, 11, 15, 16, 20, 25, 26];
const SORCERESS_LIGHTNING: [usize; 10] = [2, 6, 7, 12, 13, 17, 18, 21, 22, 27];
const SORCERESS_COLD: [usize; 10] = [3, 4, 8, 9, 14, 19, 23, 24, 28, 29];
const SORCERESS_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Fire Spells",
        slots: &SORCERESS_FIRE,
    },
    SkillCategory {
        name: "Lightning Spells",
        slots: &SORCERESS_LIGHTNING,
    },
    SkillCategory {
        name: "Cold Spells",
        slots: &SORCERESS_COLD,
    },
];

const NECROMANCER_SUMMONING: [usize; 10] = [3, 4, 9, 13, 14, 19, 23, 24, 28, 29];
const NECROMANCER_POISON_AND_BONE: [usize; 10] = [1, 2, 7, 8, 12, 17, 18, 22, 26, 27];
const NECROMANCER_CURSES: [usize; 10] = [0, 5, 6, 10, 11, 15, 16, 20, 21, 25];
const NECROMANCER_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Summoning Spells",
        slots: &NECROMANCER_SUMMONING,
    },
    SkillCategory {
        name: "Poison and Bone",
        slots: &NECROMANCER_POISON_AND_BONE,
    },
    SkillCategory {
        name: "Curses",
        slots: &NECROMANCER_CURSES,
    },
];

const PALADIN_COMBAT: [usize; 10] = [0, 1, 5, 10, 11, 15, 16, 20, 21, 25];
const PALADIN_OFFENSIVE_AURAS: [usize; 10] = [2, 6, 7, 12, 17, 18, 22, 23, 26, 27];
const PALADIN_DEFENSIVE_AURAS: [usize; 10] = [3, 4, 8, 9, 13, 14, 19, 24, 28, 29];
const PALADIN_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Defensive Auras",
        slots: &PALADIN_DEFENSIVE_AURAS,
    },
    SkillCategory {
        name: "Offensive Auras",
        slots: &PALADIN_OFFENSIVE_AURAS,
    },
    SkillCategory {
        name: "Combat Skills",
        slots: &PALADIN_COMBAT,
    },
];

const BARBARIAN_COMBAT: [usize; 10] = [0, 6, 7, 13, 14, 17, 18, 21, 25, 26];
const BARBARIAN_MASTERIES: [usize; 10] = [1, 2, 3, 8, 9, 10, 15, 19, 22, 27];
const BARBARIAN_WARCRIES: [usize; 10] = [4, 5, 11, 12, 16, 20, 23, 24, 28, 29];
const BARBARIAN_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Warcries",
        slots: &BARBARIAN_WARCRIES,
    },
    SkillCategory {
        name: "Combat Masteries",
        slots: &BARBARIAN_MASTERIES,
    },
    SkillCategory {
        name: "Combat Skills",
        slots: &BARBARIAN_COMBAT,
    },
];

const DRUID_ELEMENTAL: [usize; 10] = [4, 8, 9, 13, 14, 19, 23, 24, 28, 29];
const DRUID_SHAPE_SHIFTING: [usize; 10] = [2, 3, 7, 11, 12, 17, 18, 21, 22, 27];
const DRUID_SUMMONING: [usize; 10] = [0, 1, 5, 6, 10, 15, 16, 20, 25, 26];
const DRUID_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Elemental",
        slots: &DRUID_ELEMENTAL,
    },
    SkillCategory {
        name: "Shape Shifting",
        slots: &DRUID_SHAPE_SHIFTING,
    },
    SkillCategory {
        name: "Summoning",
        slots: &DRUID_SUMMONING,
    },
];

const ASSASSIN_MARTIAL_ARTS: [usize; 10] = [3, 4, 8, 9, 14, 18, 19, 23, 24, 29];
const ASSASSIN_SHADOW_DISCIPLINES: [usize; 10] = [1, 2, 7, 12, 13, 16, 17, 22, 27, 28];
const ASSASSIN_TRAPS: [usize; 10] = [0, 5, 6, 10, 11, 15, 20, 21, 25, 26];
const ASSASSIN_SKILL_CATEGORIES: [SkillCategory; 3] = [
    SkillCategory {
        name: "Martial Arts",
        slots: &ASSASSIN_MARTIAL_ARTS,
    },
    SkillCategory {
        name: "Shadow Disciplines",
        slots: &ASSASSIN_SHADOW_DISCIPLINES,
    },
    SkillCategory {
        name: "Traps",
        slots: &ASSASSIN_TRAPS,
    },
];

const WARLOCK_SKILLS: [usize; 30] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29,
];
const WARLOCK_SKILL_CATEGORIES: [SkillCategory; 1] = [SkillCategory {
    name: "Warlock Skills",
    slots: &WARLOCK_SKILLS,
}];

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

    // Quests for Normal, Nightmare, Hell (48 words each as per d2s layout)
    pub quests: [[u16; 48]; 3],

    pub hardcore: bool,
    pub died: bool,

    // Waypoints for Normal, Nightmare, Hell (39 waypoints total, stored as bits)
    pub waypoints: [[bool; 39]; 3],

    pub game_version: GameVersion,
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
    pub const VISIBLE_QUEST_INDICES: &'static [usize] = &VISIBLE_QUEST_INDICES;

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
        let mut quests = [[0u16; 48]; 3];
        if let Some(woo_idx) = raw_bytes.windows(4).position(|window| window == b"Woo!") {
            let mut offset = woo_idx + 10;
            for difficulty in &mut quests {
                if offset + 96 <= raw_bytes.len() {
                    for (i, word) in difficulty.iter_mut().enumerate() {
                        let b1 = raw_bytes[offset + i * 2] as u16;
                        let b2 = raw_bytes[offset + i * 2 + 1] as u16;
                        *word = b1 | (b2 << 8);
                    }
                    offset += 96;
                }
            }
        }

        let mut waypoints = [[false; 39]; 3];
        if let Some(ws_idx) = raw_bytes.windows(2).position(|window| window == b"WS") {
            let mut offset = ws_idx + 8; // Skip WS, unknown, and length
            for difficulty in &mut waypoints {
                if offset + 24 <= raw_bytes.len() {
                    let data_offset = offset + 2;
                    for (i, waypoint) in difficulty.iter_mut().enumerate() {
                        let byte_idx = i / 8;
                        let bit_idx = i % 8;
                        *waypoint = (raw_bytes[data_offset + byte_idx] & (1 << bit_idx)) != 0;
                    }
                    offset += 24;
                }
            }
        }

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
        raw[0x14f..0x153].copy_from_slice(b"Woo!");
        raw[0x153..0x159].copy_from_slice(&[0x06, 0x00, 0x00, 0x00, 0x2a, 0x01]);

        // Waypoints
        raw[0x279..0x279 + 2].copy_from_slice(b"WS");
        raw[0x279 + 2..0x279 + 8].copy_from_slice(&[0x06, 0x00, 0x00, 0x00, 0x50, 0x00]);
        for diff in 0..3 {
            let offset = 0x281 + diff * 24;
            raw[offset] = 0x02;
            raw[offset + 1] = 0x01;
        }
        raw[0x2c9] = 0x01; // Trailer

        // NPC
        raw[0x2ca..0x2ca + 2].copy_from_slice(b"w4");
        raw[0x2ca + 2..0x2ca + 4].copy_from_slice(&[0x34, 0x00]); // 52 bytes len

        // Initialize quests with intro/travel markers
        let mut quests = [[0u16; 48]; 3];
        for difficulty in &mut quests {
            // Standard act-introduction markers for a template character.
            for &idx in &[0, ACT_II_INTRO, ACT_III_INTRO, ACT_IV_INTRO, ACT_V_INTRO] {
                difficulty[idx] = QUEST_REWARD_GRANTED;
            }
        }

        Self {
            name,
            class,
            level: 99,
            experience: 3_520_485_254,
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
            waypoints: [[false; 39]; 3],
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

        // Apply Quests overrides
        if let Some(woo_idx) = new_raw.windows(4).position(|window| window == b"Woo!") {
            // Ensure Woo! header magic is correct
            new_raw[woo_idx + 4..woo_idx + 10].copy_from_slice(&[6, 0, 0, 0, 0x2a, 0x01]);
            let mut offset = woo_idx + 10;
            for difficulty in &quests {
                if offset + 96 <= new_raw.len() {
                    for (i, &word) in difficulty.iter().enumerate() {
                        new_raw[offset + i * 2] = (word & 0xFF) as u8;
                        new_raw[offset + i * 2 + 1] = (word >> 8) as u8;
                    }
                    offset += 96;
                }
            }
        }
        apply_progression_from_quests(&mut new_raw, &quests);

        // Apply Waypoints overrides
        if let Some(ws_idx) = new_raw.windows(2).position(|window| window == b"WS") {
            new_raw[ws_idx + 2..ws_idx + 8].copy_from_slice(&[6, 0, 0, 0, 0x50, 0x00]);
            let mut offset = ws_idx + 8;
            for difficulty in &self.waypoints {
                if offset + 24 <= new_raw.len() {
                    new_raw[offset] = 0x02;
                    new_raw[offset + 1] = 0x01;
                    let data_offset = offset + 2;
                    // Clear existing bits first
                    new_raw[data_offset..data_offset + 5].fill(0);
                    for (i, waypoint) in difficulty.iter().enumerate() {
                        let byte_idx = i / 8;
                        let bit_idx = i % 8;
                        if *waypoint {
                            new_raw[data_offset + byte_idx] |= 1 << bit_idx;
                        }
                    }
                    offset += 24;
                }
            }
            new_raw[ws_idx + 80] = 0x01; // Trailer
        }

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
        let level_points = self.level.saturating_sub(1) * 5;
        let mut quest_points = 0;
        for difficulty in &self.quests {
            if (difficulty[17] & 1) == 1 {
                quest_points += 5;
            }
        }
        level_points + quest_points
    }

    pub fn total_allowed_skill_points(&self) -> u32 {
        let mut quest_points = 0;
        for difficulty in &self.quests {
            if quest_is_completed(difficulty[1]) {
                quest_points += 1;
            }
            if quest_is_completed(difficulty[9]) {
                quest_points += 1;
            }
            if quest_is_completed(difficulty[25]) {
                quest_points += 2;
            }
        }
        self.level.saturating_sub(1) + quest_points
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
        self.experience = Self::calculate_experience_for_level(self.level);
        if self.level != old_level {
            self.normalize_point_totals();
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
                self.current_hp += actual_amount * 2;
                self.max_hp += actual_amount * 2;
                self.current_stamina += actual_amount;
                self.max_stamina += actual_amount;
            }
            CharacterStat::Energy => {
                self.energy += actual_amount;
                self.current_mana += actual_amount * 2;
                self.max_mana += actual_amount * 2;
            }
            _ => return,
        }
        self.stat_points_remaining -= actual_amount;
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
                self.current_hp = self.current_hp.saturating_sub(diff * 2).max(base.hp);
                self.max_hp = self.max_hp.saturating_sub(diff * 2).max(base.hp);
                self.current_stamina = self.current_stamina.saturating_sub(diff).max(base.stamina);
                self.max_stamina = self.max_stamina.saturating_sub(diff).max(base.stamina);
                self.stat_points_remaining += diff;
            }
            CharacterStat::Energy => {
                let diff = self.energy.saturating_sub(base.eng).min(amount);
                self.energy -= diff;
                self.current_mana = self.current_mana.saturating_sub(diff * 2).max(base.mana);
                self.max_mana = self.max_mana.saturating_sub(diff * 2).max(base.mana);
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

    pub fn consumed_resistance_scrolls(&self) -> [bool; 3] {
        [
            self.quests[0][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED != 0,
            self.quests[1][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED != 0,
            self.quests[2][PRISON_OF_ICE] & QUEST_PRISON_OF_ICE_SCROLL_CONSUMED != 0,
        ]
    }

    pub fn base_resistance_bonus(&self) -> u32 {
        self.consumed_resistance_scrolls()
            .iter()
            .filter(|&&consumed| consumed)
            .count() as u32
            * 10
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
        (self.level.clamp(1, 99) * 10_000).min(GOLD_MAX_ENCODED)
    }

    pub fn max_stash_gold(&self) -> u32 {
        let level = self.level.clamp(1, 99);
        let multiplier = if level <= 30 {
            level / 10 + 1
        } else {
            level / 2 + 1
        };
        (multiplier * 50_000).min(GOLD_MAX_ENCODED)
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
                for &idx in Self::VISIBLE_QUEST_INDICES {
                    let is_completed = quest_is_completed(self.quests[diff][idx]);
                    if is_completed != state {
                        self.toggle_quest(diff, idx);
                    }
                }
            }
            None => {
                for diff in 0..3 {
                    for &idx in Self::VISIBLE_QUEST_INDICES {
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

    pub fn get_quest_name(idx: usize) -> &'static str {
        match idx {
            1 => "Den of Evil",
            2 => "Sisters' Burial Grounds",
            3 => "Tools of the Trade",
            4 => "The Search for Cain",
            5 => "The Forgotten Tower",
            6 => "Sisters to the Slaughter",
            9 => "Radament's Lair",
            10 => "The Horadric Staff",
            11 => "Tainted Sun",
            12 => "Arcane Sanctuary",
            13 => "The Summoner",
            14 => "The Seven Tombs",
            17 => "Lam Esen's Tome",
            18 => "Khalim's Will",
            19 => "Blade of the Old Religion",
            20 => "The Golden Bird",
            21 => "The Blackened Temple",
            22 => "The Guardian",
            25 => "The Fallen Angel",
            26 => "Terror's End",
            27 => "Hellforge",
            35 => "Siege on Harrogath",
            36 => "Rescue on Mount Arreat",
            37 => "Prison of Ice",
            38 => "Betrayal of Harrogath",
            39 => "Rite of Passage",
            40 => "Eve of Destruction",
            _ => "Unknown Quest",
        }
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
                4 => "Howl",
                5 => "Find Potion",
                6 => "Leap",
                7 => "Double Swing",
                8 => "Pole Arm Mastery",
                9 => "Throwing Mastery",
                10 => "Spear Mastery",
                11 => "Taunt",
                12 => "Shout",
                13 => "Stun",
                14 => "Double Throw",
                15 => "Increased Stamina",
                16 => "Find Item",
                17 => "Leap Attack",
                18 => "Concentrate",
                19 => "Iron Skin",
                20 => "Battle Cry",
                21 => "Frenzy",
                22 => "Increased Speed",
                23 => "Battle Orders",
                24 => "Grim Ward",
                25 => "Whirlwind",
                26 => "Berserk",
                27 => "Natural Resistance",
                28 => "War Cry",
                29 => "Battle Command",
                _ => "Unknown",
            },
            CharacterClass::Druid => match slot {
                0 => "Raven",
                1 => "Poison Creeper",
                2 => "Werewolf",
                3 => "Lycanthropy",
                4 => "Firestorm",
                5 => "Oak Sage",
                6 => "Summon Spirit Wolf",
                7 => "Werebear",
                8 => "Molten Boulder",
                9 => "Arctic Blast",
                10 => "Carrion Vine",
                11 => "Feral Rage",
                12 => "Maul",
                13 => "Fissure",
                14 => "Cyclone Armor",
                15 => "Heart of Wolverine",
                16 => "Summon Dire Wolf",
                17 => "Rabies",
                18 => "Fire Claws",
                19 => "Twister",
                20 => "Solar Creeper",
                21 => "Hunger",
                22 => "Shock Wave",
                23 => "Volcano",
                24 => "Tornado",
                25 => "Spirit of Barbs",
                26 => "Summon Grizzly",
                27 => "Fury",
                28 => "Armageddon",
                29 => "Hurricane",
                _ => "Unknown",
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
            CharacterClass::Warlock => "Warlock Skill",
        }
    }

    pub fn skill_categories(class: CharacterClass) -> &'static [SkillCategory] {
        match class {
            CharacterClass::Amazon => &AMAZON_SKILL_CATEGORIES,
            CharacterClass::Sorceress => &SORCERESS_SKILL_CATEGORIES,
            CharacterClass::Necromancer => &NECROMANCER_SKILL_CATEGORIES,
            CharacterClass::Paladin => &PALADIN_SKILL_CATEGORIES,
            CharacterClass::Barbarian => &BARBARIAN_SKILL_CATEGORIES,
            CharacterClass::Druid => &DRUID_SKILL_CATEGORIES,
            CharacterClass::Assassin => &ASSASSIN_SKILL_CATEGORIES,
            CharacterClass::Warlock => &WARLOCK_SKILL_CATEGORIES,
        }
    }

    fn skill_requirement(class: CharacterClass, slot: usize) -> SkillRequirement {
        match class {
            CharacterClass::Amazon => match slot {
                0 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                1 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                2 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                3 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                4 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                5 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                6 => SkillRequirement {
                    level: 6,
                    prereqs: &[0],
                },
                7 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                8 => SkillRequirement {
                    level: 6,
                    prereqs: &[4],
                },
                9 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                10 => SkillRequirement {
                    level: 12,
                    prereqs: &[1, 6],
                },
                11 => SkillRequirement {
                    level: 12,
                    prereqs: &[2],
                },
                12 => SkillRequirement {
                    level: 12,
                    prereqs: &[7],
                },
                13 => SkillRequirement {
                    level: 12,
                    prereqs: &[4],
                },
                14 => SkillRequirement {
                    level: 12,
                    prereqs: &[9],
                },
                15 => SkillRequirement {
                    level: 18,
                    prereqs: &[5],
                },
                16 => SkillRequirement {
                    level: 18,
                    prereqs: &[5, 6],
                },
                17 => SkillRequirement {
                    level: 18,
                    prereqs: &[3],
                },
                18 => SkillRequirement {
                    level: 18,
                    prereqs: &[8, 14],
                },
                19 => SkillRequirement {
                    level: 18,
                    prereqs: &[14],
                },
                20 => SkillRequirement {
                    level: 24,
                    prereqs: &[16],
                },
                21 => SkillRequirement {
                    level: 24,
                    prereqs: &[10],
                },
                22 => SkillRequirement {
                    level: 24,
                    prereqs: &[11],
                },
                23 => SkillRequirement {
                    level: 24,
                    prereqs: &[12],
                },
                24 => SkillRequirement {
                    level: 24,
                    prereqs: &[13],
                },
                25 => SkillRequirement {
                    level: 30,
                    prereqs: &[15],
                },
                26 => SkillRequirement {
                    level: 30,
                    prereqs: &[22, 23],
                },
                27 => SkillRequirement {
                    level: 30,
                    prereqs: &[17],
                },
                28 => SkillRequirement {
                    level: 30,
                    prereqs: &[18],
                },
                29 => SkillRequirement {
                    level: 30,
                    prereqs: &[19],
                },
                _ => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
            },
            CharacterClass::Sorceress => match slot {
                0 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                1 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                2 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                3 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                4 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                5 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                6 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                7 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                8 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                9 => SkillRequirement {
                    level: 6,
                    prereqs: &[3],
                },
                10 => SkillRequirement {
                    level: 12,
                    prereqs: &[5],
                },
                11 => SkillRequirement {
                    level: 12,
                    prereqs: &[0],
                },
                12 => SkillRequirement {
                    level: 12,
                    prereqs: &[6],
                },
                13 => SkillRequirement {
                    level: 12,
                    prereqs: &[2],
                },
                14 => SkillRequirement {
                    level: 12,
                    prereqs: &[9, 4],
                },
                15 => SkillRequirement {
                    level: 18,
                    prereqs: &[10],
                },
                16 => SkillRequirement {
                    level: 18,
                    prereqs: &[1, 11],
                },
                17 => SkillRequirement {
                    level: 18,
                    prereqs: &[13],
                },
                18 => SkillRequirement {
                    level: 18,
                    prereqs: &[7],
                },
                19 => SkillRequirement {
                    level: 18,
                    prereqs: &[9],
                },
                20 => SkillRequirement {
                    level: 24,
                    prereqs: &[11, 15],
                },
                21 => SkillRequirement {
                    level: 24,
                    prereqs: &[12, 17],
                },
                22 => SkillRequirement {
                    level: 24,
                    prereqs: &[18, 17],
                },
                23 => SkillRequirement {
                    level: 24,
                    prereqs: &[8, 19],
                },
                24 => SkillRequirement {
                    level: 24,
                    prereqs: &[14],
                },
                25 => SkillRequirement {
                    level: 30,
                    prereqs: &[],
                },
                26 => SkillRequirement {
                    level: 30,
                    prereqs: &[16],
                },
                27 => SkillRequirement {
                    level: 30,
                    prereqs: &[],
                },
                28 => SkillRequirement {
                    level: 30,
                    prereqs: &[23],
                },
                29 => SkillRequirement {
                    level: 30,
                    prereqs: &[],
                },
                _ => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
            },
            CharacterClass::Necromancer => match slot {
                0 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                1 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                2 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                3 => SkillRequirement {
                    level: 1,
                    prereqs: &[4],
                },
                4 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                5 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                6 => SkillRequirement {
                    level: 6,
                    prereqs: &[0],
                },
                7 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                8 => SkillRequirement {
                    level: 6,
                    prereqs: &[1],
                },
                9 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                10 => SkillRequirement {
                    level: 12,
                    prereqs: &[0],
                },
                11 => SkillRequirement {
                    level: 12,
                    prereqs: &[6],
                },
                12 => SkillRequirement {
                    level: 12,
                    prereqs: &[2],
                },
                13 => SkillRequirement {
                    level: 12,
                    prereqs: &[9],
                },
                14 => SkillRequirement {
                    level: 12,
                    prereqs: &[4],
                },
                15 => SkillRequirement {
                    level: 18,
                    prereqs: &[5],
                },
                16 => SkillRequirement {
                    level: 18,
                    prereqs: &[10],
                },
                17 => SkillRequirement {
                    level: 18,
                    prereqs: &[7, 8],
                },
                18 => SkillRequirement {
                    level: 18,
                    prereqs: &[8],
                },
                19 => SkillRequirement {
                    level: 18,
                    prereqs: &[9],
                },
                20 => SkillRequirement {
                    level: 24,
                    prereqs: &[15],
                },
                21 => SkillRequirement {
                    level: 24,
                    prereqs: &[11],
                },
                22 => SkillRequirement {
                    level: 24,
                    prereqs: &[12, 18],
                },
                23 => SkillRequirement {
                    level: 24,
                    prereqs: &[13],
                },
                24 => SkillRequirement {
                    level: 24,
                    prereqs: &[19],
                },
                25 => SkillRequirement {
                    level: 30,
                    prereqs: &[16, 21],
                },
                26 => SkillRequirement {
                    level: 30,
                    prereqs: &[17],
                },
                27 => SkillRequirement {
                    level: 30,
                    prereqs: &[18],
                },
                28 => SkillRequirement {
                    level: 30,
                    prereqs: &[24],
                },
                29 => SkillRequirement {
                    level: 30,
                    prereqs: &[14, 24],
                },
                _ => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
            },
            CharacterClass::Paladin => match slot {
                0 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                1 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                2 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                3 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                4 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                5 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                6 => SkillRequirement {
                    level: 6,
                    prereqs: &[2],
                },
                7 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                8 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                9 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                10 => SkillRequirement {
                    level: 12,
                    prereqs: &[0],
                },
                11 => SkillRequirement {
                    level: 12,
                    prereqs: &[1],
                },
                12 => SkillRequirement {
                    level: 12,
                    prereqs: &[2],
                },
                13 => SkillRequirement {
                    level: 12,
                    prereqs: &[3],
                },
                14 => SkillRequirement {
                    level: 12,
                    prereqs: &[],
                },
                15 => SkillRequirement {
                    level: 18,
                    prereqs: &[10],
                },
                16 => SkillRequirement {
                    level: 18,
                    prereqs: &[5],
                },
                17 => SkillRequirement {
                    level: 18,
                    prereqs: &[12],
                },
                18 => SkillRequirement {
                    level: 18,
                    prereqs: &[6],
                },
                19 => SkillRequirement {
                    level: 18,
                    prereqs: &[13, 8],
                },
                20 => SkillRequirement {
                    level: 24,
                    prereqs: &[15],
                },
                21 => SkillRequirement {
                    level: 24,
                    prereqs: &[11, 16],
                },
                22 => SkillRequirement {
                    level: 24,
                    prereqs: &[18],
                },
                23 => SkillRequirement {
                    level: 24,
                    prereqs: &[7, 18],
                },
                24 => SkillRequirement {
                    level: 24,
                    prereqs: &[13],
                },
                25 => SkillRequirement {
                    level: 30,
                    prereqs: &[16, 20],
                },
                26 => SkillRequirement {
                    level: 30,
                    prereqs: &[17],
                },
                27 => SkillRequirement {
                    level: 30,
                    prereqs: &[23],
                },
                28 => SkillRequirement {
                    level: 30,
                    prereqs: &[19],
                },
                29 => SkillRequirement {
                    level: 30,
                    prereqs: &[],
                },
                _ => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
            },
            CharacterClass::Barbarian => match slot {
                0 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                1 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                2 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                3 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                4 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                5 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                6 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                7 => SkillRequirement {
                    level: 6,
                    prereqs: &[0],
                },
                8 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                9 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                10 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                11 => SkillRequirement {
                    level: 6,
                    prereqs: &[4],
                },
                12 => SkillRequirement {
                    level: 6,
                    prereqs: &[4],
                },
                13 => SkillRequirement {
                    level: 12,
                    prereqs: &[0],
                },
                14 => SkillRequirement {
                    level: 12,
                    prereqs: &[7],
                },
                15 => SkillRequirement {
                    level: 12,
                    prereqs: &[],
                },
                16 => SkillRequirement {
                    level: 12,
                    prereqs: &[5],
                },
                17 => SkillRequirement {
                    level: 18,
                    prereqs: &[6],
                },
                18 => SkillRequirement {
                    level: 18,
                    prereqs: &[13],
                },
                19 => SkillRequirement {
                    level: 18,
                    prereqs: &[],
                },
                20 => SkillRequirement {
                    level: 18,
                    prereqs: &[11],
                },
                21 => SkillRequirement {
                    level: 24,
                    prereqs: &[14],
                },
                22 => SkillRequirement {
                    level: 24,
                    prereqs: &[15],
                },
                23 => SkillRequirement {
                    level: 24,
                    prereqs: &[12],
                },
                24 => SkillRequirement {
                    level: 24,
                    prereqs: &[16],
                },
                25 => SkillRequirement {
                    level: 30,
                    prereqs: &[17, 18],
                },
                26 => SkillRequirement {
                    level: 30,
                    prereqs: &[18],
                },
                27 => SkillRequirement {
                    level: 30,
                    prereqs: &[19],
                },
                28 => SkillRequirement {
                    level: 30,
                    prereqs: &[20, 23],
                },
                29 => SkillRequirement {
                    level: 30,
                    prereqs: &[23],
                },
                _ => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
            },
            CharacterClass::Druid => match slot {
                0 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                1 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                2 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                3 => SkillRequirement {
                    level: 1,
                    prereqs: &[2],
                },
                4 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                5 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                6 => SkillRequirement {
                    level: 6,
                    prereqs: &[0],
                },
                7 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                8 => SkillRequirement {
                    level: 6,
                    prereqs: &[4],
                },
                9 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                10 => SkillRequirement {
                    level: 12,
                    prereqs: &[1],
                },
                11 => SkillRequirement {
                    level: 12,
                    prereqs: &[2],
                },
                12 => SkillRequirement {
                    level: 12,
                    prereqs: &[7],
                },
                13 => SkillRequirement {
                    level: 12,
                    prereqs: &[8],
                },
                14 => SkillRequirement {
                    level: 12,
                    prereqs: &[9],
                },
                15 => SkillRequirement {
                    level: 18,
                    prereqs: &[5],
                },
                16 => SkillRequirement {
                    level: 18,
                    prereqs: &[5, 6],
                },
                17 => SkillRequirement {
                    level: 18,
                    prereqs: &[11],
                },
                18 => SkillRequirement {
                    level: 18,
                    prereqs: &[11, 12],
                },
                19 => SkillRequirement {
                    level: 18,
                    prereqs: &[14],
                },
                20 => SkillRequirement {
                    level: 24,
                    prereqs: &[10],
                },
                21 => SkillRequirement {
                    level: 24,
                    prereqs: &[18],
                },
                22 => SkillRequirement {
                    level: 24,
                    prereqs: &[12],
                },
                23 => SkillRequirement {
                    level: 24,
                    prereqs: &[13],
                },
                24 => SkillRequirement {
                    level: 24,
                    prereqs: &[19],
                },
                25 => SkillRequirement {
                    level: 30,
                    prereqs: &[15],
                },
                26 => SkillRequirement {
                    level: 30,
                    prereqs: &[16],
                },
                27 => SkillRequirement {
                    level: 30,
                    prereqs: &[17],
                },
                28 => SkillRequirement {
                    level: 30,
                    prereqs: &[23, 29],
                },
                29 => SkillRequirement {
                    level: 30,
                    prereqs: &[24],
                },
                _ => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
            },
            CharacterClass::Assassin => match slot {
                0 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                1 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                2 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                3 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                4 => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
                5 => SkillRequirement {
                    level: 6,
                    prereqs: &[0],
                },
                6 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                7 => SkillRequirement {
                    level: 6,
                    prereqs: &[1],
                },
                8 => SkillRequirement {
                    level: 6,
                    prereqs: &[],
                },
                9 => SkillRequirement {
                    level: 6,
                    prereqs: &[4],
                },
                10 => SkillRequirement {
                    level: 12,
                    prereqs: &[5],
                },
                11 => SkillRequirement {
                    level: 12,
                    prereqs: &[0],
                },
                12 => SkillRequirement {
                    level: 12,
                    prereqs: &[1],
                },
                13 => SkillRequirement {
                    level: 12,
                    prereqs: &[2],
                },
                14 => SkillRequirement {
                    level: 12,
                    prereqs: &[3],
                },
                15 => SkillRequirement {
                    level: 18,
                    prereqs: &[6, 11],
                },
                16 => SkillRequirement {
                    level: 18,
                    prereqs: &[7],
                },
                17 => SkillRequirement {
                    level: 18,
                    prereqs: &[13, 12],
                },
                18 => SkillRequirement {
                    level: 18,
                    prereqs: &[8],
                },
                19 => SkillRequirement {
                    level: 18,
                    prereqs: &[9],
                },
                20 => SkillRequirement {
                    level: 24,
                    prereqs: &[10],
                },
                21 => SkillRequirement {
                    level: 24,
                    prereqs: &[11],
                },
                22 => SkillRequirement {
                    level: 24,
                    prereqs: &[13],
                },
                23 => SkillRequirement {
                    level: 24,
                    prereqs: &[18],
                },
                24 => SkillRequirement {
                    level: 24,
                    prereqs: &[19],
                },
                25 => SkillRequirement {
                    level: 30,
                    prereqs: &[20],
                },
                26 => SkillRequirement {
                    level: 30,
                    prereqs: &[15],
                },
                27 => SkillRequirement {
                    level: 30,
                    prereqs: &[16],
                },
                28 => SkillRequirement {
                    level: 30,
                    prereqs: &[17],
                },
                29 => SkillRequirement {
                    level: 30,
                    prereqs: &[14, 23],
                },
                _ => SkillRequirement {
                    level: 1,
                    prereqs: &[],
                },
            },
            CharacterClass::Warlock => SkillRequirement {
                level: 1,
                prereqs: &[],
            },
        }
    }

    fn missing_skill_prereqs(&self, slot: usize) -> Vec<usize> {
        let mut missing = Vec::new();
        self.collect_missing_skill_prereqs(slot, &mut missing);
        missing
    }

    fn collect_missing_skill_prereqs(&self, slot: usize, missing: &mut Vec<usize>) {
        if slot >= 30 {
            return;
        }

        for &prereq in Self::skill_requirement(self.class, slot).prereqs {
            self.collect_missing_skill_prereqs(prereq, missing);
            if self.skills[prereq] == 0 && !missing.contains(&prereq) {
                missing.push(prereq);
            }
        }
    }

    fn has_required_level_for_skill_tree(&self, slot: usize, visited: &mut [bool; 30]) -> bool {
        if slot >= 30 || visited[slot] {
            return true;
        }

        visited[slot] = true;
        let requirement = Self::skill_requirement(self.class, slot);
        self.level >= requirement.level
            && requirement
                .prereqs
                .iter()
                .all(|&prereq| self.has_required_level_for_skill_tree(prereq, visited))
    }

    fn skill_points_needed_to_increase(&self, slot: usize) -> u32 {
        1 + self.missing_skill_prereqs(slot).len() as u32
    }

    pub fn can_increase_skill(&self, slot: usize) -> bool {
        if slot >= 30 || self.skills[slot] >= 20 {
            return false;
        }
        if !self.has_required_level_for_skill_tree(slot, &mut [false; 30]) {
            return false;
        }
        self.skill_points_remaining >= self.skill_points_needed_to_increase(slot)
    }

    pub fn increase_skill(&mut self, slot: usize) {
        if self.can_increase_skill(slot) {
            let missing = self.missing_skill_prereqs(slot);
            let spent = 1 + missing.len() as u32;
            for prereq in missing {
                self.skills[prereq] = 1;
            }
            self.skills[slot] += 1;
            self.skill_points_remaining -= spent;
        }
    }

    fn skill_depends_on(
        class: CharacterClass,
        slot: usize,
        dependency: usize,
        visited: &mut [bool; 30],
    ) -> bool {
        if slot >= 30 || visited[slot] {
            return false;
        }

        visited[slot] = true;
        for &prereq in Self::skill_requirement(class, slot).prereqs {
            if prereq == dependency || Self::skill_depends_on(class, prereq, dependency, visited) {
                return true;
            }
        }
        false
    }

    fn has_allocated_dependent_skill(&self, slot: usize) -> bool {
        (0..30).any(|other| {
            other != slot
                && self.skills[other] > 0
                && Self::skill_depends_on(self.class, other, slot, &mut [false; 30])
        })
    }

    pub fn can_decrease_skill(&self, slot: usize) -> bool {
        if slot >= 30 || self.skills[slot] == 0 {
            return false;
        }
        self.skills[slot] > 1 || !self.has_allocated_dependent_skill(slot)
    }

    pub fn decrease_skill(&mut self, slot: usize) {
        if self.can_decrease_skill(slot) {
            self.skills[slot] -= 1;
            self.skill_points_remaining += 1;
        }
    }

    pub fn calculate_experience_for_level(level: u32) -> u32 {
        let index = level.clamp(1, 99) as usize - 1;
        EXPERIENCE_BY_LEVEL[index]
    }

    pub fn toggle_quest(&mut self, difficulty: usize, quest_idx: usize) {
        if difficulty < 3 && quest_idx < 48 {
            let current = self.quests[difficulty][quest_idx];
            if quest_is_completed(current) {
                set_quest_completed(&mut self.quests[difficulty][quest_idx], false);
                match quest_idx {
                    1 | 9 => {
                        self.skill_points_remaining = self.skill_points_remaining.saturating_sub(1)
                    }
                    25 => {
                        self.skill_points_remaining = self.skill_points_remaining.saturating_sub(2)
                    }
                    17 => self.stat_points_remaining = self.stat_points_remaining.saturating_sub(5),
                    _ => {}
                }
            } else {
                set_quest_completed(&mut self.quests[difficulty][quest_idx], true);
                match quest_idx {
                    1 | 9 => self.skill_points_remaining += 1,
                    25 => self.skill_points_remaining += 2,
                    17 => self.stat_points_remaining += 5,
                    _ => {}
                }
            }
            sync_quest_progression(&mut self.quests[difficulty]);
        }
    }
}

fn quest_is_completed(word: u16) -> bool {
    word & QUEST_REWARD_GRANTED != 0
}

fn set_quest_completed(word: &mut u16, completed: bool) {
    if completed {
        *word &= !QUEST_REWARD_PENDING;
        *word |= QUEST_CLOSED_COMPLETE;
    } else {
        *word &= !QUEST_COMPLETION_MASK;
    }
}

fn set_bool_word(word: &mut u16, completed: bool) {
    *word = if completed { QUEST_REWARD_GRANTED } else { 0 };
}

fn set_difficulty_completed_word(word: &mut u16, completed: bool) {
    *word = if completed {
        DIFFICULTY_COMPLETED_WORD
    } else {
        0
    };
}

fn sync_quest_progression(quests: &mut [u16; 48]) {
    for &idx in Savegame::VISIBLE_QUEST_INDICES {
        if quest_is_completed(quests[idx]) {
            quests[idx] &= !QUEST_REWARD_PENDING;
            quests[idx] |= QUEST_LOG_CLOSED;
        }
    }
    if quest_is_completed(quests[PRISON_OF_ICE]) {
        quests[PRISON_OF_ICE] |= QUEST_PRISON_OF_ICE_SCROLL_CONSUMED;
    } else {
        quests[PRISON_OF_ICE] &= !QUEST_PRISON_OF_ICE_SCROLL_CONSUMED;
    }

    let act_i_complete = quest_is_completed(quests[SISTERS_TO_THE_SLAUGHTER]);
    let act_ii_complete = quest_is_completed(quests[THE_SEVEN_TOMBS]);
    let act_iii_complete = quest_is_completed(quests[THE_GUARDIAN]);
    let act_iv_complete = quest_is_completed(quests[TERRORS_END]);
    let act_v_complete = quest_is_completed(quests[EVE_OF_DESTRUCTION]);

    set_bool_word(&mut quests[ACT_I_COMPLETE], act_i_complete);
    if act_i_complete {
        set_bool_word(&mut quests[ACT_II_INTRO], true);
    }

    set_bool_word(&mut quests[ACT_II_COMPLETE], act_ii_complete);
    if act_ii_complete {
        set_bool_word(&mut quests[ACT_III_INTRO], true);
    }

    set_bool_word(&mut quests[ACT_III_COMPLETE], act_iii_complete);
    if act_iii_complete {
        set_bool_word(&mut quests[ACT_IV_INTRO], true);
    }

    set_bool_word(&mut quests[ACT_IV_COMPLETE], act_iv_complete);
    if act_iv_complete {
        set_bool_word(&mut quests[ACT_V_INTRO], true);
    }

    set_difficulty_completed_word(&mut quests[ACT_V_COMPLETE], act_v_complete);
}

fn apply_progression_from_quests(raw: &mut [u8], quests: &[[u16; 48]; 3]) {
    let progression = progression_from_quests(quests);
    if let Some(byte) = raw.get_mut(LEGACY_PROGRESSION_OFFSET) {
        *byte = (*byte).max(progression);
    }
}

fn progression_from_quests(quests: &[[u16; 48]; 3]) -> u8 {
    if quest_is_completed(quests[2][EVE_OF_DESTRUCTION]) {
        PROGRESSION_HELL_COMPLETED
    } else if quest_is_completed(quests[1][EVE_OF_DESTRUCTION]) {
        PROGRESSION_NIGHTMARE_UNLOCKED
    } else if quest_is_completed(quests[0][EVE_OF_DESTRUCTION]) {
        PROGRESSION_NORMAL_UNLOCKED
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experience_table_matches_lord_of_destruction_levels() {
        assert_eq!(Savegame::calculate_experience_for_level(1), 0);
        assert_eq!(Savegame::calculate_experience_for_level(80), 681_027_665);
        assert_eq!(Savegame::calculate_experience_for_level(81), 742_730_244);
        assert_eq!(Savegame::calculate_experience_for_level(90), 1_618_470_619);
        assert_eq!(Savegame::calculate_experience_for_level(95), 2_492_671_933);
        assert_eq!(Savegame::calculate_experience_for_level(96), 2_717_422_497);
        assert_eq!(Savegame::calculate_experience_for_level(97), 2_962_400_612);
        assert_eq!(Savegame::calculate_experience_for_level(98), 3_229_426_756);
        assert_eq!(Savegame::calculate_experience_for_level(99), 3_520_485_254);
        assert_eq!(Savegame::calculate_experience_for_level(100), 3_520_485_254);
    }

    #[test]
    fn level_99_template_uses_exact_experience_breakpoint() {
        let save = Savegame::generate_template(CharacterClass::Amazon);

        assert_eq!(save.level, 99);
        assert_eq!(
            save.experience,
            Savegame::calculate_experience_for_level(99)
        );
    }

    #[test]
    fn skill_categories_use_class_tree_names_and_slots() {
        let categories = Savegame::skill_categories(CharacterClass::Paladin);

        assert_eq!(categories[0].name, "Defensive Auras");
        assert_eq!(categories[1].name, "Offensive Auras");
        assert_eq!(categories[2].name, "Combat Skills");
        assert_eq!(categories[2].slots, &[0, 1, 5, 10, 11, 15, 16, 20, 21, 25]);
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
                Savegame::get_skill_name(save.class, slot)
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
        assert_eq!(
            save.experience,
            Savegame::calculate_experience_for_level(99)
        );
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

        assert_eq!(save.consumed_resistance_scrolls(), [false, false, false]);
        assert_eq!(save.base_resistance_bonus(), 0);

        save.toggle_quest(0, PRISON_OF_ICE);

        assert_eq!(save.consumed_resistance_scrolls(), [true, false, false]);
        assert_eq!(save.base_resistance_bonus(), 10);

        save.toggle_all_quests(None, true);

        assert_eq!(save.consumed_resistance_scrolls(), [true, true, true]);
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
            for &idx in Savegame::VISIBLE_QUEST_INDICES {
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

    fn quest_words(bytes: &[u8]) -> [[u16; 48]; 3] {
        let woo_idx = bytes
            .windows(4)
            .position(|window| window == b"Woo!")
            .expect("quest header should exist");
        let mut offset = woo_idx + 10;
        let mut quests = [[0u16; 48]; 3];
        for difficulty in &mut quests {
            for (idx, word) in difficulty.iter_mut().enumerate() {
                let pos = offset + idx * 2;
                *word = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
            }
            offset += 96;
        }
        quests
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
