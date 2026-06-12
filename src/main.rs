use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length, Task};
use libd2::core::character_class::CharacterClass;
use std::path::PathBuf;

mod model;
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
                    // TODO: Actually load and parse the file here
                    // For now, generate a fallback template
                    self.state = AppState::Editor(Savegame::generate_template(CharacterClass::Amazon));
                }
                Task::none()
            }
            Message::BackToLaunch => {
                self.state = AppState::LaunchScreen;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        match &self.state {
            AppState::LaunchScreen => self.view_launch_screen(),
            AppState::Editor(save) => self.view_editor(save),
        }
    }

    fn view_launch_screen(&self) -> Element<Message> {
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
            load_btn = load_btn.on_press(Message::LoadCharacter).style(button::primary);
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

    fn view_editor(&self, save: &Savegame) -> Element<Message> {
        let top_pane = row![
            button("Save .d2s").padding(10), // TODO
            button("Back to Launch").on_press(Message::BackToLaunch).padding(10),
            Space::new().width(Length::Fill),
            text(format!("Loaded: {} (Level {} {})", save.name, save.level, save.class)).size(20)
        ].spacing(10).align_y(Alignment::Center).padding(10);

        let left_pane = column![
            text("Character Overview").size(24),
            text(format!("Strength: {}", save.strength)),
            text(format!("Dexterity: {}", save.dexterity)),
            text(format!("Vitality: {}", save.vitality)),
            text(format!("Energy: {}", save.energy)),
            text(format!("Stat Points Remaining: {}", save.stat_points_remaining)),
            Space::new().height(20),
            text("Detailed Stats").size(24),
            text("Stash").size(24),
        ].spacing(10).padding(10).width(Length::FillPortion(1));

        let right_pane = column![
            text("Skills").size(24),
            text(format!("Skill Points Remaining: {}", save.skill_points_remaining)),
            Space::new().height(20),
            text("Quests").size(24),
            Space::new().height(20),
            text("Inventory").size(24),
        ].spacing(10).padding(10).width(Length::FillPortion(2));

        let center_content = row![left_pane, right_pane].spacing(20);

        let bottom_pane = container(
            text("Log: Initialized editor.").size(14)
        ).padding(10).width(Length::Fill);

        let main_layout = column![
            top_pane,
            center_content.height(Length::Fill),
            bottom_pane,
        ];

        container(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
