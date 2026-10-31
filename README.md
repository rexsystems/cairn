# Cairn

Minecraft launcher for Fabric, Quilt, Forge and NeoForge instances, with Modrinth installs built in. Library can also import CurseForge modpack zips and instances from Modrinth App, Prism Launcher, PolyMC and MultiMC. Vanilla 26.3 instances can run the Horizon client.

Cairn is not affiliated with Mojang, Microsoft, Modrinth or CurseForge. Minecraft is a trademark of Mojang AB.

## Install

Grab a build from the [GitHub Releases](https://github.com/rexsystems/cairn/releases) page:

- Windows: `Cairn_*_x64-setup.exe`
- Linux: `.AppImage`, `.deb` or `.rpm` (x86_64 and aarch64)
- macOS: `.dmg` / `.app` (Intel and Apple Silicon)

Release builds check for updates on launch. When a newer build is on your update channel you get a banner in the launcher (and a short toast). Use **Update now**, or **Settings → Check for updates**. **Settings → About → Update channel** switches between Stable and Beta.

A Flatpak manifest lives in [`flatpak/`](./flatpak). CI builds a `cairn.flatpak` bundle from the `.deb`; it isn't on Flathub yet. The Flatpak build updates through Flatpak, so its built-in updater is off.

### Privacy

Cairn asks once, on first launch, whether it may send crash reports (Sentry) and anonymous usage stats. Both are off until you say yes, and you can change either in **Settings**. Nothing is sent before that.

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
| `npm run bump -- X.Y.Z` | Set the version everywhere and start its CHANGELOG section |

Data lives under the OS app-data directory for `app.cairn.launcher` (open it from **Settings → Open data folder**).

## Importing modpacks

**Library → Import pack** accepts:

- Modrinth `.mrpack`
- CurseForge modpack `.zip` (`manifest.json` + `overrides/`, e.g. ATM packs)

CurseForge downloads need a Core API key from [console.curseforge.com](https://console.curseforge.com/). Release builds get it from the `CAIRN_CURSEFORGE_API_KEY` secret at build time; there is no key in the source. You can also set your own in **Settings → CurseForge API key** or with the `CAIRN_CURSEFORGE_API_KEY` environment variable. Local builds without a key can still import Modrinth packs.

**Library → Import from app** copies instances from Modrinth App, RexLauncher, Prism Launcher, PolyMC and MultiMC (a MultiMC folder is only found in the standard data folder, since MultiMC is portable).

**Discover → CurseForge** browses and installs CurseForge mods, packs and resource packs with the same key. Projects that turn off third-party downloads (`allowModDistribution: false`) open on the website instead; Cairn watches your Downloads folder and installs the file once it arrives.

**Library → Paste share code** rebuilds an instance from a `CAIRN1:` code made with **Instance → ⋯ → Share**. Codes only list Modrinth and CurseForge versions, so files added by hand are left out (export an `.mrpack` for those).

## Hosting a server

The **Host** tab of an instance sets up a matching server in `servers/<instance id>` under the data folder (vanilla, Fabric, Quilt, Forge or NeoForge). Client-only mods are skipped. The server won't start until you accept the Minecraft EULA, and it listens on `127.0.0.1` unless you turn on LAN. Cairn never opens ports on your router.

To let friends join over the internet, **Link playit.gg** downloads the playit agent (checked against a pinned SHA-256) and opens playit.gg to approve it. The agent exists for Windows and Linux only.

## Settings worth knowing

- Theme, reduce motion, hide-on-launch, gamepad nav
- Discord Rich Presence: shows the instance, loader and play time. It uses Discord app `1566104003291390072`; set `CAIRN_DISCORD_APP_ID` to use another one
- Default RAM, JVM args, window size, parallel downloads
- Game environment isolation so launcher GTK/WebKit flags do not leak into Minecraft
- Managed Eclipse Temurin installs when a version needs a Java you do not have yet, checked against Adoptium's SHA-256
- **Logs → Share** uploads a log to [mclo.gs](https://mclo.gs) after you confirm, and copies the link
- **Lighter graphics** turns off blur and heavy animations for slow GPUs or software rendering
- **Instance → ⋯ → Create desktop shortcut** (Windows, macOS and Linux, including Flatpak) starts that instance directly. Opening Cairn twice focuses the running window

Sign-in uses the official Minecraft launcher's Microsoft client by default. To use your own Azure app, set `CAIRN_MSA_CLIENT_ID` at runtime or build time (see `src-tauri/src/msa_client.rs`). Adding an offline account needs a signed-in Microsoft account that owns Minecraft.

## Updates (maintainers)

Every push to `main` runs `.github/workflows/release.yml`: it stamps a build number, builds Windows / Linux / macOS, signs updater artifacts, uploads them to Cloudflare R2, and publishes a GitHub Release.

The app reads one of two manifests, depending on the channel:

- Stable: `https://pub-f68c946cc3b343619b3faea90c762d28.r2.dev/latest.json`
- Beta: `https://pub-f68c946cc3b343619b3faea90c762d28.r2.dev/beta/latest.json`

By default every `main` build goes to both, as before. Set the repo variable `CAIRN_STABLE_FROM_TAGS` to `true` to split them: `main` builds then go to Beta only (as GitHub prereleases), and stable moves only when you release a version:

1. `npm run bump -- 0.2.0`, fill in the CHANGELOG section, merge to `main`, and let that build finish.
2. Push a `v0.2.0` tag. `promote-stable.yml` checks that the latest beta build is 0.2.0 and that all its files exist, points the stable manifest at it, and publishes a GitHub release with the CHANGELOG notes. You can also run it by hand with a specific `build_tag`.

Required secrets: `TAURI_SIGNING_PRIVATE_KEY` (and password if used), `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `CAIRN_CURSEFORGE_API_KEY`. Optional repo variables: `CAIRN_STABLE_FROM_TAGS`, `CAIRN_MSA_CLIENT_ID`. The matching public key is in `src-tauri/tauri.conf.json`.

## License

[AGPL-3.0-or-later](./LICENSE)
