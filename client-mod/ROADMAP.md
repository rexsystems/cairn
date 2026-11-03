# Requested client modules

The November 2026 request supplies a Feather 1.21.8 configuration inventory as a behavior reference. It is a backlog, not a claim of implemented compatibility or feature parity. Cairn keeps its own implementation and assets. Unknown defaults in that inventory must be chosen explicitly and documented rather than presented as extracted values.

Current work comes first: title menu, Right Shift HUD editor, module settings with live previews, plain HUD defaults, armor/effect icons, in-game cosmetic selection and administrator grants tied to verified identities. See VALIDATION.md for actual coverage.

## Configuration requirements

Every HUD must expose independent text/background colors, background opacity, background visibility, padding, width/height, scale and alignment where applicable. Colors accept custom RGB HEX. Previews must reflect changes immediately. Module-specific options must change the real behavior and have regression tests. A catalog entry or toggle without an implementation is not a finished module.

## Requested module groups

| Group | Requested modules and behavior |
| --- | --- |
| Existing HUD expansion | FPS, Ping, Coordinates, CPS, Direction, Playtime, Potion Effects, Time, Server Address, System Resources: formatting, visibility, per-field colors, timing and layout options from the supplied inventory |
| Equipment and inventory | Armor Bar, Armor Status, Attack Indicator, Drop Prevention, Elytras, Hearts, Inventory, Item Despawn, Item Info, Item Physics, Pack Display, Pack Organizer, Saturation, Shield Status, Totem, Tooltips, UHC Overlay |
| Camera and visuals | Animations, Auto Perspective, Block Overlay, Brightness, Camera, Color Saturation, Custom Crosshair, Dark Mode, FOV Changer, Hitbox, Hit Indicator, Light Level Overlay, Loot Beams, Mob Overlay, Motion Blur, Mousestrokes, Nametags, Nick Hider, Perspective, Snaplook, Time Changer, Title Tweaker, ViewModel, Weather Changer, Particles and Visuals |
| Additional HUD | Autohide HUD, Block Indicator, Boss Bar, Combo Display, Death Info, Horses, Item Info, Jump Reset, Reach Display, Scoreboard, Subtitles, TNT Timer, TPS |
| Chat, input and utilities | Auto Text, Backups, Cull Logs, Custom Advancements Screen, Custom Chat, Custom F3, Discord, Hypixel utilities, Reconnect, Screenshot, Keybind Search, Tablist, Tier Tagger, Toast Control, Toggle Sprint, UI Scaling |
| Social and world | Voice with device/PTT controls, party/proximity modes and volume limits; Waypoints with names/beams/death markers; Team Tracker; party sharing remains part of the original handoff |
| Client settings | Server search/folders/favorites, world favorites/trash, preview backgrounds, window/fullscreen controls, idle/background FPS limits, chat history/copy/mentions, sound filters, graphics and performance options, cosmetic visibility/physics |

Implement and validate each group across the supported versions, accounting for features that do not exist in older Minecraft versions. Preserve custom configurations through migrations. Keep server-dependent controls honest when no server is configured, authentication is unavailable or the backend is down.

The earlier handoff also requests emotes, parties, minimap, launcher 3D cosmetic preview, Tab badges, a protected admin page, seasonal grants and optional compatible performance mods. Those remain open until implemented and tested; this inventory does not mark them complete.
