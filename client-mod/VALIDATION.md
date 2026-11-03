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
