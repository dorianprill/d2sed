use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Task};
use libd2::core::character_class::CharacterClass;
use libd2::core::character_file::CharacterStat;
use std::path::PathBuf;

mod model;
mod save;
use model::Savegame;

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("d2sed - Diablo 2 Save Editor")
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
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: AppState::LaunchScreen,
            file_path: String::new(),
            selected_template: None,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    FilePathChanged(String),
    BrowseFile,
    FileSelected(Option<PathBuf>),
    TemplateSelected(CharacterClass),
    LoadCharacter,
    BackToLaunch,
    IncreaseStat(CharacterStat),
    DecreaseStat(CharacterStat),
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
            Message::LoadCharacter => {
                if let Some(class) = self.selected_template {
                    self.state = AppState::Editor {
                        save: Savegame::generate_template(class),
                        left_tab: EditorLeftTab::Stats,
                        right_tab: EditorRightTab::Skills,
                    };
                } else if !self.file_path.is_empty() {
                    match Savegame::load_from_file(&self.file_path) {
                        Ok(savegame) => {
                            self.state = AppState::Editor {
                                save: savegame,
                                left_tab: EditorLeftTab::Stats,
                                right_tab: EditorRightTab::Skills,
                            };
                        }
                        Err(e) => {
                            println!("Failed to load savegame: {:?}", e); // TODO: show in UI
                        }
                    }
                }
                Task::none()
            }
            Message::BackToLaunch => {
                self.state = AppState::LaunchScreen;
                Task::none()
            }
            Message::IncreaseStat(stat) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.increase_stat(stat);
                }
                Task::none()
            }
            Message::DecreaseStat(stat) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.decrease_stat(stat);
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
            Message::SaveCharacter => {
                if let AppState::Editor { save, .. } = &self.state {
                    let path = &self.file_path;
                    if !path.is_empty() {
                        match save.save_to_file(path) {
                            Ok(_) => println!("Successfully saved {}", path), // TODO: UI log
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
        let header = container(
            row![
                column![
                    text(format!("{}", save.name)).size(30),
                    text(format!("Level {} {}", save.level, save.class)).size(18),
                ],
                Space::new().width(20),
                column![
                    text(if save.hardcore {
                        "HARDCORE"
                    } else {
                        "Softcore"
                    })
                    .size(18),
                    text(if save.died { "DEAD" } else { "Alive" }).color(if save.died {
                        iced::Color::from_rgb(1.0, 0.0, 0.0)
                    } else {
                        iced::Color::from_rgb(0.0, 1.0, 0.0)
                    }),
                ],
                Space::new().width(Length::Fill),
                button("Save .d2s")
                    .on_press(Message::SaveCharacter)
                    .padding(10)
                    .style(button::primary),
                button("Back").on_press(Message::BackToLaunch).padding(10),
            ]
            .align_y(Alignment::Center)
            .padding(10),
        )
        .style(|_| container::Style {
            background: Some(iced::Color::from_rgb(0.1, 0.1, 0.1).into()),
            ..Default::default()
        })
        .width(Length::Fill);

        // Left Pane Tabs
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
                        button("-").on_press(Message::DecreaseStat(stat)).padding(5),
                        text(value.to_string())
                            .width(Length::Fixed(40.0))
                            .align_x(Alignment::Center),
                        button("+").on_press(Message::IncreaseStat(stat)).padding(5),
                    ]
                    .spacing(10)
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
                text("Gold in Hand:"),
                text_input("0", &save.gold.to_string()).padding(5),
                text("Gold in Stash:"),
                text_input("0", &save.stashed_gold.to_string()).padding(5),
            ]
            .spacing(10)
            .into(),
            EditorLeftTab::Stash => self
                .draw_grid(10, 10, 35.0, "Shared/Private Stash (10x10)".to_string())
                .into(),
        };

        // Right Pane Tabs
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
                    text("Skill Trees").size(24),
                    text(format!(
                        "Skill Points Remaining: {}",
                        save.skill_points_remaining
                    )),
                    Space::new().height(10),
                ]
                .spacing(5);

                let mut skill_trees_row = row![].spacing(20);
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
                    ("Act I", &[0, 1, 2, 3, 4, 5]),
                    ("Act II", &[9, 10, 11, 12, 13, 14]),
                    ("Act III", &[17, 18, 19, 20, 21, 22]),
                    ("Act IV", &[25, 26, 27]),
                    ("Act V", &[35, 36, 37, 38, 39, 40]),
                ];

                let difficulties = ["Normal", "Nightmare", "Hell"];
                let mut diff_tabs = row![].spacing(20);

                for (diff_idx, diff_name) in difficulties.iter().enumerate() {
                    let mut diff_col = column![text(*diff_name).size(20)].spacing(15);

                    for (act_name, quest_indices) in acts {
                        let mut act_col = column![text(act_name).size(16)].spacing(5);
                        let mut quest_grid = column![].spacing(2);

                        // 2x3 grid logic for quests
                        for chunk in quest_indices.chunks(3) {
                            let mut q_row = row![].spacing(5);
                            for &q_idx in chunk {
                                let is_completed = (save.quests[diff_idx][q_idx] & 1) == 1;
                                q_row = q_row.push(
                                    button(text(format!("Q{}", q_idx)))
                                        .on_press(Message::ToggleQuest(diff_idx, q_idx))
                                        .style(if is_completed {
                                            button::success
                                        } else {
                                            button::secondary
                                        })
                                        .width(Length::Fixed(40.0)),
                                );
                            }
                            quest_grid = quest_grid.push(q_row);
                        }
                        act_col = act_col.push(quest_grid);
                        diff_col = diff_col.push(act_col);
                    }
                    diff_tabs = diff_tabs.push(diff_col);
                }
                column![
                    text("Quest Log").size(24),
                    Space::new().height(10),
                    diff_tabs
                ]
                .spacing(10)
                .into()
            }
            EditorRightTab::Waypoints => {
                let wp_names = [
                    // Act 1
                    "Rogue Encampment",
                    "Cold Plains",
                    "Stony Field",
                    "Dark Wood",
                    "Black Marsh",
                    "Outer Cloister",
                    "Jail Level 1",
                    "Inner Cloister",
                    "Catacombs Level 2",
                    // Act 2
                    "Lut Gholein",
                    "Sewers Level 2",
                    "Dry Hills",
                    "Halls of the Dead L2",
                    "Far Oasis",
                    "Lost City",
                    "Palace Cellar L1",
                    "Arcane Sanctuary",
                    "Canyon of the Magi",
                    // Act 3
                    "Kurast Docks",
                    "Spider Forest",
                    "Great Marsh",
                    "Flayer Jungle",
                    "Lower Kurast",
                    "Kurast Bazaar",
                    "Upper Kurast",
                    "Travincal",
                    "Durance of Hate L2",
                    // Act 4
                    "Pandemonium Fortress",
                    "City of the Damned",
                    "River of Flame",
                    // Act 5
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
                let mut diff_row = row![].spacing(20);
                for (diff_idx, diff_name) in difficulties.iter().enumerate() {
                    let mut diff_col = column![text(*diff_name).size(20)].spacing(5);
                    for (wp_idx, wp_name) in wp_names.iter().enumerate() {
                        let is_active = save.waypoints[diff_idx][wp_idx];
                        diff_col = diff_col.push(
                            button(text(*wp_name))
                                .on_press(Message::ToggleWaypoint(diff_idx, wp_idx))
                                .style(if is_active {
                                    button::success
                                } else {
                                    button::secondary
                                })
                                .padding(5)
                                .width(Length::Fixed(180.0)),
                        );
                    }
                    diff_row = diff_row.push(diff_col);
                }

                column![
                    text("Waypoints").size(24),
                    Space::new().height(10),
                    scrollable(diff_row).height(Length::Fill),
                ]
                .spacing(10)
                .into()
            }
            EditorRightTab::Inventory => self
                .draw_grid(10, 4, 40.0, "Character Inventory (10x4)".to_string())
                .into(),
        };

        let main_content = row![
            column![left_tabs, Space::new().height(10), left_content]
                .width(Length::FillPortion(1))
                .padding(10),
            column![right_tabs, Space::new().height(10), right_content]
                .width(Length::FillPortion(2))
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
