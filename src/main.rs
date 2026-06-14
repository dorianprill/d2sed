use crate::config::Config;
use crate::model::{GameVersion, Savegame};
use iced::widget::{
    Space, button, checkbox, column, container, opaque, pick_list, row, scrollable, stack, text,
    text_input, tooltip,
};
use iced::{Alignment, Element, Length, Size, Task};
use libd2::core::character_class::CharacterClass;
use libd2::core::character_file::CharacterStat;
use libd2::core::quest::{self, VISIBLE_QUEST_ACTS, VISIBLE_QUEST_INDICES};
use libd2::core::skills;
use libd2::core::waypoint::{WAYPOINT_ACTS, WAYPOINT_COUNT, WAYPOINT_NAMES};
use std::path::Path;
use std::path::PathBuf;

mod config;
mod model;
mod save;

const INITIAL_WINDOW_WIDTH: f32 = 1080.0;
const INITIAL_WINDOW_HEIGHT: f32 = 760.0;
const SKILL_PANE_WIDTH: f32 = 620.0;

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("d2sed - Diablo 2 Save Editor")
        .window_size(Size::new(INITIAL_WINDOW_WIDTH, INITIAL_WINDOW_HEIGHT))
        .run()
}

fn default_save_folder() -> PathBuf {
    user_home_dir()
        .map(|home| default_save_folder_for_home(&home, home.join("Saved Games").is_dir()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

fn default_save_folder_for_home(home: &Path, saved_games_exists: bool) -> PathBuf {
    if saved_games_exists {
        home.join("Saved Games")
    } else {
        home.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_save_folder_prefers_saved_games_when_available() {
        let home = PathBuf::from(r"C:\Users\Test");

        assert_eq!(
            default_save_folder_for_home(&home, true),
            home.join("Saved Games")
        );
    }

    #[test]
    fn default_save_folder_falls_back_to_home_without_saved_games() {
        let home = PathBuf::from(r"C:\Users\Test");

        assert_eq!(default_save_folder_for_home(&home, false), home);
    }
}

fn user_home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .filter(|value| !value.as_os_str().is_empty())
        .or_else(|| std::env::var_os("HOME").filter(|value| !value.as_os_str().is_empty()))
        .map(PathBuf::from)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EditorLeftTab {
    Character,
    Advanced,
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
        save: Box<Savegame>,
        left_tab: EditorLeftTab,
        right_tab: EditorRightTab,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfirmationAction {
    BackToLaunch,
    UpgradeVersion,
}

#[derive(Debug, Clone)]
struct ConfirmationDialog {
    title: &'static str,
    description: &'static str,
    confirm_label: &'static str,
    action: ConfirmationAction,
}

struct App {
    state: AppState,
    file_path: String,
    selected_template: Option<CharacterClass>,
    selected_version: GameVersion,
    config: Config,
    export_path_input: String,
    status_message: Option<String>,
    confirmation: Option<ConfirmationDialog>,
}

impl Default for App {
    fn default() -> Self {
        let config: Config = confy::load("d2sed", None).unwrap_or_default();
        let export_path_input = config
            .export_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });

        Self {
            state: AppState::LaunchScreen,
            file_path: String::new(),
            selected_template: None,
            selected_version: GameVersion::Legacy,
            config,
            export_path_input,
            status_message: None,
            confirmation: None,
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
    MinimizeStat(CharacterStat),
    MaximizeStat(CharacterStat),
    ResetStats,
    IncreaseLevel(u32),
    DecreaseLevel(u32),
    MinimizeLevel,
    MaximizeLevel,
    SaveCharacter,
    UpgradeVersion,
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
    BrowseExportPath,
    ExportPathSelected(Option<PathBuf>),
    ExportPathChanged(String),
    ToggleDead,
    GoldChanged(u32),
    StashGoldChanged(u32),
    ConfirmDialog,
    CancelDialog,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FilePathChanged(path) => {
                self.file_path = path;
                self.selected_template = None;
                self.status_message = None;
                Task::none()
            }
            Message::BrowseFile => {
                let path = rfd::FileDialog::new()
                    .add_filter("Diablo 2 Save", &["d2s"])
                    .set_directory(default_save_folder())
                    .pick_file();
                Task::done(Message::FileSelected(path))
            }
            Message::FileSelected(Some(path)) => {
                self.file_path = path.to_string_lossy().into_owned();
                self.selected_template = None;
                self.status_message = None;
                Task::none()
            }
            Message::FileSelected(None) => Task::none(),
            Message::TemplateSelected(class) => {
                self.selected_template = Some(class);
                self.file_path.clear();
                self.status_message = None;
                Task::none()
            }
            Message::VersionSelected(version) => {
                self.selected_version = version;
                Task::none()
            }
            Message::LoadCharacter => {
                let left_tab = EditorLeftTab::Character;
                let right_tab = EditorRightTab::Skills;

                if let Some(class) = self.selected_template {
                    let mut save = Savegame::generate_template(class);
                    save.game_version = self.selected_version;
                    self.state = AppState::Editor {
                        save: Box::new(save),
                        left_tab,
                        right_tab,
                    };
                } else if !self.file_path.is_empty() {
                    let path = PathBuf::from(&self.file_path);
                    if let Some(parent) = path.parent()
                        && self.config.export_path.is_none()
                    {
                        self.config.export_path = Some(parent.to_path_buf());
                        self.export_path_input = parent.to_string_lossy().into_owned();
                        let _ = confy::store("d2sed", None, &self.config);
                    }

                    match Savegame::load_from_file(&self.file_path) {
                        Ok(savegame) => {
                            self.status_message = None;
                            self.state = AppState::Editor {
                                save: Box::new(savegame),
                                left_tab,
                                right_tab,
                            };
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Failed to load savegame: {e}"));
                        }
                    }
                }
                Task::none()
            }
            Message::BackToLaunch => {
                self.confirmation = Some(ConfirmationDialog {
                    title: "Return to start screen?",
                    description: "Unsaved changes in the current editor session will be discarded.",
                    confirm_label: "Back",
                    action: ConfirmationAction::BackToLaunch,
                });
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
            Message::MinimizeStat(stat) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.minimize_stat(stat);
                }
                Task::none()
            }
            Message::MaximizeStat(stat) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.maximize_stat(stat);
                }
                Task::none()
            }
            Message::ResetStats => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.reset_stats();
                }
                Task::none()
            }
            Message::IncreaseLevel(amount) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_level(save.level.saturating_add(amount));
                }
                Task::none()
            }
            Message::DecreaseLevel(amount) => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_level(save.level.saturating_sub(amount));
                }
                Task::none()
            }
            Message::MinimizeLevel => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_level(1);
                }
                Task::none()
            }
            Message::MaximizeLevel => {
                if let AppState::Editor { save, .. } = &mut self.state {
                    save.set_level(99);
                }
                Task::none()
            }
            Message::UpgradeVersion => {
                self.confirmation = Some(ConfirmationDialog {
                    title: "Upgrade character version?",
                    description: "Upgrading a character save is irreversible. Continue only if you have a backup.",
                    confirm_label: "Upgrade",
                    action: ConfirmationAction::UpgradeVersion,
                });
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
                if let AppState::Editor { save, .. } = &mut self.state
                    && diff < 3
                    && idx < WAYPOINT_COUNT
                {
                    save.waypoints[diff][idx] = !save.waypoints[diff][idx];
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
            Message::BrowseExportPath => {
                let default_path = if self.export_path_input.trim().is_empty() {
                    default_save_folder()
                } else {
                    PathBuf::from(self.export_path_input.trim())
                };
                let path = rfd::FileDialog::new()
                    .set_directory(default_path)
                    .pick_folder();
                Task::done(Message::ExportPathSelected(path))
            }
            Message::ExportPathSelected(Some(path)) => {
                self.export_path_input = path.to_string_lossy().into_owned();
                self.config.export_path = Some(path);
                let _ = confy::store("d2sed", None, &self.config);
                self.status_message = None;
                Task::none()
            }
            Message::ExportPathSelected(None) => Task::none(),
            Message::ExportPathChanged(path) => {
                self.export_path_input = path;
                self.status_message = None;
                Task::none()
            }
            Message::ToggleDead => {
                if let AppState::Editor { save, .. } = &mut self.state
                    && save.hardcore
                {
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
                let folder = self.export_path_input.trim().to_string();
                if folder.is_empty() {
                    self.status_message = Some("Set an export path before saving.".to_string());
                } else {
                    let mut path = PathBuf::from(&folder);
                    let save_result = if let AppState::Editor { save, .. } = &self.state {
                        path.push(format!("{}.d2s", save.name));
                        Some(save.save_to_file(&path))
                    } else {
                        None
                    };

                    if let Some(save_result) = save_result {
                        self.config.export_path = Some(PathBuf::from(&folder));
                        let _ = confy::store("d2sed", None, &self.config);

                        match save_result {
                            Ok(_) => {
                                self.status_message =
                                    Some(format!("Saved {}", path.to_string_lossy()));
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Failed to save: {e}"));
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ConfirmDialog => {
                if let Some(dialog) = self.confirmation.take() {
                    match dialog.action {
                        ConfirmationAction::BackToLaunch => {
                            self.status_message = None;
                            self.state = AppState::LaunchScreen;
                        }
                        ConfirmationAction::UpgradeVersion => {
                            self.status_message =
                                Some("Version upgrade is not implemented yet.".to_string());
                        }
                    }
                }
                Task::none()
            }
            Message::CancelDialog => {
                self.confirmation = None;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match &self.state {
            AppState::LaunchScreen => self.view_launch_screen(),
            AppState::Editor {
                save,
                left_tab,
                right_tab,
            } => self.view_editor(save, left_tab, right_tab),
        };

        if let Some(dialog) = &self.confirmation {
            self.view_with_confirmation(content, dialog)
        } else {
            content
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
            let mut btn = button(text(class.to_string())).padding(10);

            btn = btn.on_press(Message::TemplateSelected(class));

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
            text("d2sed").size(39),
            text("Diablo 2 Save Editor").size(19),
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
            column![
                text(self.status_message.as_deref().unwrap_or(""))
                    .size(11)
                    .style(|_: &iced::Theme| text::Style {
                        color: Some(iced::Color::from_rgb(0.95, 0.75, 0.25)),
                    }),
            ],
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
        let display_dead = save.is_dead_for_display();
        let mut death_button =
            button(text(if display_dead { "DEAD" } else { "Alive" })).style(if display_dead {
                button::danger
            } else {
                button::success
            });

        if save.hardcore {
            death_button = death_button.on_press(Message::ToggleDead);
        }

        let header_content = row![
            row![
                column![
                    text_input("Name", &save.name)
                        .on_input(Message::NameChanged)
                        .size(29)
                        .width(Length::Fixed(200.0)),
                    text(format!(
                        "Level {} {} ({})",
                        save.level, save.class, save.game_version
                    ))
                    .size(13),
                ]
                .spacing(2),
                column![
                    button("Upgrade")
                        .on_press(Message::UpgradeVersion)
                        .padding(5),
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(Alignment::End),
            Space::new().width(20),
            column![
                text(if save.hardcore {
                    "HARDCORE"
                } else {
                    "Softcore"
                })
                .size(17),
                death_button,
            ],
            Space::new().width(Length::Fill),
            column![
                text("Export Path:").size(11),
                row![
                    text_input("Folder path...", &self.export_path_input)
                        .on_input(Message::ExportPathChanged)
                        .padding(5)
                        .size(11)
                        .width(Length::Fixed(260.0)),
                    button("Browse")
                        .on_press(Message::BrowseExportPath)
                        .padding(5),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            ]
            .spacing(2),
            Space::new().width(20),
            button("Save .d2s")
                .on_press(Message::SaveCharacter)
                .padding(10)
                .style(button::primary),
            Space::new().width(20),
            button("Back").on_press(Message::BackToLaunch).padding(10),
        ]
        .align_y(Alignment::End)
        .padding(10);

        let status_line: Element<'_, Message> = if let Some(msg) = &self.status_message {
            row![
                Space::new().width(Length::Fill),
                text(msg).size(11).style(|_: &iced::Theme| text::Style {
                    color: Some(iced::Color::from_rgb(0.95, 0.75, 0.25)),
                }),
                Space::new().width(10),
            ]
            .into()
        } else {
            Space::new().height(0).into()
        };

        let header = container(column![header_content, status_line])
            .style(|_| container::Style {
                background: Some(iced::Color::from_rgb(0.1, 0.1, 0.1).into()),
                ..Default::default()
            })
            .width(Length::Fill);

        let left_tabs = row![
            button("Character")
                .on_press(Message::SetLeftTab(EditorLeftTab::Character))
                .style(if *left_tab == EditorLeftTab::Character {
                    button::primary
                } else {
                    button::secondary
                }),
            button("Advanced")
                .on_press(Message::SetLeftTab(EditorLeftTab::Advanced))
                .style(if *left_tab == EditorLeftTab::Advanced {
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
            EditorLeftTab::Character => {
                let stat_row = |name: String, value: u32, stat: CharacterStat| {
                    row![
                        text(name).width(Length::Fixed(100.0)),
                        button("Min")
                            .on_press(Message::MinimizeStat(stat))
                            .padding(5),
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
                        button("Max")
                            .on_press(Message::MaximizeStat(stat))
                            .padding(5),
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center)
                };

                let resistance_bonus = save.base_resistance_bonus();

                column![
                    row![
                        text("Level:").width(Length::Fixed(100.0)),
                        button("Min").on_press(Message::MinimizeLevel).padding(5),
                        button("-10")
                            .on_press(Message::DecreaseLevel(10))
                            .padding(5),
                        button("-").on_press(Message::DecreaseLevel(1)).padding(5),
                        text(save.level.to_string())
                            .width(Length::Fixed(40.0))
                            .align_x(Alignment::Center),
                        button("+").on_press(Message::IncreaseLevel(1)).padding(5),
                        button("+10")
                            .on_press(Message::IncreaseLevel(10))
                            .padding(5),
                        button("Max").on_press(Message::MaximizeLevel).padding(5),
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                    text(format!("Experience: {}", save.experience)),
                    Space::new().height(8),
                    text("Attributes").size(23),
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
                    Space::new().height(6),
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
                    Space::new().height(8),
                    text(format!("HP: {} / {}", save.current_hp, save.max_hp)),
                    text(format!("Mana: {} / {}", save.current_mana, save.max_mana)),
                    text(format!(
                        "Stamina: {} / {}",
                        save.current_stamina, save.max_stamina
                    )),
                    Space::new().height(6),
                    text("Base Resistances").size(19),
                    text(format!("Fire Resist: +{resistance_bonus}")),
                    text(format!("Cold Resist: +{resistance_bonus}")),
                    text(format!("Lightning Resist: +{resistance_bonus}")),
                    text(format!("Poison Resist: +{resistance_bonus}")),
                ]
                .spacing(8)
                .into()
            }
            EditorLeftTab::Advanced => column![
                text("Advanced").size(23),
                text("(Not implemented)").size(13),
                Space::new().height(8),
                text("Accumulated Item Bonuses").size(19),
                text("Resistances, skills, attributes, life, mana, speed, and other item-derived totals will appear here.").size(13),
                Space::new().height(8),
                text("Melee").size(19),
                text("Critical Hit: Not implemented"),
                text("Deadly Strike: Not implemented"),
                text("Crushing Blow: Not implemented"),
                text("+ Damage: Not implemented"),
                text("+ Damage %: Not implemented"),
            ]
            .spacing(8)
            .into(),
            EditorLeftTab::Stash => column![
                text("(Not implemented)").size(13),
                row![
                    text("Gold:").size(17),
                    text_input("0", &save.stashed_gold.to_string())
                        .on_input(|s| Message::StashGoldChanged(s.parse().unwrap_or(0)))
                        .padding(5)
                        .width(Length::Fixed(150.0)),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                self.draw_grid(10, 10, 35.0, "Stash".to_string()),
            ]
            .spacing(8)
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
                            text("Skill Trees").size(23),
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

                let mut skill_trees_row = row![].spacing(10).width(Length::Fixed(SKILL_PANE_WIDTH));
                for category in skills::skill_categories(save.class) {
                    let mut tree_col = column![text(category.name).size(14)].spacing(5);
                    for &slot in category.slots {
                        let name = skills::skill_name(save.class, slot);
                        let value = save.skills[slot];

                        let can_increase = save.can_increase_skill(slot);
                        let mut plus_btn = button("+").padding(2);
                        if can_increase {
                            plus_btn = plus_btn.on_press(Message::IncreaseSkill(slot));
                        }

                        let mut minus_btn = button("-").padding(2);
                        if save.can_decrease_skill(slot) {
                            minus_btn = minus_btn.on_press(Message::DecreaseSkill(slot));
                        }

                        let skill_row = row![
                            text(name).size(11).width(Length::Fixed(114.0)),
                            minus_btn,
                            text(value.to_string())
                                .size(11)
                                .width(Length::Fixed(22.0))
                                .align_x(Alignment::Center),
                            plus_btn,
                        ]
                        .spacing(3)
                        .align_y(Alignment::Center);

                        tree_col = tree_col.push(skill_row);
                    }
                    skill_trees_row = skill_trees_row.push(
                        container(tree_col)
                            .padding(6)
                            .width(Length::Fixed(200.0))
                            .style(|_| container::Style {
                                background: Some(iced::Color::from_rgb(0.08, 0.08, 0.08).into()),
                                border: iced::Border {
                                    color: iced::Color::from_rgb(0.3, 0.3, 0.3),
                                    width: 1.0,
                                    radius: iced::border::Radius::default(),
                                },
                                ..Default::default()
                            }),
                    );
                }
                skills_col = skills_col.push(skill_trees_row);
                skills_col.into()
            }
            EditorRightTab::Quests => {
                let difficulties = ["Normal", "Nightmare", "Hell"];

                let all_completed = difficulties.iter().enumerate().all(|(d, _)| {
                    VISIBLE_QUEST_INDICES
                        .iter()
                        .all(|&q| quest::quest_is_completed(save.quests[d][q]))
                });

                let quest_header = row![
                    text("Quest Log").size(23),
                    Space::new().width(40),
                    checkbox(all_completed)
                        .label("Complete All Difficulties")
                        .on_toggle(move |state| Message::ToggleAllQuests(None, state)),
                ]
                .spacing(10)
                .align_y(Alignment::Center);

                let mut diff_row = row![].spacing(20);

                for (diff_idx, diff_name) in difficulties.iter().enumerate() {
                    let diff_all_done = VISIBLE_QUEST_INDICES
                        .iter()
                        .all(|&q| quest::quest_is_completed(save.quests[diff_idx][q]));

                    let mut diff_col = column![
                        text(*diff_name).size(19),
                        checkbox(diff_all_done)
                            .label("All")
                            .on_toggle(move |state| Message::ToggleAllQuests(
                                Some(diff_idx),
                                state
                            )),
                    ]
                    .spacing(15);

                    for (act_idx, act) in VISIBLE_QUEST_ACTS.iter().enumerate() {
                        let mut act_col = column![text(act.name).size(15)].spacing(5);
                        let mut quest_grid = column![].spacing(2);

                        for (chunk_idx, chunk) in act.quest_indices.chunks(3).enumerate() {
                            let mut q_row = row![].spacing(5);
                            for (q_pos, &q_idx) in chunk.iter().enumerate() {
                                let is_completed =
                                    quest::quest_is_completed(save.quests[diff_idx][q_idx]);
                                let q_name = quest::quest_name(q_idx);

                                let quest_position = chunk_idx * 3 + q_pos + 1;
                                let btn =
                                    button(text(format!("A{}Q{}", act_idx + 1, quest_position)))
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
                let difficulties = ["Normal", "Nightmare", "Hell"];

                let all_wps_done =
                    (0..3).all(|d| (0..WAYPOINT_COUNT).all(|w| save.waypoints[d][w]));

                let wp_header = row![
                    text("Waypoints").size(23),
                    Space::new().width(40),
                    checkbox(all_wps_done)
                        .label("Unlock All Diffs")
                        .on_toggle(|state| Message::ToggleAllWaypoints(None, state)),
                ]
                .align_y(Alignment::Center);

                let mut diff_row = row![].spacing(20);
                for (diff_idx, diff_name) in difficulties.iter().enumerate() {
                    let diff_all_wps = (0..WAYPOINT_COUNT).all(|w| save.waypoints[diff_idx][w]);

                    let mut diff_col = column![
                        text(*diff_name).size(19),
                        checkbox(diff_all_wps).label("All").on_toggle(move |state| {
                            Message::ToggleAllWaypoints(Some(diff_idx), state)
                        }),
                    ]
                    .spacing(10);

                    for act in WAYPOINT_ACTS {
                        let mut act_col = column![text(act.name).size(15)].spacing(2);
                        for wp_idx in act.indices() {
                            let is_active = save.waypoints[diff_idx][wp_idx];
                            act_col = act_col.push(
                                button(text(WAYPOINT_NAMES[wp_idx]).size(11))
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
                    container(text(label).size(9))
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
                    text("(Not implemented)").size(13),
                    row![
                        text("Gold:").size(17),
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
        let mut grid_col = column![text(title).size(19), Space::new().height(8)].spacing(5);
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

    fn view_with_confirmation<'a>(
        &'a self,
        content: Element<'a, Message>,
        dialog: &'a ConfirmationDialog,
    ) -> Element<'a, Message> {
        let scrim = opaque(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: Some(
                        iced::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.55,
                        }
                        .into(),
                    ),
                    ..Default::default()
                }),
        );

        let dialog_body = container(
            column![
                text(dialog.title).size(20),
                text(dialog.description).size(13),
                Space::new().height(8),
                row![
                    Space::new().width(Length::Fill),
                    button("Cancel")
                        .on_press(Message::CancelDialog)
                        .padding([8, 14]),
                    button(dialog.confirm_label)
                        .on_press(Message::ConfirmDialog)
                        .padding([8, 14])
                        .style(button::danger),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(10),
        )
        .width(Length::Fixed(420.0))
        .padding(18)
        .style(|_| container::Style {
            background: Some(iced::Color::from_rgb(0.12, 0.12, 0.12).into()),
            border: iced::Border {
                color: iced::Color::from_rgb(0.42, 0.32, 0.18),
                width: 1.0,
                radius: iced::border::Radius::from(6.0),
            },
            ..Default::default()
        });

        let dialog_layer = opaque(
            container(dialog_body)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        );

        stack(vec![content, scrim, dialog_layer])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
