use iced::widget::{Space, button, column, container, row, text, text_input};
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

enum AppState {
    LaunchScreen,
    Editor(Savegame),
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
                    self.state = AppState::Editor(Savegame::generate_template(class));
                } else if !self.file_path.is_empty() {
                    match Savegame::load_from_file(&self.file_path) {
                        Ok(savegame) => {
                            self.state = AppState::Editor(savegame);
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
                if let AppState::Editor(save) = &mut self.state {
                    save.increase_stat(stat);
                }
                Task::none()
            }
            Message::DecreaseStat(stat) => {
                if let AppState::Editor(save) = &mut self.state {
                    save.decrease_stat(stat);
                }
                Task::none()
            }
            Message::ResetStats => {
                if let AppState::Editor(save) = &mut self.state {
                    save.reset_stats();
                }
                Task::none()
            }
            Message::IncreaseLevel => {
                if let AppState::Editor(save) = &mut self.state {
                    save.set_level(save.level + 1);
                }
                Task::none()
            }
            Message::DecreaseLevel => {
                if let AppState::Editor(save) = &mut self.state {
                    save.set_level(save.level.saturating_sub(1));
                }
                Task::none()
            }
            Message::IncreaseSkill(slot) => {
                if let AppState::Editor(save) = &mut self.state {
                    save.increase_skill(slot);
                }
                Task::none()
            }
            Message::DecreaseSkill(slot) => {
                if let AppState::Editor(save) = &mut self.state {
                    save.decrease_skill(slot);
                }
                Task::none()
            }
            Message::ToggleQuest(diff, idx) => {
                if let AppState::Editor(save) = &mut self.state {
                    save.toggle_quest(diff, idx);
                }
                Task::none()
            }
            Message::SaveCharacter => {
                if let AppState::Editor(save) = &self.state {
                    if save.char_file.is_some() {
                        let path = &self.file_path;
                        if !path.is_empty() {
                            match save.save_to_file(path) {
                                Ok(_) => println!("Successfully saved {}", path), // TODO: UI log
                                Err(e) => println!("Failed to save: {:?}", e),
                            }
                        }
                    } else {
                        println!("Saving templates is not fully supported yet.");
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match &self.state {
            AppState::LaunchScreen => self.view_launch_screen(),
            AppState::Editor(save) => self.view_editor(save),
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

    fn view_editor(&self, save: &Savegame) -> Element<'_, Message> {
        let top_pane = row![
            button("Save .d2s")
                .on_press(Message::SaveCharacter)
                .padding(10),
            button("Back to Launch")
                .on_press(Message::BackToLaunch)
                .padding(10),
            Space::new().width(Length::Fill),
            text(format!("Loaded: {} (Class: {})", save.name, save.class)).size(20)
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding(10);

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

        let left_pane = column![
            text("Character Overview").size(24),
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
            Space::new().height(20),
            text("Detailed Stats").size(24),
            text("Stash").size(24),
        ]
        .spacing(10)
        .padding(10)
        .width(Length::FillPortion(1));

        let mut skills_col = column![
            text("Skills").size(24),
            text(format!(
                "Skill Points Remaining: {}",
                save.skill_points_remaining
            )),
            Space::new().height(10),
        ]
        .spacing(5);

        // Group skills into 3 columns (e.g. 10 skills per tree)
        let mut skill_trees_row = row![].spacing(20);
        for tree_idx in 0..3 {
            let mut tree_col = column![].spacing(5);
            for skill_idx in 0..10 {
                let slot = tree_idx * 10 + skill_idx;
                let name = Savegame::get_skill_name(save.class, slot);
                let value = save.skills[slot];

                let skill_row = row![
                    text(name).width(Length::Fixed(120.0)),
                    button("-")
                        .on_press(Message::DecreaseSkill(slot))
                        .padding(2),
                    text(value.to_string())
                        .width(Length::Fixed(24.0))
                        .align_x(Alignment::Center),
                    button("+")
                        .on_press(Message::IncreaseSkill(slot))
                        .padding(2),
                ]
                .align_y(Alignment::Center);

                tree_col = tree_col.push(skill_row);
            }
            skill_trees_row = skill_trees_row.push(tree_col);
        }
        skills_col = skills_col.push(skill_trees_row);

        let mut quests_col =
            column![text("Key Quests").size(24), Space::new().height(10),].spacing(5);

        let difficulties = ["Normal", "Nightmare", "Hell"];
        let key_quests = [
            (1, "Den of Evil (+1 Skill)"),
            (9, "Radament's Lair (+1 Skill)"),
            (17, "Lam Esen's Tome (+5 Stats)"),
            (20, "Golden Bird (+20 Life)"),
            (25, "Fallen Angel (+2 Skills)"),
            (37, "Prison of Ice (+10 All Res)"),
        ];

        let mut diff_row = row![].spacing(20);
        for (diff_idx, diff_name) in difficulties.iter().enumerate() {
            let mut diff_col = column![text(*diff_name).size(18)].spacing(5);
            for &(quest_idx, quest_name) in &key_quests {
                let is_completed = (save.quests[diff_idx][quest_idx] & 1) == 1;
                let status_text = if is_completed { "[X]" } else { "[ ]" };

                diff_col = diff_col.push(
                    button(text(format!("{} {}", status_text, quest_name)))
                        .on_press(Message::ToggleQuest(diff_idx, quest_idx))
                        .style(if is_completed {
                            button::success
                        } else {
                            button::secondary
                        }),
                );
            }
            diff_row = diff_row.push(diff_col);
        }
        quests_col = quests_col.push(diff_row);

        let draw_grid = |cols: usize, rows: usize, cell_size: f32, title: String| {
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
            grid_col
        };

        // Determine grid sizes based on expansion / resurrected (assuming Resurrected layout 10x10 stash for now)
        let inventory_grid = draw_grid(10, 4, 30.0, "Inventory (10x4)".to_string());
        let stash_grid = draw_grid(10, 10, 30.0, "Stash (10x10)".to_string());

        let right_pane = column![
            skills_col,
            Space::new().height(20),
            quests_col,
            Space::new().height(20),
            text("Inventory & Stash").size(24),
            row![inventory_grid, Space::new().width(20), stash_grid].spacing(20),
        ]
        .spacing(10)
        .padding(10)
        .width(Length::FillPortion(2));

        let center_content = row![left_pane, right_pane].spacing(20);

        let bottom_pane = container(text("Log: Editor active.").size(14))
            .padding(10)
            .width(Length::Fill);

        let main_layout = column![top_pane, center_content.height(Length::Fill), bottom_pane,];

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
