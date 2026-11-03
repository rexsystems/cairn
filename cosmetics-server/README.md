# Cairn cosmetics server

A small self-hosted backend for Cairn Client cosmetics:

- Mojang-verified sign-in.
- Equipped capes and badges.
- Moderated custom capes, static or animated.
- Staff roles.
- A public online counter.
- Friends, presence and notifications for Cairn Client players.

It is one Rust binary with a SQLite database and a folder of cape PNGs.

## How sign-in works

The server never sees a Minecraft or Microsoft access token.

1. The client sends `POST /v1/auth/start {"name"}` and gets back a random `serverId`. That id is single use and expires after 2 minutes.
2. The client calls Mojang's `session/minecraft/join` with its own access token and that `serverId`. This is the same thing vanilla does when it joins an online-mode server.
3. The client sends `POST /v1/auth/finish {"name","serverId"}`. The server asks Mojang's `hasJoined` endpoint whether that player really joined. If they did, it returns a random bearer token valid for 1 hour by default. Only a SHA-256 of that token is stored.

Offline (cracked) accounts can't pass step 3, so they never get cosmetics.

## Security measures

- **Auth:**
  - Challenges are single use and bound to the name they were issued for.
  - Banned players lose all sessions, and are hidden from lookups.
- **Limits:**
  - Per-IP token-bucket rate limits:
    - sign-in: 6/min
    - uploads: 1/min after a burst of 5
    - friend requests, removals and blocks: 30/min after a burst of 15
    - notification polls: 60/min after a burst of 20
    - everything else: `COSMETICS_RATE_PER_MINUTE`
  - 16 KiB request bodies (1 MiB for cape uploads) and a 20 s request timeout.
- **Cape uploads:**
  - PNG only, decoded under strict size and allocation limits.
  - Then re-encoded to plain RGBA, so no original bytes or metadata are kept.
  - Stored under their SHA-256.
  - Shape rules:
    - Width is a multiple of 64, up to 1024.
    - Each frame is width × width/2.
    - Animated capes stack up to 32 frames vertically, with a 50–1000 ms frame time.
    - At most 4 Mpx in total.
- **Moderation:**
  - By default, custom capes stay invisible to other players until an admin approves them.
  - With `instant` moderation, uploads are saved and reported as approved immediately.
  - A rejected image can't be uploaded again.
  - A new upload withdraws the player's previous pending one.
- **Admin access:**
  - The admin API needs `COSMETICS_ADMIN_TOKEN` (32+ chars, compared in constant time). It is off when the token is unset.
  - There are no default credentials.
- **Headers:**
  - CORS is closed unless you list origins.
  - Every response gets `nosniff`, a `default-src 'none'` CSP and `no-referrer`.
- **Docker:** the image runs as a non-root user with a read-only root filesystem and no capabilities.

## Friends and presence

Friends only exist between players who have signed in with Cairn Client. Presence comes only from
the in-game mod's heartbeat, so a player who isn't running Cairn Client shows as offline. The
launcher can list friends and change settings, but never reports activity.

- **Requests:** by name (case-insensitive) or UUID. Sending a request to someone who already sent
  you one accepts it. Up to 100 pending outgoing requests and 200 friends.
- **What friends see** is set per player in `/v1/me/social`:
  - `share`: `full` (version, mode and server address), `game` (version and mode, never the server)
    or `status` (online or offline only).
  - `appearOffline`: friends see offline with no activity, and get no "came online" notice.
  - `dnd`: friends see "do not disturb". Notices are still queued; the client skips the toasts.
  - `requests`: `everyone` or `none`.
- **Privacy:** there is no endpoint that shows anyone's presence except to their friends. Hidden
  fields are never put in a reply or a notice. The public `/v1/players` lookup never includes activity.
- **Blocking** removes the friendship and any requests both ways and drops queued notices between
  the two. Requests from a blocked player look sent to them but never arrive.
- **Notifications** are stored in SQLite: the last 50 per player for 10 minutes. Events and
  cursors survive server restarts. Connecting without a cursor skips the backlog. Blocking or
  removing a friend deletes queued notices between both players; privacy changes revoke queued
  activity notices before friends can retrieve them.

## Self-hosting

You need a domain (e.g. `cosmetics.example.com`) pointing at a Linux box with Docker.

```sh
git clone https://github.com/rexsystems/cairn && cd cairn/cosmetics-server
cp .env.example .env
sed -i "s/^COSMETICS_ADMIN_TOKEN=.*/COSMETICS_ADMIN_TOKEN=$(openssl rand -hex 32)/" .env
docker compose up -d --build
curl http://127.0.0.1:8787/healthz   # -> ok
```

The container publishes on `127.0.0.1:8787` only. Put a TLS reverse proxy in front of it. With Caddy, which fetches certificates automatically, the whole `Caddyfile` is:

```
cosmetics.example.com {
    reverse_proxy 127.0.0.1:8787
}
```

With nginx, add a TLS server block containing:

```
location / {
    proxy_pass http://127.0.0.1:8787;
    proxy_set_header X-Forwarded-For $remote_addr;   # overwrite, don't append client values
    client_max_body_size 1m;
}
```

Keep `COSMETICS_TRUST_PROXY=1` only while a proxy you control sets `X-Forwarded-For`. Otherwise clients could fake their IP to dodge rate limits.

Without Docker, run `cargo build --release`, then start `target/release/cairn-cosmetics-server serve` with the same environment variables (see `.env.example`).

### Environment

| Variable | Default | Meaning |
| --- | --- | --- |
| `COSMETICS_BIND` | `127.0.0.1:8787` | Listen address |
| `COSMETICS_DATA_DIR` | `data` | SQLite database and `capes/` folder; back this up |
| `COSMETICS_ADMIN_TOKEN` | unset (admin API off) | Bearer token for `/v1/admin/*`, 32+ chars |
| `COSMETICS_CAPE_MODERATION` | `approval` | `approval` or `instant` |
| `COSMETICS_TRUST_PROXY` | off | Use the last `X-Forwarded-For` hop as the client IP |
| `COSMETICS_CORS_ORIGINS` | empty | Comma-separated browser origins; the launcher and mod don't need any |
| `COSMETICS_RATE_PER_MINUTE` | `120` | Normal API calls per IP per minute |
| `COSMETICS_TOKEN_TTL_SECS` | `3600` | Session token lifetime |
| `COSMETICS_SESSION_SERVER` | `https://sessionserver.mojang.com` | Only change for testing |
| `COSMETICS_DEV_INSECURE_AUTH` | off | **Dev only.** Skips Mojang and trusts any name (offline UUIDs). Refuses to start unless bound to loopback |

### First admin and staff roles

Roles are `player`, `staff`, `admin` and `banned`. `staff` and `admin` unlock the staff cape and the gold badge; in-game, staff are told apart only by that badge colour next to their vanilla name. Only the administrator CLI or admin-token API grants roles. Players choose their equipped cosmetics in Minecraft; the launcher only previews the catalog and uploads custom capes.

Grant by Minecraft name after that account signs in through Mojang at least once. Names are case-insensitive and resolve to a verified UUID. Unknown, offline/dev and ambiguous names are refused. Roles follow the UUID if the account changes its name. UUID grants are still available before first login, but staff cosmetics remain locked until Mojang verifies that UUID. Existing databases require a fresh verified login before their old privileged roles become effective. Development authentication never unlocks staff cosmetics.

```sh
docker compose exec cosmetics cairn-cosmetics-server role <uuid> admin
docker compose exec cosmetics cairn-cosmetics-server role <uuid> staff
docker compose exec cosmetics cairn-cosmetics-server role <verified-name> staff
```

You can also use the HTTP admin API:

```sh
curl -X PUT -H "Authorization: Bearer $COSMETICS_ADMIN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"role":"staff"}' https://cosmetics.example.com/v1/admin/players/<verified-name>/role

curl -X PUT -H "Authorization: Bearer $COSMETICS_ADMIN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"role":"staff"}' https://cosmetics.example.com/v1/admin/users/<uuid>/role
```

### Reviewing custom capes

```sh
docker compose exec cosmetics cairn-cosmetics-server capes            # pending queue
docker compose exec cosmetics cairn-cosmetics-server approve <hash>
docker compose exec cosmetics cairn-cosmetics-server reject <hash>
```

The same actions over HTTP:

- `GET /v1/admin/capes?status=pending`
- `GET /v1/admin/capes/<hash>/image` to look at a cape
- `POST /v1/admin/capes/<hash>/approve`
- `POST /v1/admin/capes/<hash>/reject`

### Pointing Cairn at your server

- **Launcher:** open Settings, find Cosmetics server, and enter `https://cosmetics.example.com`. The launcher passes it to the game as `-Dcairn.cosmetics.url`.
- **Release builds:** you can bake in a default by building with `CAIRN_COSMETICS_URL=https://cosmetics.example.com`.
- **Mod without the launcher:** set the `-Dcairn.cosmetics.url=...` JVM argument or the `CAIRN_COSMETICS_URL` environment variable.

If no URL is set, or the server is down, everything else keeps working and cosmetics are simply hidden.

## API

| Method and path | Auth | Purpose |
| --- | --- | --- |
| `POST /v1/auth/start` | – | `{name}` → `{serverId}` |
| `POST /v1/auth/finish` | – | `{name, serverId}` → `{token, expiresIn, uuid, name, role}` |
| `GET /v1/me` | bearer | Role, equipped items, custom cape status, unlocked items |
| `PUT /v1/me/equipped` | bearer | `{cape, badge}`: catalog ids, `"custom"`, `"none"` (badge) or `null` |
| `POST /v1/me/cape?frameMs=100` | bearer | Raw PNG body, `Content-Type: image/png` |
| `DELETE /v1/me/cape` | bearer | Remove the custom cape |
| `POST /v1/heartbeat` | bearer | Marks the player online for 90 s; the mod sends it every 60 s. Optional `{activity: {version, mode: menu\|singleplayer\|multiplayer, server}}` |
| `GET /v1/friends` | bearer | `{friends: [{uuid, name, since, status: online\|dnd\|offline, activity}], incoming, outgoing, blocked, settings}` |
| `POST /v1/friends/requests` | bearer | `{target}` (name or UUID) → `{status: sent\|accepted, uuid, name}` |
| `POST /v1/friends/requests/<uuid>/<accept\|decline\|cancel>` | bearer | Answer an incoming request or cancel your own |
| `DELETE /v1/friends/<uuid>` | bearer | Remove a friend on both sides |
| `PUT`, `DELETE /v1/blocks/<uuid>` | bearer | Block or unblock |
| `GET`, `PUT /v1/me/social` | bearer | `{share, appearOffline, requests, dnd}`; `PUT` accepts any subset |
| `GET /v1/events?after=<cursor>&wait=<s>` | bearer | Long poll, up to 15 s. Without `after` it only returns the current `cursor` |
| `GET /v1/players?uuids=a,b,…` | – | Up to 100 UUIDs; only Cairn users are returned. Cached 60 s |
| `GET /v1/stats` | – | `{online, peak, peakAt, total}`. Cached 30 s |
| `GET /v1/catalog` | – | Built-in capes and badges |
| `PUT /v1/admin/players/<name>/role` | admin token | Grant/revoke a role by verified Minecraft name |
| `GET /v1/capes/<hash>.png` | – | Approved custom capes only |

## Development

```sh
cargo test
COSMETICS_DEV_INSECURE_AUTH=1 COSMETICS_CAPE_MODERATION=instant cargo run   # local testing with offline clients
```

`catalog.json` is generated by `client-mod/cosmetics/generate.py` together with the textures shipped in the mod and launcher.

The launcher repository's [two-client friends harness](../client-mod/README.md#two-client-friends-test) runs real offline game clients against an isolated loopback dev server. The [validation record](../client-mod/VALIDATION.md#friends-verification-3-november-2026) distinguishes those in-game checks from the Rust API tests and records the Docker Compose smoke test.
