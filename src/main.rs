use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length, Task};
use libd2::core::character_class::CharacterClass;
use std::path::PathBuf;

pub fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view)
        .title("d2sed - Diablo 2 Save Editor")
        .run()
}

enum AppState {
    LaunchScreen,
    Editor,
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
                // We use standard blocking FileDialog for simplicity.
                // It blocks the UI thread while open, which is standard for file pickers.
                let path = rfd::FileDialog::new()
                    .add_filter("Diablo 2 Save", &["d2s"])
                    .set_directory(default_path)
                    .pick_file();

                // Return a task that resolves immediately with the selected file
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
                // Here we would either load the file or generate the template
                // For now, just transition to the editor state
                if !self.file_path.is_empty() || self.selected_template.is_some() {
                    self.state = AppState::Editor;
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        match self.state {
            AppState::LaunchScreen => self.view_launch_screen(),
            AppState::Editor => self.view_editor(),
        }
    }

    fn view_launch_screen(&self) -> Element<Message> {
        // File path input and browse button
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

        // Template character selection
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
                // simple visual feedback for selection
                btn = btn.style(button::success);
            }

            class_row = class_row.push(btn);
        }

        // Load button
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

    fn view_editor(&self) -> Element<Message> {
        let content = column![
            text("Editor Mode (Under Construction)").size(30),
            button("Back to Launch Screen").on_press(Message::LoadCharacter), // Temp reset
        ]
        .spacing(20)
        .align_x(Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}
