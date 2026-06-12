# d2sed Agent Instructions

You are an expert Rust developer specializing in GUI applications and reverse engineering. You are helping build **d2sed**, a modern Diablo 2 savegame editor covering the full range of `.d2s` file modifications on several game versions (`Classic`, `Lord of Destruction`, `Resurrected`, `Reign of the Warlock`) and patches (1.04 through 1.14d, and for `Resurrected` and `Reign of the Warlock` often called 1.15 but there may be different versions so please check that in the sources).

## Project Vision

A simple and fast, reliable, and user-friendly GUI for editing `.d2s` files, leveraging `libd2`.  
The application should be cross-platform (Windows, macOS, Linux) and provide a modern user experience while ensuring data integrity and safety.
The idea is to allow user to easily test out "legit" (as in: could be achieved through playing the game) character builds because the editor automatically applies sanity checks and prevents users from creating save files that would be considered "corrupted" or "hacked" by the game.

Examples:

- Players can increase their character's level, but the editor will automatically adjust stats and skill points according to the game's rules.
- Players can reset stat/skill points to reallocate them either in the editor or in game, but the editor will ensure that the total points do not exceed what is allowed for the character's level and quests completed.
- Players can edit quest completion status, but the editor will ensure that any changes are consistent with the character's progress and the game's mechanics. i.e. unchecking a skill quest will remove the skill points granted by that quest (if all are spent, generate a message saying cannot deduct skill points, all are spent, please reset skill points first and then uncheck the quest) and unchecking a stat quest will remove the stat points granted by that quest (if all are spent, generate a message saying cannot deduct stat points, all are spent, please reset stat points first and then uncheck the quest).
- Each stat/skill can be increased by left-clicking on the skill icon, and decreased by right-clicking on the skill icon. The editor will ensure that the total skill points spent do not exceed what is allowed for the character's level and quests completed, and that no single skill exceeds the maximum allowed points (20 before item bonuses).
- All remaining stat/skill points can be spent by shift-left-clicking on the skill or stat button, and all points can be removed by shift-right-clicking on the skill or stat button. The editor will ensure that the total points spent do not exceed what is allowed for the character's level and quests completed, and that no single skill exceeds the maximum allowed points (20 before item bonuses). Also, stats cannot be removed beyond each character classes base points (i.e. a sorceress cannot have less than 15 strength, 15 dexterity, 10 vitality and 10 energy) and skills cannot be removed beyond level 0.
- Total stats must always satisfy the formula `Total = BaseTotal + (Level * 5) + (5 * HasStatPointsQuest[norm,nm,hell]) + (EquipmentBonuses)`.
- Total skill levels must satisfy the formula `Total = BaseTotal + (Level * 1) + (SkillPointsQuests[norm, nm, hell]) + (EquipmentBonuses)`.
- Maximum spent hard skill points per skill cannot exceed 20 (before item bonuses)
- Life should be increased by flat 20 points for the act 3 quest "The Golden Bird" from Alkor for each difficulty setting

https://www.d2tomb.com/ has detailed quest information if you are not sure on any of these restrictions.

## GUI Description

When starting the editor, there is a single screen with a text input box for the file path and a button beside it that launches a file picker to select a `.d2s` file. Loading a d2s file will infer the game Version and patch from the file and adapt the editor accordingly (or ask if there are multiple possibilities).
Below that, are state-buttons to choose from a new level 99 (all quests and bonuses, all waypoints) template character (goes green when selected) for each class, then below that a final button with the text "Load Character" that becomes enabled once a valid file path is entered or a template character is selected. The user can also press `Enter` to load the character after entering the file path or selecting a template character.


Once a file is loaded, the main editor screen is displayed, showing the character's information and allowing the user to edit various aspects of the character.

The GUI should be organized into several sections:

- A left pane with a menu for character overview (name, class, level, experience, etc.), a menu for detailed stats (critical strike crushing blow etc from items bonuses), and another menu for stash (like in the game).
- A right pane with a tab for Skills (with 3 tabs each in itself), another menu for quests with 3 tabs each for normal, nightmare and hell difficulty, and another menu for Inventory (equipped and bags).
- A top pane with buttons for loading and saving `.d2s` files, and a status bar for showing messages and errors.
- A bottom pane with a log of recent actions and changes made to the character.
- All opens should be openable at the same time, so the user can for example have the inventory and stash open at the same time to easily move items between them, or have the character stats overview and skill tree open at the same time to easily see how changes in one affect the other.
- The menus should open with the hotkeys like in the game:
  - `C` for character overview
  - `D` for detailed stats (like what is on second page char stats in d2r)
  - `T` for skills
  - `Q` for quests
  - `I` for inventory
  - `S` for stash (not in the game but works well)

Inventory and stash sizes need to adapt to the game version and patch of the loaded character. For example, in `Classic` and `Lord of Destruction`, the inventory has 4 rows and 10 columns, while in `Resurrected` and `Reign of the Warlock`, the inventory has 4 rows and 12 columns. The stash also varies in size depending on the game version and patch. In Reign of the Warlock, the stash has tabs and stackable runes, gems and rejuv potions (small/large) so the stash UI needs to adapt to that as well.
You need to think about this for the design, but i want to only start with support for three/four modes: Diablo II Classic and Lord of Destruction patch 1.14d and Resurrected and Reign of the Warlock.
Older game version that are not live on servers anymore are more niche and can be added later if there is demand for them, but the editor should be designed in a way that allows for easy addition of support for older versions in the future.

You don't need to use original d2 assets/textures, but it would be really cool if you could use them for the UI icons and buttons. Maybe the libd2 mpq reader can extract the necessary assets from the game files and convert them to a format that can be used in the editor (png).

It should be a simpler version of the well-loved Diablo 2 Hero Editor (thats what i am going for with the visuals).

If you need any design directions during your development, do not hesitate to ask for them, but try to keep the design simple and clean, and focus on functionality and reliability first. The UI can be improved iteratively over time.

## Technical Standards

create a github repository at github.com/dorianprill/d2sed and push your code there. Make sure to write clear commit messages and use branches for new features or bug fixes.

### Architecture

- **State-Driven UI:** Use the `iced` Elm-style architecture (Model-View-Update).
- **Domain Separation:** Keep UI logic (`iced` widgets/styling) strictly separated from savegame manipulation logic.
- **Error Handling:** Use `anyhow` for application-level errors and `thiserror` for library-level domain errors. Never use `unwrap()` or `expect()` on user-provided data.
- **Concurrency:** Use standard OS threads (`std::thread`) for background file I/O or long-running tasks to keep the UI responsive. Avoid complex async runtimes unless necessary.

### Code Style

- **Edition 2024:** Utilize modern Rust features (e.g., improved `impl Trait`, async traits).
- **Safety:** Minimize `unsafe` code. Any necessary `unsafe` blocks must be documented with a safety justification.
- **Documentation:** Provide doc comments for all public structs, enums, and methods.
- **Types:** Leverage Rust's type system to make invalid save states unrepresentable.

## Development Workflow

1. **Research:** Before implementing a new feature, verify if `libd2` already supports the underlying `.d2s` format changes.
2. **Implementation:**
   - Define the `Model` (application state).
   - Define the `Message` (user actions).
   - Implement `update()` to handle messages.
   - Implement `view()` to render the UI.
3. **Verification:** Add extensive unit tests for State, UI and savegame logic and write integration tests for each class template (e.g. generate->save->load->edit->save->load->undoedits->save->comparetotemplate).

If you are missing features in libd2 that are necessary for the editor, you can implement them in the editor codebase and then contribute them back to libd2 (but only via pull requests with extensive testing) if they are generally useful for other applications as well. example: character class base stats would be useful to have in libd2 as well, but the editor-specific sanity checks for editing characters would not be useful for that library.

Write and maintain a REPORT.md that is intended as a history of the project (e.g. things that didn work out because of something can be documented here, as this will not be reflected in git history).
Here you can also earmark future features and their requisites. This is intended to be a resource for you and for future contributors to understand the design decisions and trade-offs made during development.

## UI/UX Guidelines

- **Modern Aesthetics:** Use `iced`'s styling capabilities for a clean, non-native look.
- **Safety First:** Always backup the original `.d2s` file before overwriting.
- **Feedback:** Show clear success/error messages for file operations.
- **Progress:** Use progress bars or spinners for tasks taking longer than 100ms.
