# Changelog

Notable changes to Cairn. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and versions follow [Semantic Versioning](https://semver.org/). Every push to `main`
ships as `X.Y.Z+N` (build N of X.Y.Z); `npm run bump -- X.Y.Z` starts a new version.

## [Unreleased]

### Added
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

### Changed
- Mod descriptions are sanitized with DOMPurify, and the app ships a Content Security Policy.
- The CurseForge API key is no longer in the source. Builds take it from `CAIRN_CURSEFORGE_API_KEY`, and Settings can override it.
- The Microsoft client ID lives in one module and can be set with `CAIRN_MSA_CLIENT_ID`.
- Offline accounts need a Microsoft account on the launcher first.
- A crashed background task no longer takes every later lock down with it.
- File and network commands run off the main thread, progress events are throttled and live logs are batched, so the window stays responsive during installs and while the game runs (most noticeable on Windows).

## [0.1.0] - 2026-10-11

- First public builds.
