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

### Changed
- Mod descriptions are sanitized with DOMPurify, and the app ships a Content Security Policy.
- The CurseForge API key is no longer in the source. Builds take it from `CAIRN_CURSEFORGE_API_KEY`, and Settings can override it.
- The Microsoft client ID lives in one module and can be set with `CAIRN_MSA_CLIENT_ID`.
- Offline accounts need a Microsoft account on the launcher first.
- A crashed background task no longer takes every later lock down with it.

## [0.1.0] - 2026-10-11

- First public builds.
