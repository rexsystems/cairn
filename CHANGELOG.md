# Changelog

Notable changes to Cairn. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and versions follow [Semantic Versioning](https://semver.org/). Every push to `main`
ships as `X.Y.Z+N` (build N of X.Y.Z); `npm run bump -- X.Y.Z` starts a new version.

## [Unreleased]

### Added
- In-game Join controls for shared friend server addresses on all supported Fabric and Forge versions, available outside a world.
- Stable and beta update channels, picked under Settings → About.
- SHA-256 checks for Java runtimes, Forge/NeoForge installers and Horizon downloads.
- Opt-in usage stats and crash reports, asked once on first launch.
- Discord Rich Presence shows the version, loader and play time, and follows settings mid-game.
- Frontend tests (Vitest + Testing Library).
- Share a log to mclo.gs from the Logs tab.
- Desktop shortcuts that start one instance (Windows and Linux). Opening Cairn twice focuses the running window.
- Import instances from Prism Launcher, PolyMC and MultiMC.
- A Flatpak manifest, plus CI builds for Fedora (.rpm) and Flatpak.
- Mod metadata is read from the jars themselves, and each instance gets a compatibility report with one-click fixes.
- Installs resolve required dependencies recursively across Modrinth and CurseForge, and show a review before anything downloads.
- Browse and install CurseForge content in Discover. Projects that block third-party downloads open on the website, and Cairn picks the file up from Downloads.
- Memory presets sized to the computer's RAM, tuned G1 flags, and a per-loader performance mod pack.
- Play sessions and crashes are recorded. The Activity tab shows playtime, and **Find the cause** in Logs explains a crash and offers the fix.
- Share an instance as a `CAIRN1:` code that another Cairn rebuilds from the same Modrinth and CurseForge versions.
- Desktop shortcuts on macOS and in the Flatpak build.
- A **Host** tab runs a local server from an instance: client-only mods are left out, the EULA must be accepted first, LAN is opt-in, and an optional playit.gg tunnel lets friends join without opening router ports.
- A **Lighter graphics** setting for slow GPUs or software rendering.
- **Invite friends** from the Host tab or an instance: friends join through Discord (Join or Ask to join on your profile), and Cairn builds the matching instance before starting the game on your server. A copyable invite code works without Discord.
- A **Friends** page lists the people you played with, with Invite again and Join again.

### Changed
- Mod descriptions are sanitized with DOMPurify, and the app ships a Content Security Policy.
- The CurseForge API key is no longer in the source. Builds take it from `CAIRN_CURSEFORGE_API_KEY`, and Settings can override it.
- The Microsoft client ID lives in one module and can be set with `CAIRN_MSA_CLIENT_ID`.
- Offline accounts need a Microsoft account on the launcher first.
- A crashed background task no longer takes every later lock down with it.
- File and network commands run off the main thread, progress events are throttled and live logs are batched, so the window stays responsive during installs and while the game runs (most noticeable on Windows).

### Fixed
- Cairn Client's Friends button remains accessible at narrow GUI sizes, and Tab navigates fields even when it is the menu key.
- Minecraft 26.3 menu clicks, HUD dragging, CPS button selection and text input use the SDL input APIs.
- Preparing a second game retains unchanged native libraries and replaces changed files atomically, so it cannot truncate libraries loaded by a running client. Concurrent atomic writes use separate temporary files.
- Instant-moderation cape uploads report `approved` instead of `pending`.
- Cosmetics-server friend notifications and cursors survive restarts. Privacy changes also remove queued activity notices, including previously shared server addresses.
- The compatibility report no longer flags multi-loader mods for another loader's dependencies, and checks Fabric API modules against their own versions.
- Horizon Play no longer asks you to prepare Minecraft 26.3 by hand: Cairn installs 26.3 into its own meta folder, runs `horizon-prepare`, and imports sounds on first launch.
- Skins, capes and account heads load again. Mojang serves them over http, which the Content Security Policy blocked; Cairn now downloads them itself and caches them.

## [0.1.0] - 2026-10-11

- First public builds.
