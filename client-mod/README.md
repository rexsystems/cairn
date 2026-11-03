# Cairn Client

Cairn's own client mod, controlled by one toggle in the launcher's Options. Game adapters share the same module settings, HUD editor and menu.

| Minecraft | Loader | Artifact |
| --- | --- | --- |
| 1.8 | Forge 11.14.4.1577 | `cairn-client-forge-1.8.jar` |
| 1.8.8 | Forge 11.15.0.1655 | `cairn-client-forge-1.8.8.jar` |
| 1.8.9 | Forge 11.15.1.2318 | `cairn-client-forge-1.8.9.jar` |
| 1.16.5 | Fabric Loader 0.14.21+ | `cairn-client-fabric-1.16.5.jar` |
| 1.20.1 | Fabric Loader 0.14.21+ | `cairn-client-fabric-1.20.1.jar` |
| 1.20.4 | Fabric Loader 0.15.0+ | `cairn-client-fabric-1.20.4.jar` |
| 1.21.1 | Fabric Loader 0.15.11+ | `cairn-client-fabric-1.21.1.jar` |
| 1.21.11 | Fabric Loader 0.17.0+ | `cairn-client-fabric-1.21.11.jar` |
| 26.3 | Fabric Loader 0.20.1+ | `cairn-client-fabric-26.3.jar` |

The launcher picks the jar matching the instance's loader and game version and reads the loader floor from that jar's `fabric.mod.json`. An instance pinned to an older loader is refused before launch; an unpinned instance whose cached Fabric profile is too old gets the current loader reinstalled. Support is limited to the exact versions above. Further versions need their own compiled and tested adapters. These modules are an initial Cairn implementation, not full Lunar feature parity.

## Controls

Enable **Options > Cairn Client**, save, then launch. **Right Shift** opens the module panel, in game or from the title screen. Key presses are handled as input events, so a short tap always registers. On Fabric the menu and zoom keys can be rebound from the panel header and the Zoom settings; Forge uses Minecraft's Controls screen. Use search or category filters to find a module. Its ON/OFF button controls activation; Settings opens its inspector. Tab moves between controls, arrows select modules, Enter/Space activates a control, and Ctrl+F focuses search. Minimal, Combat and Explore presets select real module combinations.

**E** or **Edit HUD** opens the HUD editor. Drag each widget to move it, or drag its bottom-right corner to resize it. Selected widgets have an outline and resize handle. Tab selects the next enabled widget, arrows nudge it, and +/- adjusts scale. **G** toggles four-pixel snapping. **H** hides the toolbars so every part of the screen is available for dragging. Enter opens the selected widget's settings; Escape returns to modules. Positions are clamped to the viewport. Reset restores one widget's settings and position.

New widgets flow down the left edge in columns without overlapping, and older saved layouts that stacked widgets are migrated. Text widgets fit their content by default (Auto width); turning it off uses the fixed Width setting. Each text widget has independent scale, width, text/background/accent RGB colors, background opacity, background visibility, accent edge, shadow and alignment. The color field accepts any six-digit RGB hex value. Click it and type to replace the current value.

Keystrokes draws W/A/S/D, left/right mouse buttons and Space as individual keys. Key size, spacing, widget scale, released background, pressed color, text color, mouse buttons, Space and CPS labels are configurable. Pressed labels automatically choose black or white for contrast. The mod uses Minecraft's normal font, as requested; no custom font or resource-loader dependency is bundled.

## Modules

Twenty modules: FPS, coordinates, ping, keystrokes, left/right CPS, armor durability, potion effects, local clock, movement speed, toggle sprint, zoom, crosshair, direction, biome, held item, hunger/saturation, world time, session duration, memory and server address.

Zoom has adjustable strength and optional smoothing. While zooming, the mouse wheel changes the zoom level instead of the hotbar slot, and the cinematic camera can be applied for the duration of the zoom; the previous camera setting is restored on release. Toggle sprint has auto-sprint and reset-on-stop options while respecting food, sneak, item use and collision checks. Clock supports seconds and 12/24-hour formats. Crosshair has configurable size, gap, thickness, center dot and color; it can be positioned and scaled in the editor. It replaces Minecraft's crosshair by default, with an option to retain the original. Armor, held item and effects currently use compact text displays.

Choices persist in `config/cairn-client.properties` inside the instance. Existing settings survive upgrades, and invalid numeric settings are bounded.

## Instance behavior

The launcher installs only `mods/cairn-client-managed.jar`. Turning Cairn off removes this managed artifact on the next Java launch and passes `-Dcairn.client.enabled=false`. Other mods, their enabled states, worlds and Cairn settings remain. A manually installed Cairn jar is reused and stays on disk; the same JVM flag disables its features. A foreign file using the reserved filename is never overwritten or deleted. An instance keeps its existing loader.

## Build and validation

`npm run build:client` rebuilds all six artifacts under `src-tauri/client-mods`, then runs shared Java tests. Bundled jars are included in source so building the launcher does not require Java tools or mod downloads.

Requirements: Node and curl; JDK 25 for 26.3; Gradle 8.14 running on JDK 21 for mapped Fabric and legacy Forge; an installed JDK 8 for Forge's legacy tooling. Set `CAIRN_MODERN_JAVA_HOME`, `CAIRN_LEGACY_JAVA_HOME`, `CAIRN_JAVA_INSTALLATIONS` (comma-separated JDK directories) and `CAIRN_GRADLE` when tools are not the system defaults. Output bytecode targets Java 8 for Forge and 1.16.5, Java 17 for 1.20.1/1.20.4, Java 21 for 1.21.1/1.21.11 and Java 25 for 26.3. Minecraft is not included in output jars.

Build one family with `node client-mod/scripts/build.mjs forge`, `mapped` or `modern`. Minecraft compile artifacts use Mojang SHA-1 checks; pinned Fabric Loader and Mixin compile dependencies for 26.3 use SHA-256 checks. Mapped targets use Unimined's dependency resolution and mixin remapping.

`npm run test:client` checks configuration, search, controls, arbitrary colors, per-widget size, presets, snapping, resize handles, cached HUD measurement and settings pages at twelve viewport sizes. `npm test` covers the Options toggle; `npm run test:rust` covers managed installation and preservation of user files. To run the headless core CPU benchmark after compiling tests: `java -cp client-mod/common/build/test-classes app.cairn.client.HudBenchmark`. This excludes Minecraft, font drawing and GPU work and does not measure in-game FPS.

## Rendering decisions

ENERGY 2 / RHYTHM 1 / MOTION 1. The existing moss accent identifies enabled modules, selection and HUD handles. Search and category filters reduce scanning; the inspector holds per-module controls separately from activation. Two columns collapse to one in narrower windows. Solid surfaces keep labels readable over gameplay. Minecraft's normal font follows the user's preference. Widths are measured by the active font and text is clipped to its container. No decorative motion competes with gameplay.

HUD state is parsed once when the mod starts. Text layout is cached until content, width or scale changes. Click counters refresh at 100 ms; clocks and memory at one second; game statistics at five ticks. Disabled statistics skip formatting. Configuration writes happen on control changes and drag release, never during HUD rendering. Text component caches are bounded. Keystrokes input and zoom stay responsive independently of statistics formatting.

## Adapter boundaries

`common` owns settings, module state, search, menu, HUD rendering and editing. `forge` uses legacy events and MCP/SRG remapping through Unimined. `fabric-mapped` targets the obfuscated 1.16.5, 1.20.1, 1.20.4, 1.21.1 and 1.21.11 APIs through three adapter families (`src/v1_16`, `src/v1_20`, `src/v1_21_9`), per-version HUD mixins where the `Gui` signature differs, and intermediary remapping. `modern` targets the unobfuscated 26.3 API. Compatibility is not inferred merely because another version number is newer.
