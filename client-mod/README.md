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

Enable **Options > Cairn Client**, save, then launch. **Right Shift** opens the HUD editor, in game or from the title screen. **Cairn Settings** opens the module gallery; **Cosmetics** and **Friends** open their pages directly. Each widget has settings and hide controls beside it. Each gallery card has a large **Enabled/Disabled** switch and a settings cog; clicking its title or preview also opens settings. **Modules**, **Friends** and **Cosmetics** stay in the same navigation row. **X** closes the menu. Search and category filters narrow the grid. On Fabric the menu and zoom keys can be rebound from the header and Zoom settings; Forge uses Minecraft's Controls screen. Tab moves between controls, arrows select modules, Enter/Space activates a control, and Ctrl+F focuses search. A rebound menu key does not interrupt text entry. Minimal, Combat and Explore presets select real module combinations.

The Minecraft title screen uses Cairn controls over the game's own panorama. Singleplayer, Multiplayer, Minecraft settings and Quit retain vanilla actions and restrictions. Cairn Settings, Cosmetics and Friends are available directly. Tab/arrows and Enter/Space operate the title controls.

Press **Right Shift** again to close the interface directly from the editor, settings or a color field. **Escape** navigates back within the menu. Letter keys rebound as the menu key still work as text while a field has focus; Tab remains field navigation.

**E** or **Edit HUD** opens the HUD editor. Drag each widget to move it, or drag its bottom-right corner to resize it. Selected widgets have an outline and resize handle. Tab selects the next enabled widget, arrows nudge it, and +/- adjusts scale. **G** toggles four-pixel snapping. **H** hides the toolbars so every part of the screen is available for dragging. Enter opens the selected widget's settings; Escape returns to modules. Positions are clamped to the viewport. Reset restores one widget's settings and position.

New widgets flow down the left edge in columns without overlapping, and older saved layouts that stacked widgets are migrated. Text widgets fit their content by default (Auto width); turning it off uses the fixed Width setting. Each text widget has independent scale, width, height, horizontal/vertical padding, text/background/accent colors and opacity, background visibility, accent edge, shadow and alignment. Height is a minimum: padding always leaves room for the text or item icon. New HUDs use white text on neutral backgrounds with the accent edge off. Old default green backgrounds/edges migrate to this plain style; custom colors are retained. Settings have a live HUD preview, with a side panel on wide screens and a compact preview on smaller screens; switches show their state with both text and a thumb.

Numeric settings use draggable sliders throughout the module settings. Clicking the track selects a value; dragging remains captured beyond the track and clamps to the range. Tab focuses a slider and arrow keys adjust it. Changes appear in the preview immediately and are saved on release. Alignment uses a named choice rather than a numeric slider.

**Change** opens a visual color picker: saturation/brightness square, hue strip, alpha strip, checkerboard transparency preview and live module preview. Choose a color with the mouse; typing is optional. The field accepts `RRGGBB` or `RRGGBBAA`. Text, backgrounds and accent/crosshair colors have independent opacity. Apply saves the draft; Cancel discards it. Arrow keys adjust the focused color control; Enter switches the square between saturation and brightness adjustment.

The crosshair is locked to the exact screen center. Old saved positions are ignored; dragging and arrow-key movement cannot move it. Its size, gap, thickness, color and opacity remain configurable.

Keystrokes draws W/A/S/D, left/right mouse buttons and Space as individual keys. Key size, spacing, widget scale, released background, pressed color, text color, mouse buttons, Space and CPS labels are configurable. Pressed labels automatically choose black or white for contrast. The mod uses Minecraft's normal font, as requested; no custom font or resource-loader dependency is bundled.

## Modules

Twenty-two modules: FPS, coordinates, ping, keystrokes, left/right CPS, armor durability, potion effects, local clock, movement speed, toggle sprint, zoom, crosshair, direction, biome, held item, hunger/saturation, world time, session duration, memory, server address, block indicator and reach.

Block Indicator shows the localized name and native item icon of the block under the crosshair. Looking away clears the previous target. Reach shows the eye-to-hit distance in blocks for the current block or entity target, or "No target" when nothing is hit. Both default off and read the current Minecraft hit result every five ticks, without network requests. They use the shared HUD color, alpha, dimensions, padding and slider controls.

The larger module/configuration request is tracked separately in [ROADMAP.md](./ROADMAP.md). Entries there are not advertised as implemented modules.

## Friends

The module menu's **Friends** button opens requests, friends and privacy settings. It remains available at narrow GUI sizes. Add a player by name, accept their request, and see the version and server they choose to share. **Privacy** opens sharing and toast settings on shorter screens. **DND** mutes Cairn notifications; **Appear offline** hides activity from friends. Blocking removes the friendship and queued notifications in both directions.

**Join** appears only when a friend shares an address and the local client is outside a world. Leave the current world before joining another server. All six Fabric adapters and all three Forge adapters provide the connection handler; in-game request, join and privacy checks have been completed on 26.3 and Forge 1.8.9. Other versions currently have build and shared-test coverage for these changes.

Zoom has adjustable strength and optional smoothing. While zooming, the mouse wheel changes the zoom level instead of the hotbar slot, and the cinematic camera can be applied for the duration of the zoom; the previous camera setting is restored on release. Toggle sprint has auto-sprint and reset-on-stop options while respecting food, sneak, item use and collision checks. Clock supports seconds and 12/24-hour formats. Crosshair has configurable size, gap, thickness, center dot, color and opacity, and stays locked to the center. It replaces Minecraft's crosshair by default, with an option to retain the original. Armor and effects use up to eight vertical rows with native item/effect icons and durability or duration text. Held item currently uses a compact text display.

Cosmetics are equipped from the in-game page after server authentication. Without a server or verified sign-in, catalog items remain previews. The launcher previews and uploads capes but does not equip them. Staff capes and gold badges require an administrator-granted role on the server; offline accounts cannot authenticate to a production server or obtain staff roles through its name-based grant command. See [server setup and grants](../cosmetics-server/README.md#first-admin-and-staff-roles).

Choices persist in `config/cairn-client.properties` inside the instance. Existing settings survive upgrades, and invalid numeric settings are bounded.

## Instance behavior

The launcher installs only `mods/cairn-client-managed.jar`. Turning Cairn off removes this managed artifact on the next Java launch and passes `-Dcairn.client.enabled=false`. Other mods, their enabled states, worlds and Cairn settings remain. A manually installed Cairn jar is reused and stays on disk; the same JVM flag disables its features. A foreign file using the reserved filename is never overwritten or deleted. An instance keeps its existing loader.

## Build and validation

`npm run build:client` rebuilds all nine artifacts under `src-tauri/client-mods`, then runs shared Java tests. Bundled jars are included in source so building the launcher does not require Java tools or mod downloads.

Requirements: Node and curl; JDK 25 for 26.3; Gradle 8.14 running on JDK 21 for mapped Fabric and legacy Forge; an installed JDK 8 for Forge's legacy tooling. Set `CAIRN_MODERN_JAVA_HOME`, `CAIRN_LEGACY_JAVA_HOME`, `CAIRN_JAVA_INSTALLATIONS` (comma-separated JDK directories) and `CAIRN_GRADLE` when tools are not the system defaults. Output bytecode targets Java 8 for Forge and 1.16.5, Java 17 for 1.20.1/1.20.4, Java 21 for 1.21.1/1.21.11 and Java 25 for 26.3. Minecraft is not included in output jars.

Build one family with `node client-mod/scripts/build.mjs forge`, `mapped` or `modern`. Minecraft compile artifacts use Mojang SHA-1 checks; pinned Fabric Loader and Mixin compile dependencies for 26.3 use SHA-256 checks. Mapped targets use Unimined's dependency resolution and mixin remapping.

`npm run test:client` checks configuration, search, controls, arbitrary colors, per-widget size, presets, snapping, resize handles, cached HUD measurement and settings pages at twelve viewport sizes. `npm test` covers the Options toggle; `npm run test:rust` covers managed installation and preservation of user files. To run the headless core CPU benchmark after compiling tests: `java -cp client-mod/common/build/test-classes app.cairn.client.HudBenchmark`. This excludes Minecraft, font drawing and GPU work and does not measure in-game FPS.

### Two-client friends test

On Linux, install Xvfb, xdotool, ImageMagick (`import`), Python 3, curl, ripgrep and the normal launcher build dependencies. Start a matching vanilla Minecraft server in offline mode on loopback with its whitelist disabled. The test creates its own loopback cosmetics server with dev authentication, private X display and two offline identities. It drives Add, Accept, Join, DND, hide-server, appear-offline and Block through the real game windows, checks API responses, and saves screenshots and a JSON result. Inspect the DND screenshots to confirm toast absence; that visual check is not automated.

```sh
CAIRN_SMOKE_SERVER=127.0.0.1:25575 bash client-mod/scripts/friends-e2e.sh 26.3 fabric
E2E_JAVA=/path/to/jdk8/bin/java CAIRN_SMOKE_SERVER=127.0.0.1:25576 \
  bash client-mod/scripts/friends-e2e.sh 1.8.9 forge
```

`CAIRN_SMOKE_DIR` selects the artifact directory; the default is a new temporary directory. Set `CAIRN_SMOKE_CACHE` to a launcher data directory to copy its `meta/` and `java/` caches into the isolated test directory. The harness retains artifacts and stops its own clients and services on exit. Use a fresh directory for each completed run, since successful runs leave players blocked. No Microsoft account is used, and dev authentication is never exposed outside loopback.

## Rendering decisions

ENERGY 2 / RHYTHM 1 / MOTION 1. The module gallery follows the user's reference: a centered graphite panel, red page selection, monochrome module previews, a separate settings cog and a large green enabled switch. Equal cards represent equal module choices; two, three or four columns follow the available width. The panel is bounded in height to avoid empty full-screen surfaces. Short viewports omit gallery previews so controls remain usable. Red identifies navigation and focus, while green plus an explicit Enabled label marks activation. Rounded corners separate controls from the game background. Minecraft's normal font remains in use. Plain HUD defaults keep attention on game information; custom colors remain available. The settings preview sits beside the controls on wide screens so changes can be compared without scrolling away. Its backdrop comes from Minecraft's own panorama, with no downloaded artwork. The title screen uses the same controls and panorama to match the supplied reference; the editor keeps its center actions compact so most of the HUD stays visible.

The primary navigation stays above every page. Menu-key rebinding moves to the footer when the header is full. Labels are measured and clipped to their controls. Previews are drawn locally with simple geometry rather than downloaded artwork or another client's assets. The menu adds no decorative animation. In the 26.3 deferred GUI renderer, transformed text uses separate strata so later previews cannot hide navigation labels.

HUD state is parsed once when the mod starts. Text layout is cached until content, width or scale changes. Click counters refresh at 100 ms; clocks and memory at one second; game statistics at five ticks. Disabled statistics skip formatting. Configuration writes happen on control changes and drag release, never during HUD rendering. Text component caches are bounded. Keystrokes input and zoom stay responsive independently of statistics formatting.

## Adapter boundaries

`common` owns settings, module state, search, menu, HUD rendering and editing. `forge` uses legacy events and MCP/SRG remapping through Unimined. `fabric-mapped` targets the obfuscated 1.16.5, 1.20.1, 1.20.4, 1.21.1 and 1.21.11 APIs through three adapter families (`src/v1_16`, `src/v1_20`, `src/v1_21_9`), per-version HUD mixins where the `Gui` signature differs, and intermediary remapping. `modern` targets the unobfuscated 26.3 API. Compatibility is not inferred merely because another version number is newer.
