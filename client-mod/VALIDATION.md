# Validation record

- Hard gate PASS: all nine mod jars build; TypeScript/Vite build passes; 59 frontend tests and the shared Java interaction workflows pass. Java checks cover search, colors, per-widget size, presets, keyboard selection, scrolling, dragging, resize handles and viewport bounds. The Minecraft 26.3 menu was visually inspected with its normal font restored.
- Purpose gate PASS: the palette, solid surfaces, module list, inspector, font choice and spacing have written reasons in README.md. No decorative images, gradients, effects, invented testimonials or external navigation are used.
- Liveliness PASS: ENERGY 2 / RHYTHM 1 / MOTION 1; a repeated moss selection edge identifies Cairn, search leads module discovery, and the inspector separates configuration from activation.
- Craftsmanship PASS: each control has a state change or navigation action in the shared menu; settings are persisted; numeric inputs are bounded; settings pages are checked at twelve viewport sizes. Default muted text has at least 4.78:1 contrast on selected surfaces, and hovered controls use the brighter foreground. Pressed key labels choose black or white for contrast. Custom font assets and font-loader dependencies are absent from every bundled jar.

Native tests cover enabling/disabling while preserving user files, reuse of manually installed Cairn, foreign-file protection, compatibility of older instance JSON, target versions/bytecode, normal-font packaging and pre-launch checks using the next launch's mode without altering files.

Minecraft 26.3 was exercised in an isolated test directory; 1.21.1 was also launched with the mapped adapter. Forge 1.8/1.8.8/1.8.9 and Fabric 1.20.1 have compilation and artifact checks, not a completed gameplay test in that initial round. Support is restricted to the nine versions listed in README.md. No universal compatibility or in-game performance gain is claimed. The optional headless HUD CPU benchmark excludes Minecraft rendering and GPU work.

## November 2026 update

All nine jars were rebuilt and launched headlessly (Xvfb, software OpenGL) into a local vanilla server: Fabric 26.3, 1.21.11, 1.20.4 and 1.16.5 and Forge 1.8.9. Each run captured the in-world HUD, the menu opened from its key and the zoom view. Under Xvfb, LWJGL 3.2.2 (1.16.5) reports the synthetic Right Shift as Left Shift, so that run rebound the menu to M through the config file. On 1.16.5 the vanilla `--server` launch argument crashes the game with or without Cairn, so that run joins from the server list. Fabric 1.20.1 and 1.21.1 were built and unit-checked, but not relaunched in this round. The Java interaction tests also cover overlap-free presets, legacy layout migration, key rebinding, scroll zoom and restoring the cinematic camera.

## Friends verification, 3 November 2026

The Release and CI workflows for `f2aab6c6b7378cd78df0c10e1fb57a348c806c3e` succeeded. [Build 25](https://github.com/rexsystems/cairn/releases/tag/v0.1.0-build.25) published v0.1.0+25. This verification work stays on `feature/social-verification`; it has not been merged or released.

`client-mod/scripts/friends-e2e.sh` ran two real offline clients against a private loopback cosmetics server with `COSMETICS_DEV_INSECURE_AUTH=1` and a matching local vanilla server. The game windows ran under Xvfb with software OpenGL. The test drove friend requests and acceptance, joined the second client's shared server from the first client's title-screen menu, and verified version/server presence. It changed hide-server, appear-offline, DND and blocking through the game UI and checked the resulting API responses. Both clients joining was also visible in each Minecraft server's log.

The complete friends scenario passed on Fabric 26.3 and Forge 1.8.9. DND screenshots were inspected: a new incoming request reached the client while its Cairn toast stayed muted. SQLite tests reopen the server state and verify event recovery, cursor continuity after pruning, the 50-event limit, blocking and revocation of queued activity after privacy changes.

Local validation passed: `cargo test` in `src-tauri/` (176 passed, two pre-existing ignored integration tests), cosmetics-server tests (21 passed), Clippy with `--all-targets -- -D warnings` in both Rust projects, `npm test` (70 passed), `npm run build:web`, and `npm run build:client` (all nine jars plus shared Java tests). No in-game FPS improvement is claimed from the software-rendered runs.

| Version | Loader | Rebuilt and shared tests | Two-client friends, Join and privacy | Cape/badge rendering in this run |
| --- | --- | --- | --- | --- |
| 26.3 | Fabric | Pass | Pass | Not checked |
| 1.21.11 | Fabric | Pass | Not run | Not checked |
| 1.21.1 | Fabric | Pass | Not run | Not checked |
| 1.20.4 | Fabric | Pass | Not run | Not checked |
| 1.20.1 | Fabric | Pass | Not run | Not checked |
| 1.16.5 | Fabric | Pass | Not run | Not checked |
| 1.8.9 | Forge | Pass | Pass | Not checked |
| 1.8.8 | Forge | Pass | Not run | Not checked |
| 1.8 | Forge | Pass | Not run | Not checked |

Artifacts in `cosmetics/previews/` include `e2e-26.3-friends-and-toast.png`, `e2e-26.3-friends-dnd.png`, `e2e-26.3-friends-dnd-muted-request.png` and the matching `e2e-1.8.9-friends*.png` captures. The two `friends-*-result.json` files record the automated assertions. The 26.3 Friends image also captures the accepted-request toast.

The real-game run found and fixed two 26.3 adapter defects: SDL uses mouse button values 1/3 rather than GLFW's 0/1, and custom text fields must enable the text-input manager. Forge startup exposed native-library truncation while another client prepared its files. Regression tests verify that changed libraries preserve the old open file and identical extractions preserve its inode. A concurrent-writer test covers unique atomic-write temporary files.

Docker Compose `up --build -d` was exercised against the local Podman Docker API socket. The final image served `/healthz`, ran as `cosmetics` with a read-only root filesystem and dropped capabilities, and retained a CLI-written staff role across a container restart. No Dockerfile change was needed. Instant-moderation tests check `approved` in the upload response and `/v1/me`, and an empty pending queue for that upload.

- Hard gate PASS: all nine jars build; the Java workflows pass at twelve viewport sizes; the two real-game scenarios exercise the changed Friends controls and 26.3 mouse/text input. Screenshots show actual local test accounts and addresses.
- Purpose gate PASS: README.md records the existing palette/font choices and the reason navigation moves to a separate row or footer. No new decorative assets or effects were introduced.
- Liveliness PASS: ENERGY 2 / RHYTHM 1 / MOTION 1 is retained; the moss selection edge, normal Minecraft font and solid menu surfaces remain consistent in the captured game windows.
- Craftsmanship PASS: navigation and menu rebinding stay reachable at all twelve tested viewports. Tests cover Tab as the menu binding and leaving text focus with Tab; in-game request/accept, Join, privacy and blocking have real effects on both verified versions.

The remaining handoff work is still open: per-version cape and badge rendering, scoreboard-line alignment, a Forge cape render hook, emotes and the later new features, Windows checks, description-image proxying and a real two-account Discord join test. No new release is claimed for this branch.

## Module gallery and mouse verification, 3 November 2026

The Right Shift menu now follows the supplied visual reference: a bounded charcoal panel, red navigation, a module gallery with individual previews, separate settings controls and green activation switches. The normal Minecraft font is retained. README.md records the layout, icon, spacing and color decisions.

`scripts/menu-e2e.py` launched all nine supported versions as real offline Minecraft processes under Xvfb/software OpenGL. On each version, native mouse input selected Modules, disabled and re-enabled FPS, typed `zoom` into search and disabled the filtered Zoom module. Assertions read the persisted configuration after those actions. Settings and close screenshots were inspected. Three captures were repeated with a longer settling interval because the initial captures caught incomplete frames; all three settled settings pages rendered correctly. The 1.16.5 run used M for the previously documented Xvfb Right Shift limitation. These tests cover menu interaction, not per-version multiplayer cosmetics or GPU performance.

A separate 26.3 client joined a loopback vanilla server. Two native left clicks and three native right clicks produced `2 CPS` on the left and `3 CPS` on the right in the in-world HUD. Shared Java regression tests cover both GLFW 0/1 and SDL 1/3 mouse-button conventions. The tests also cover gallery activation, opening module settings, closing, text entry and reserved menu keys at twelve viewport sizes.

Evidence is stored in `cosmetics/previews/menu-modules-<version>.png`, `menu-settings-26.3.png`, `menu-settings-1.8.9.png`, `menu-cps-26.3.png` and `menu-native-results.json`. The Friends harness coordinates were updated for the new header; its full social scenario has not been repeated after this visual redesign.

All nine client artifacts and shared Java tests pass. The local Tauri debug application was built with the updated embedded jars. GitHub push and PR CI runs 41644863884 and 41644873632 passed, including Check, Fedora and Flatpak. PR build jobs disable updater signing explicitly; the Release workflow still signs release builds.

- Hard gate PASS: native navigation, switches and search passed on all nine versions; settled settings and close captures were inspected. Shared tests verify keyboard interaction and twelve viewport sizes. No fictional product claims or decorative assets were added.
- Purpose gate PASS: the user's reference supplies the graphite/red gallery direction; README.md explains the grid, normal font, spacing and original module previews. Equal card sizes support scanning and separate activation from settings.
- Liveliness PASS: ENERGY 2 / RHYTHM 1 / MOTION 1; red selected navigation anchors the gallery, repeated module previews identify functions, and hover/focus states provide interaction feedback.
- Craftsmanship PASS: persisted state confirms actual actions. Contrast checks pass for normal text: EEEEEE on BF303D 4.88:1, A4E3B4 on 1D432B 7.53:1, AEB5BF on 191E25 8.10:1. SDL and GLFW click mappings have independent regression coverage.

## Title screen, HUD overview and verified grants

The new Right Shift flow was driven with native mouse/keyboard input on all nine supported versions: HUD editor, Cairn Settings, Modules navigation, FPS off/on, typed Zoom search and the filtered switch. All assertions passed. The final layout was additionally repeated on 26.3, Forge 1.8.9 and 1.16.5; their title-screen Singleplayer action opened the real vanilla world selection/creation screen. Native world-screen, title, overlay and settings captures are included for these three versions. The 1.16.5 synthetic key remains rebound to M for the documented Xvfb/LWJGL limitation.

Separate 26.3 and Forge 1.8.9 clients joined isolated loopback vanilla servers. Server console commands equipped four diamond armor pieces and applied Speed/Haste. Actual armor/effect icons and durability/duration rows were visually checked in both clients. Each client opened Cosmetics, equipped Ember and the Moss badge through native clicks, and SQLite contained exactly that selection afterward. The Forge third-person capture shows the equipped cape on the player. The 26.3 FPS settings capture shows the current FPS in its live preview; the custom HEX field saved AADDFF through native text input. These in-game cosmetics checks use an isolated loopback development backend, not the user's production-mode local service.

The local production-mode service was separately started on 127.0.0.1:8787 with insecure development auth off. Its health endpoint succeeded and an unverified offline login was refused with HTTP 401. A real Microsoft/Mojang account sign-in to that service has not been completed by this test. Server tests use a controlled Mojang session-server fixture to verify grants by case-insensitive name, administrator authentication, UUID persistence across restart, revocation and offline/dev rejection. Old privileged identities stay locked until a fresh verified login. The launcher no longer exposes an equip command or equip controls; its component test checks the preview/upload-only page.

Local checks pass: all nine client jars and Java workflows; 176 launcher Rust tests with two pre-existing ignored tests; two server unit tests and 21 server API tests; both Clippy checks with all targets and warnings denied; 71 frontend tests; web build; and the local Tauri application build. Shared tests cover plain-style migration, preservation of custom text colors, preview opacity/background state, row icons, actual color swatches, title mouse/keyboard actions and all settings pages at twelve viewport sizes.

Artifacts include `title-native-results.json`, `title-*.png`, `overlay-*.png`, `settings-*.png`, `worlds-*.png`, `hud-icons-26.3.png`, `hud-icons-1.8.9.png`, `fps-preview-26.3.png`, `cosmetics-equipped-*.png` and `cape-thirdperson-1.8.9.png` under `cosmetics/previews/`.

- Hard gate PASS: all nine native menu scenarios and twelve viewport tests pass; representative title, settings, HUD icons and cosmetics interactions were run in Minecraft. Existing vanilla actions and disabled states are retained; no store or advertising controls were invented.
- Purpose gate PASS: README.md records the user-directed title/editor/preview layout, normal font, plain HUD defaults, local geometric icons and vanilla panorama backdrop. Switch labels supplement their color and thumb position.
- Liveliness PASS: ENERGY 2 / RHYTHM 1 / MOTION 1. Red navigation and the central Cairn Settings control provide hierarchy; consistent previews and restrained hover/focus feedback match the requested direction.
- Craftsmanship PASS: persisted configuration and SQLite verify real actions. Custom RGB/opacity changes affect the same HUD preview renderer; native armor/effect icons were checked on 26.3 and 1.8.9. Normal text retains the earlier checked neutral/red contrast values; keyboard title and module controls have visible focus.

Per-version limits: the new menu was exercised on every supported version; real armor/effect rows and cosmetic selection were checked on 26.3 and 1.8.9 only. Two-client cape/badge rendering across all versions, scoreboard alignment and the Forge cape render-hook change remain open. The larger supplied module inventory is tracked in ROADMAP.md and is not implemented by this change. No public release is claimed.
