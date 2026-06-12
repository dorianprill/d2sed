use crate::config::Config;
use crate::model::{GameVersion, Savegame};
use iced::widget::{
    Space, button, checkbox, column, container, pick_list, row, scrollable, text, text_input,
    tooltip,
};
use iced::{Alignment, Element, Length, Size, Task};
use libd2::core::character_class::CharacterClass;
use libd2::core::character_file::CharacterStat;
use std::path::PathBuf;

mod config;
mod model;
mod save;

const INITIAL_WINDOW_WIDTH: f32 = 1100.0;
const INITIAL_WINDOW_HEIGHT: f32 = 800.0;
const SKILL_PANE_WIDTH: f32 = 620.0;

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("d2sed - Diablo 2 Save Editor")
        .window_size(Size::new(INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT))
        .run()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EditorLeftTab {
    Stats,
    ExtendedStats,
    Stash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EditorRightTab {
    Skills,
    Quests,
    Waypoints,
    Inventory,
}

enum AppState {
    LaunchScreen,
    Editor {
        save: Savegame,
        left_tab: EditorLeftTab,
        right_tab: EditorRightTab,
    },
}

struct App {
    state: AppState,
    file_path: String,
    selected_template: Option<CharacterClass>,
    selected_version: GameVersion,
    config: Config,
}

impl Default for App {
    fn default() -> Self {
        let config = confy::load("d2sed", None).unwrap_or_default();
        Self {
            state: AppState::LaunchScreen,
            file_path: String::new(),
            selected_template: None,
            selected_version: GameVersion::Legacy,
            config,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    FilePathChanged(String),
    BrowseFile,
    FileSelected(Option<PathBuf>),
    TemplateSelected(CharacterClass),
    VersionSelected(GameVersion),
    LoadCharacter,
    BackToLaunch,
    IncreaseStat(CharacterStat, u32),
    DecreaseStat(CharacterStat, u32),
    ResetStats,
    IncreaseLevel,
    DecreaseLevel,
    SaveCharacter,
    IncreaseSkill(usize),
    DecreaseSkill(usize),
    ToggleQuest(usize, usize),
    SetLeftTab(EditorLeftTab),
    SetRightTab(EditorRightTab),
    ToggleWaypoint(usize, usize),
    NameChanged(String),
    ResetSkills,
    ToggleAllWaypoints(Option<usize>, bool),
    ToggleAllQuests(Option<usize>, bool),
    ChangeExportPath,
    ExportPathSelected(Option<PathBuf>),
    ToggleDead,
    GoldChanged(u32),
    StashGoldChanged(u32),
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FilePathChanged(path) => {
                self.file_path = path;
                self.selected_template = None;
                Task::none()
            }
            Message::BrowseFile => {
                let default_path = std::env::current_dir().unwrap_or_default();
                let path = rfd::FileDialog::new()
                    .add_filter("Diablo 2 Save", &["d2s"])
                    .set_directory(default_path)
                    .pick_file();
                Task::done(Message::FileSelected(path))
            }
            Message::FileSelected(Some(path)) => {
                self.file_path = path.to_string_lossy().into_owned();
                self.selected_template = None;
                Task::none()
            }
            Message::FileSelected(None) => Task::none(),
            Message::TemplateSelected(class) => {
                self.selected_template = Some(class);
                self.file_path.clear();
                Task::none()
            }
            Message::VersionSelected(version) => {
                self.selected_version = version;
                Task::none()
            }
            Message::LoadCharacter => {
                let left_tab = EditorLeftTab::Stats;
                let right_tab = EditorRightTab::Skills;

                if let Some(class) = self.selected_template {
                    let mut save = Savegame::generate_template(class);
                    save.game_version = self.selected_version;
                    self.state = AppState::Editor {
                        save,
                        left_tab,
                        right_tab,
                    };
                } else if !self.file_path.is_empty() {
                    let path = PathBuf::from(&self.file_path);
                    if let Some(parent) = path.parent() {
                        if self.config.export_path.is_none() {
                            self.config.export_path = Some(parent.to_path_buf());
                            let _ = confy::store("d2sed", None, &self.config);
                        }
                    }

                    match Savegame::load_from_file(&self.file_path) {
                        Ok(savegame) => {
                            self.state = AppState::Editor {
                                save: savegame,
                                left_tab,
                                right_tab,
                            };
                        }
                        Err(e) => {
                            println!("Failed to load savegame: {:?}", e);
                        }
                    }
                }
                Task::none()
            }
            Message::BackToLaunch => {
                self.state = AppState::LaunchScreen;
                Task::none()
            }
            Message::IncreaseStat(stat, amount) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.increase_stat(stat, amount);
                }
                Task::none()
            }
            Message::DecreaseStat(stat, amount) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.decrease_stat(stat, amount);
                }
                Task::none()
            }
            Message::ResetStats => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.reset_stats();
                }
                Task::none()
            }
            Message::IncreaseLevel => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_level(save.level + 1);
                }
                Task::none()
            }
            Message::DecreaseLevel => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_level(save.level.saturating_sub(1));
                }
                Task::none()
            }
            Message::IncreaseSkill(slot) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.increase_skill(slot);
                }
                Task::none()
            }
            Message::DecreaseSkill(slot) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.decrease_skill(slot);
                }
                Task::none()
            }
            Message::ToggleQuest(diff, idx) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.toggle_quest(diff, idx);
                }
                Task::none()
            }
            Message::SetLeftTab(tab) => {
                if let AppState::Editor { left_tab, .. } = &mut self.state {
                    *left_tab = tab;
                }
                Task::none()
            }
            Message::SetRightTab(tab) => {
                if let AppState::Editor { right_tab, .. } = &mut self.state {
                    *right_tab = tab;
                }
                Task::none()
            }
            Message::ToggleWaypoint(diff, idx) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    if diff < 3 && idx < 39 {
                        save.waypoints[diff][idx] = !save.waypoints[diff][idx];
                    }
                }
                Task::none()
            }
            Message::NameChanged(new_name) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_name(new_name);
                }
                Task::none()
            }
            Message::ResetSkills => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.reset_skills();
                }
                Task::none()
            }
            Message::ToggleAllWaypoints(difficulty, state) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.toggle_all_waypoints(difficulty, state);
                }
                Task::none()
            }
            Message::ToggleAllQuests(difficulty, state) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.toggle_all_quests(difficulty, state);
                }
                Task::none()
            }
            Message::ChangeExportPath => {
                let default_path = self
                    .config
                    .export_path
                    .clone()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                let path = rfd::FileDialog::new()
                    .set_directory(default_path)
                    .pick_folder();
                Task::done(Message::ExportPathSelected(path))
            }
            Message::ExportPathSelected(Some(path)) => {
                self.config.export_path = Some(path);
                let _ = confy::store("d2sed", None, &self.config);
                Task::none()
            }
            Message::ExportPathSelected(None) => Task::none(),
            Message::ToggleDead => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.died = !save.died;
                }
                Task::none()
            }
            Message::GoldChanged(val) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_gold(val);
                }
                Task::none()
            }
            Message::StashGoldChanged(val) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_stashed_gold(val);
                }
                Task::none()
            }
            Message::SaveCharacter => {
                if let AppState::Editor { save, .. } = &self.state {
                    let mut path = self.config.export_path.clone().unwrap_or_else(|| {
                        rfd::FileDialog::new().pick_folder().unwrap_or_default()
                    });

                    if !path.as_os_str().is_empty() {
                        path.push(format!("{}.d2s", save.name));
                        match save.save_to_file(&path) {
                            Ok(_) => println!("Successfully saved {:?}", path),
                            Err(e) => println!("Failed to save: {:?}", e),
                        }
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match &self.state {
            AppState::LaunchScreen => self.view_launch_screen(),
            AppState::Editor {
                save,
                left_tab,
                right_tab,
            } => self.view_editor(save, left_tab, right_tab),
        }
    }

    fn view_launch_screen(&self) -> Element<'_, Message> {
        let path_input = text_input("Path to .d2s file...", &self.file_path)
            .on_input(Message::FilePathChanged)
            .on_submit(Message::LoadCharacter)
            .padding(10);

        let browse_btn = button("Browse...")
            .padding(10)
            .on_press(Message::BrowseFile);

        let file_row = row![path_input, browse_btn]
            .spacing(10)
            .align_y(Alignment::Center);

        let classes = [
            CharacterClass::Amazon,
            CharacterClass::Sorceress,
            CharacterClass::Necromancer,
            CharacterClass::Paladin,
            CharacterClass::Barbarian,
            CharacterClass::Druid,
            CharacterClass::Assassin,
            CharacterClass::Warlock,
        ];

        let mut class_row = row![].spacing(10).align_y(Alignment::Center);
        for class in classes {
            let is_selected = self.selected_template == Some(class);
            let mut btn = button(text(class.to_string()))
                .padding(10)
                .on_press(Message::TemplateSelected(class));

            if is_selected {
                btn = btn.style(button::success);
            }
            class_row = class_row.push(btn);
        }

        let version_picker = pick_list(
            &[
                GameVersion::Legacy,
                GameVersion::Resurrected,
                GameVersion::Warlock,
            ][..],
            Some(self.selected_version),
            Message::VersionSelected,
        )
        .padding(10)
        .width(Length::Fixed(200.0));

        let can_load = !self.file_path.is_empty() || self.selected_template.is_some();
        let mut load_btn = button("Load Character").padding(15);
        if can_load {
            load_btn = load_btn
                .on_press(Message::LoadCharacter)
                .style(button::primary);
        }

        let content = column![
            text("d2sed").size(40),
            text("Diablo 2 Save Editor").size(20),
            Space::new().height(30),
            text("Open Existing Character:"),
            file_row,
            Space::new().height(20),
            text("Or Create New Level 99 Template:"),
            class_row,
            Space::new().height(20),
            text("Game Version:"),
            version_picker,
            Space::new().height(40),
            load_btn,
        ]
        .spacing(10)
        .align_x(Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(20)
            .into()
    }

    fn view_editor(
        &self,
        save: &Savegame,
        left_tab: &EditorLeftTab,
        right_tab: &EditorRightTab,
    ) -> Element<'_, Message> {
        let export_folder_text = self
            .config
            .export_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "No path set".to_string());

        let header = container(
            row![
                column![
                    text_input("Name", &save.name)
                        .on_input(Message::NameChanged)
                        .size(30)
                        .width(Length::Fixed(200.0)),
                    text(format!(
                        "Level {} {} ({})",
                        save.level, save.class, save.game_version
                    ))
                    .size(14),
                ],
                Space::new().width(20),
                column![
                    text(if save.hardcore {
                        "HARDCORE"
                    } else {
                        "Softcore"
                    })
                    .size(18),
                    button(text(if save.died { "DEAD" } else { "Alive" }))
                        .on_press(Message::ToggleDead)
                        .style(if save.died {
                            button::danger
                        } else {
                            button::success
                        }),
                ],
                Space::new().width(Length::Fill),
                column![
                    text("Export Path:").size(12),
                    button(text(export_folder_text).size(12))
                        .on_press(Message::ChangeExportPath)
                        .padding(5),
                ]
                .spacing(2),
                Space::new().width(60),
                button("Save .d2s")
                    .on_press(Message::SaveCharacter)
                    .padding(10)
                    .style(button::primary),
                Space::new().width(60),
                button("Back").on_press(Message::BackToLaunch).padding(10),
            ]
            .align_y(Alignment::End)
            .padding(10),
        )
        .style(|_| container::Style {
            background: Some(iced::Color::from_rgb(0.1, 0.1, 0.1).into()),
            ..Default::default()
        })
        .width(Length::Fill);

        let left_tabs = row![
            button("Stats")
                .on_press(Message::SetLeftTab(EditorLeftTab::Stats))
                .style(if *left_tab == EditorLeftTab::Stats {
                    button::primary
                } else {
                    button::secondary
                }),
            button("Extended")
                .on_press(Message::SetLeftTab(EditorLeftTab::ExtendedStats))
                .style(if *left_tab == EditorLeftTab::ExtendedStats {
                    button::primary
                } else {
                    button::secondary
                }),
            button("Stash")
                .on_press(Message::SetLeftTab(EditorLeftTab::Stash))
                .style(if *left_tab == EditorLeftTab::Stash {
                    button::primary
                } else {
                    button::secondary
                }),
        ]
        .spacing(5);

        let left_content: Element<Message> = match left_tab {
            EditorLeftTab::Stats => {
                let stat_row = |name: String, value: u32, stat: CharacterStat| {
                    row![
                        text(name).width(Length::Fixed(100.0)),
                        button("-10")
                            .on_press(Message::DecreaseStat(stat, 10))
                            .padding(5),
                        button("-")
                            .on_press(Message::DecreaseStat(stat, 1))
                            .padding(5),
                        text(value.to_string())
                            .width(Length::Fixed(40.0))
                            .align_x(Alignment::Center),
                        button("+")
                            .on_press(Message::IncreaseStat(stat, 1))
                            .padding(5),
                        button("+10")
                            .on_press(Message::IncreaseStat(stat, 10))
                            .padding(5),
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center)
                };

                column![
                    text("Attributes").size(24),
                    row![
                        text("Level:").width(Length::Fixed(100.0)),
                        button("-").on_press(Message::DecreaseLevel).padding(5),
                        text(save.level.to_string())
                            .width(Length::Fixed(40.0))
                            .align_x(Alignment::Center),
                        button("+").on_press(Message::IncreaseLevel).padding(5),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    text(format!("Experience: {}", save.experience)),
                    Space::new().height(20),
                    stat_row(
                        "Strength:".to_string(),
                        save.strength,
                        CharacterStat::Strength
                    ),
                    stat_row(
                        "Dexterity:".to_string(),
                        save.dexterity,
                        CharacterStat::Dexterity
                    ),
                    stat_row(
                        "Vitality:".to_string(),
                        save.vitality,
                        CharacterStat::Vitality
                    ),
                    stat_row("Energy:".to_string(), save.energy, CharacterStat::Energy),
                    Space::new().height(10),
                    row![
                        text(format!(
                            "Stat Points Remaining: {}",
                            save.stat_points_remaining
                        )),
                        button("Reset Stats")
                            .on_press(Message::ResetStats)
                            .padding(5)
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    Space::new().height(20),
                    text(format!("HP: {} / {}", save.current_hp, save.max_hp)),
                    text(format!("Mana: {} / {}", save.current_mana, save.max_mana)),
                    text(format!(
                        "Stamina: {} / {}",
                        save.current_stamina, save.max_stamina
                    )),
                ]
                .spacing(10)
                .into()
            }
            EditorLeftTab::ExtendedStats => column![
                text("Extended Stats").size(24),
                text("This page will eventually show combined item stats.").size(14),
            ]
            .spacing(10)
            .into(),
            EditorLeftTab::Stash => column![
                row![
                    text("Gold:").size(18),
                    text_input("0", &save.stashed_gold.to_string())
                        .on_input(|s| Message::StashGoldChanged(s.parse().unwrap_or(0)))
                        .padding(5)
                        .width(Length::Fixed(150.0)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                self.draw_grid(10, 10, 35.0, "Stash".to_string()),
            ]
            .spacing(10)
            .into(),
        };

        let right_tabs = row![
            button("Skills")
                .on_press(Message::SetRightTab(EditorRightTab::Skills))
                .style(if *right_tab == EditorRightTab::Skills {
                    button::primary
                } else {
                    button::secondary
                }),
            button("Quests")
                .on_press(Message::SetRightTab(EditorRightTab::Quests))
                .style(if *right_tab == EditorRightTab::Quests {
                    button::primary
                } else {
                    button::secondary
                }),
            button("Waypoints")
                .on_press(Message::SetRightTab(EditorRightTab::Waypoints))
                .style(if *right_tab == EditorRightTab::Waypoints {
                    button::primary
                } else {
                    button::secondary
                }),
            button("Inventory")
                .on_press(Message::SetRightTab(EditorRightTab::Inventory))
                .style(if *right_tab == EditorRightTab::Inventory {
                    button::primary
                } else {
                    button::secondary
                }),
        ]
        .spacing(5);

        let right_content: Element<Message> = match right_tab {
            EditorRightTab::Skills => {
                let mut skills_col = column![
                    container(
                        row![
                            text("Skill Trees").size(24),
                            Space::new().width(Length::Fill),
                            button("Reset Skills")
                                .on_press(Message::ResetSkills)
                                .padding(5),
                        ]
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fixed(SKILL_PANE_WIDTH)),
                    text(format!(
                        "Skill Points Remaining: {}",
                        save.skill_points_remaining
                    )),
                    Space::new().height(10),
                ]
                .spacing(5);

                let mut skill_trees_row = row![].spacing(20).width(Length::Fixed(SKILL_PANE_WIDTH));
                for tree_idx in 0..3 {
                    let mut tree_col = column![].spacing(5);
                    for skill_idx in 0..10 {
                        let slot = tree_idx * 10 + skill_idx;
                        let name = Savegame::get_skill_name(save.class, slot);
                        let value = save.skills[slot];

                        let can_increase = save.can_increase_skill(slot);
                        let mut plus_btn = button("+").padding(2);
                        if can_increase {
                            plus_btn = plus_btn.on_press(Message::IncreaseSkill(slot));
                        }

                        let skill_row = row![
                            text(name).width(Length::Fixed(120.0)),
                            button("-")
                                .on_press(Message::DecreaseSkill(slot))
                                .padding(2),
                            text(value.to_string())
                                .width(Length::Fixed(24.0))
                                .align_x(Alignment::Center),
                            plus_btn,
                        ]
                        .align_y(Alignment::Center);

                        tree_col = tree_col.push(skill_row);
                    }
                    skill_trees_row = skill_trees_row.push(tree_col);
                }
                skills_col = skills_col.push(skill_trees_row);
                skills_col.into()
            }
            EditorRightTab::Quests => {
                let acts: [(&str, &[usize]); 5] = [
                    ("Act I", &[1, 2, 4, 5, 3, 6]),
                    ("Act II", &[9, 10, 11, 12, 13, 14]),
                    ("Act III", &[20, 19, 18, 17, 21, 22]),
                    ("Act IV", &[25, 26, 27]),
                    ("Act V", &[35, 36, 37, 38, 39, 40]),
                ];

                let difficulties = ["Normal", "Nightmare", "Hell"];

                let all_completed = difficulties.iter().enumerate().all(|(d, _)| {
                    Savegame::VISIBLE_QUEST_INDICES
                        .iter()
                        .all(|&q| (save.quests[d][q] & 1) == 1)
                });

                let quest_header = row![
                    text("Quest Log").size(24),
                    Space::new().width(40),
                    checkbox(all_completed)
                        .label("Toggle All Difficulties")
                        .on_toggle(move |state| Message::ToggleAllQuests(None, state)),
                ]
                .spacing(10)
                .align_y(Alignment::Center);

                let mut diff_row = row![].spacing(20);

                for (diff_idx, diff_name) in difficulties.iter().enumerate() {
                    let diff_all_done = Savegame::VISIBLE_QUEST_INDICES
                        .iter()
                        .all(|&q| (save.quests[diff_idx][q] & 1) == 1);

                    let mut diff_col = column![
                        text(*diff_name).size(20),
                        checkbox(diff_all_done)
                            .label("All")
                            .on_toggle(move |state| Message::ToggleAllQuests(
                                Some(diff_idx),
                                state
                            )),
                    ]
                    .spacing(15);

                    for (act_idx, (act_name, quest_indices)) in acts.iter().enumerate() {
                        let mut act_col = column![text(*act_name).size(16)].spacing(5);
                        let mut quest_grid = column![].spacing(2);

                        for chunk in quest_indices.chunks(3) {
                            let mut q_row = row![].spacing(5);
                            for (q_pos, &q_idx) in chunk.iter().enumerate() {
                                let is_completed = (save.quests[diff_idx][q_idx] & 1) == 1;
                                let q_name = Savegame::get_quest_name(q_idx);

                                let btn = button(text(format!("A{}Q{}", act_idx + 1, q_pos + 1)))
                                    .on_press(Message::ToggleQuest(diff_idx, q_idx))
                                    .style(if is_completed {
                                        button::success
                                    } else {
                                        button::secondary
                                    })
                                    .width(Length::Fixed(60.0));

                                q_row = q_row.push(tooltip(btn, q_name, tooltip::Position::Top));
                            }
                            quest_grid = quest_grid.push(q_row);
                        }
                        act_col = act_col.push(quest_grid);
                        diff_col = diff_col.push(act_col);
                    }
                    diff_row = diff_row.push(diff_col);
                }
                column![quest_header, Space::new().height(10), diff_row]
                    .spacing(10)
                    .into()
            }
            EditorRightTab::Waypoints => {
                let act_ranges = [
                    ("Act I", 0..9),
                    ("Act II", 9..18),
                    ("Act III", 18..27),
                    ("Act IV", 27..30),
                    ("Act V", 30..39),
                ];
                let wp_names = [
                    "Rogue Encampment",
                    "Cold Plains",
                    "Stony Field",
                    "Dark Wood",
                    "Black Marsh",
                    "Outer Cloister",
                    "Jail Level 1",
                    "Inner Cloister",
                    "Catacombs Level 2",
                    "Lut Gholein",
                    "Sewers Level 2",
                    "Dry Hills",
                    "Halls of the Dead L2",
                    "Far Oasis",
                    "Lost City",
                    "Palace Cellar L1",
                    "Arcane Sanctuary",
                    "Canyon of the Magi",
                    "Kurast Docks",
                    "Spider Forest",
                    "Great Marsh",
                    "Flayer Jungle",
                    "Lower Kurast",
                    "Kurast Bazaar",
                    "Upper Kurast",
                    "Travincal",
                    "Durance of Hate L2",
                    "Pandemonium Fortress",
                    "City of the Damned",
                    "River of Flame",
                    "Harrogath",
                    "Frigid Highlands",
                    "Arreat Plateau",
                    "Crystalline Passage",
                    "Halls of Pain",
                    "Glacial Trail",
                    "Frozen Tundra",
                    "Ancients' Way",
                    "Worldstone Keep L2",
                ];

                let difficulties = ["Normal", "Nightmare", "Hell"];

                let all_wps_done = (0..3).all(|d| (0..39).all(|w| save.waypoints[d][w]));

                let wp_header = row![
                    text("Waypoints").size(24),
                    Space::new().width(40),
                    checkbox(all_wps_done)
                        .label("Unlock All Diffs")
                        .on_toggle(|state| Message::ToggleAllWaypoints(None, state)),
                ]
                .align_y(Alignment::Center);

                let mut diff_row = row![].spacing(20);
                for (diff_idx, diff_name) in difficulties.iter().enumerate() {
                    let diff_all_wps = (0..39).all(|w| save.waypoints[diff_idx][w]);

                    let mut diff_col = column![
                        text(*diff_name).size(20),
                        checkbox(diff_all_wps).label("All").on_toggle(move |state| {
                            Message::ToggleAllWaypoints(Some(diff_idx), state)
                        }),
                    ]
                    .spacing(10);

                    for (act_name, range) in act_ranges.clone() {
                        let mut act_col = column![text(act_name).size(16)].spacing(2);
                        for wp_idx in range {
                            let is_active = save.waypoints[diff_idx][wp_idx];
                            act_col = act_col.push(
                                button(text(wp_names[wp_idx]).size(12))
                                    .on_press(Message::ToggleWaypoint(diff_idx, wp_idx))
                                    .style(if is_active {
                                        button::success
                                    } else {
                                        button::secondary
                                    })
                                    .padding(2)
                                    .width(Length::Fixed(150.0)),
                            );
                        }
                        diff_col = diff_col.push(act_col);
                    }
                    diff_row = diff_row.push(diff_col);
                }

                column![
                    wp_header,
                    Space::new().height(10),
                    scrollable(diff_row).height(Length::Fill)
                ]
                .spacing(10)
                .into()
            }
            EditorRightTab::Inventory => {
                let slot = |w: f32, h: f32, label: String| {
                    container(text(label).size(10))
                        .width(Length::Fixed(w * 40.0))
                        .height(Length::Fixed(h * 40.0))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(iced::Color::from_rgb(0.15, 0.15, 0.15).into()),
                            border: iced::Border {
                                color: iced::Color::from_rgb(0.3, 0.3, 0.3),
                                width: 1.0,
                                radius: iced::border::Radius::default(),
                            },
                            ..Default::default()
                        })
                };

                let gear_layout = column![
                    row![
                        Space::new().width(Length::Fixed(80.0)),
                        slot(2.0, 2.0, "HELMET".to_string()),
                        Space::new().width(Length::Fixed(40.0)),
                        slot(1.0, 1.0, "AMULET".to_string()),
                    ]
                    .spacing(10),
                    row![
                        slot(2.0, 4.0, "WEAPON L".to_string()),
                        slot(2.0, 3.0, "ARMOR".to_string()),
                        slot(2.0, 4.0, "WEAPON R".to_string()),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    row![
                        slot(2.0, 1.0, "GLOVES".to_string()),
                        slot(2.0, 1.0, "BELT".to_string()),
                        slot(2.0, 1.0, "BOOTS".to_string()),
                    ]
                    .spacing(10),
                    row![
                        Space::new().width(Length::Fixed(80.0)),
                        slot(1.0, 1.0, "RING L".to_string()),
                        Space::new().width(Length::Fixed(40.0)),
                        slot(1.0, 1.0, "RING R".to_string()),
                    ]
                    .spacing(10),
                ]
                .spacing(10)
                .align_x(Alignment::Center);

                column![
                    row![
                        text("Gold:").size(18),
                        text_input("0", &save.gold.to_string())
                            .on_input(|s| Message::GoldChanged(s.parse().unwrap_or(0)))
                            .padding(5)
                            .width(Length::Fixed(150.0)),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    Space::new().height(20),
                    gear_layout,
                    Space::new().height(20),
                    self.draw_grid(10, 4, 40.0, "Inventory".to_string()),
                ]
                .spacing(10)
                .into()
            }
        };

        let main_content = row![
            column![left_tabs, Space::new().height(10), left_content]
                .width(Length::Fixed(400.0))
                .padding(10),
            column![right_tabs, Space::new().height(10), right_content]
                .width(Length::Fill)
                .padding(10),
        ]
        .spacing(20);

        column![header, main_content.height(Length::Fill),].into()
    }

    fn draw_grid(
        &self,
        cols: usize,
        rows: usize,
        cell_size: f32,
        title: String,
    ) -> Element<'_, Message> {
        let mut grid_col = column![text(title).size(20), Space::new().height(10)].spacing(5);
        for _ in 0..rows {
            let mut row_cells = row![].spacing(2);
            for _ in 0..cols {
                row_cells = row_cells.push(
                    container(Space::new())
                        .width(Length::Fixed(cell_size))
                        .height(Length::Fixed(cell_size))
                        .style(|_| iced::widget::container::Style {
                            background: Some(iced::Color::from_rgb(0.2, 0.2, 0.2).into()),
                            border: iced::Border {
                                color: iced::Color::from_rgb(0.4, 0.4, 0.4),
                                width: 1.0,
                                radius: iced::border::Radius::default(),
                            },
                            ..Default::default()
                        }),
                );
            }
            grid_col = grid_col.push(row_cells);
        }
        grid_col.into()
    }
}
