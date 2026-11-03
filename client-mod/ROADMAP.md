# Requested client modules

This is Cairn's own module backlog from the November 2026 request. It is not a claim that every entry is finished on every supported version. Unknown defaults must be chosen explicitly and documented. A catalog entry or toggle without an implementation is not a finished module.

## Status

**In the client Module catalog now (70):** FPS, Coordinates, Ping, Keystrokes, CPS, Armor, Effects, Clock, Speed, Toggle sprint, Zoom, Crosshair, Direction, Biome, Held item, Food, World time, Session, Memory, Server, Block indicator, Reach, TPS, Combo, Death info, Horses, Boss bar, Hearts, Armor points, Attack indicator, Elytra, Shield, Totem, Light level, Scoreboard, TNT timer, Item info, Item despawn, Pack display, Inventory, System resources, Saturation, UHC overlay, Team tracker, Block overlay, Brightness, FOV changer, Hitboxes, Hit indicator, Time changer, Weather changer, Nick hider, Drop prevention, Autohide HUD, Perspective, Snaplook, Auto perspective, ViewModel, Particles, Nametags, Reconnect, Toast control, UI scaling, Cull logs, Custom F3, Chat timestamps, Waypoints, Mob overlay, Loot beams, Title tweaker.

**Still open (need dedicated systems or launcher work):** Item Physics, Pack Organizer, Tooltips rewrite, Animations, Camera presets, Color Saturation, Motion Blur, Light Level world overlay (beyond the HUD number), Subtitles HUD, Jump Reset combat helper, Auto Text, Backups, Custom Advancements screen, full Custom Chat, Discord presence module, Hypixel utilities, Screenshot helper module, Keybind Search UI, Tablist cosmetics column, Tier Tagger, Voice (devices/PTT/proximity), full Waypoints editor with beams/sharing, Party system, Admin web UI, seasonal grants, optional performance mod packs.

Current work still comes first for polish: title menu, Right Shift HUD editor, live settings previews, armor/effect icons, in-game cosmetics and verified staff grants. See VALIDATION.md for actual coverage.

## Configuration requirements

Every HUD must expose independent text/background colors, background opacity, background visibility, padding, width/height, scale and alignment where applicable. Colors accept custom RGB HEX. Previews must reflect changes immediately. Module-specific options must change the real behavior and have regression tests.

Implement and validate each remaining group across the supported versions, accounting for features that do not exist in older Minecraft versions. Preserve custom configurations through migrations. Keep server-dependent controls honest when no server is configured, authentication is unavailable or the backend is down.
