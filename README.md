# Cairn

Minecraft launcher for Fabric, Quilt, Forge and NeoForge instances, with Modrinth installs built in.

Cairn is not affiliated with Mojang, Microsoft or Modrinth. Minecraft is a trademark of Mojang AB.

## Install

Grab a build from the [GitHub Releases](https://github.com/rexsystems/cairn/releases) page:

- Windows: `Cairn_*_x64-setup.exe`
- Linux: `.AppImage`, `.deb` or `.rpm` (x86_64 and aarch64)
- macOS: `.dmg` / `.app` (Intel and Apple Silicon)

Release builds check for updates on launch. When a newer build is on the update channel you get a banner in the launcher (and a short toast). Use **Update now**, or **Settings → Check for updates**.

## Develop

Needs Node 22+, Rust (stable), and the usual Tauri 2 system deps for your OS.

```bash
npm ci
npm run dev
```

Useful scripts:

| Script | What it does |
| --- | --- |
| `npm run dev` | Tauri app + Vite |
| `npm run build` | Production desktop build |
| `npm test` | Frontend tests |
| `npm run test:rust` | Rust unit tests |

Data lives under the OS app-data directory for `app.cairn.launcher` (open it from **Settings → Open data folder**).

## Settings worth knowing

- **Custom window controls**: optional Cairn title bar (min / restore / close) instead of the system chrome
- Theme, reduce motion, hide-on-launch, gamepad nav, Discord Rich Presence
- Default RAM, JVM args, window size, parallel downloads
- Game environment isolation so launcher GTK/WebKit flags do not leak into Minecraft
- Managed Eclipse Temurin installs when a version needs a Java you do not have yet

## Updates (maintainers)

Every push to `main` runs `.github/workflows/release.yml`: it stamps a build number, builds Windows / Linux / macOS, signs updater artifacts, uploads them to Cloudflare R2, and publishes a GitHub Release.

The app reads:

`https://pub-f68c946cc3b343619b3faea90c762d28.r2.dev/latest.json`

Required secrets: `TAURI_SIGNING_PRIVATE_KEY` (and password if used), `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`. The matching public key is in `src-tauri/tauri.conf.json`.

## License

[AGPL-3.0-or-later](./LICENSE)
