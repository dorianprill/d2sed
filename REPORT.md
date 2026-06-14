# d2sed Project History and Future Features

## Project History

- **2026-06-12**: Initialized the project with Rust Edition 2024, `iced` 0.14, and `libd2`. Setup the basic repository, Cargo configuration, and architecture guidelines. The architecture emphasizes the Elm-style Model-View-Update pattern and strictly separates the UI logic from the `libd2` domain logic. Added `rfd` for native cross-platform file dialogs.
- **2026-06-13**: Replaced the generated experience curve with the canonical Lord of Destruction cumulative XP table through level 99. Added class skill tree metadata, grouped skills by their in-game categories, and made skill edits enforce prerequisite chains in the savegame model.
- **2026-06-14**: Moved reusable Diablo II rule data and pure rule logic into `libd2`: XP breakpoints, class base stats/resource growth, skill metadata/prerequisites, quest word semantics/rewards/progression, quest/waypoint section parse/write helpers, gold caps, and waypoint metadata. `d2sed` now consumes those APIs and keeps editor-specific state mutation, UI flow, template scaffolding, and safety prompts locally.

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
- **Corpse State**: Parse and preserve the softcore corpse item section so the editor can distinguish "has died before" from "currently has a corpse in game". Until item/corpse support exists, the death toggle is only editable for hardcore characters.
- **Detailed Quest Progress Editing**: Model each quest as metadata over the raw 16-bit quest word: quest name, display flags, completion/reward flags, and special reward bits such as the Prison of Ice resistance scroll. A right-side inspector can then expose known flags with safe labels while preserving unknown bits until each quest is fully decoded.

### Unresolved / Technical Challenges
- *None yet.*
