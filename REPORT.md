# d2sed Project History and Future Features

## Project History

- **2026-06-12**: Initialized the project with Rust Edition 2024, `iced` 0.14, and `libd2`. Setup the basic repository, Cargo configuration, and architecture guidelines. The architecture emphasizes the Elm-style Model-View-Update pattern and strictly separates the UI logic from the `libd2` domain logic. Added `rfd` for native cross-platform file dialogs.

## Roadmap & Future Features

### High Priority
- **Launch Screen**: Implement a file picker and new character template selection for the 7 classes.
- **Character Overview**: View and edit basic character info (Name, Class, Level, Experience).
- **Attributes & Stats**: Enforce calculation rules `Total = BaseTotal + (Level * 5) + (5 * HasStatPointsQuest) + EquipmentBonuses`. 
- **Skills**: Display skill trees, allow leveling up/down of skills, enforcing a hard cap of 20 points per skill.
- **Quests**: Manage quest completion states (Normal, Nightmare, Hell). Ensure stats/skills sync correctly when quests are unchecked.
- **Inventory/Stash**: Visual representation of character inventory and stash sizes, dynamically scaling depending on the game version (Classic/LoD vs Resurrected/Reign of the Warlock).

### Medium Priority
- **Visuals**: Extract and use original Diablo 2 assets for UI elements (icons, buttons) using `libd2` MPQ readers.
- **Mercenary Support**: View and edit mercenary equipment and attributes.
- **Item Editing**: Modify item stats, add/remove sockets, change runewords.

### Unresolved / Technical Challenges
- *None yet.*
