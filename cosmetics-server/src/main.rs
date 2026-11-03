use cairn_cosmetics_server::{config::Config, db, normalize_uuid, router, AppState};
use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::Duration;

const USAGE: &str =
    "usage: cairn-cosmetics-server [serve | role <uuid> <player|staff|admin|banned> | capes [pending|approved|rejected] | approve <hash> | reject <hash>]";

fn main() -> ExitCode {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("config error: {err}");
            return ExitCode::FAILURE;
        }
    };
    let result = match args.first().map(String::as_str) {
        None | Some("serve") => serve(cfg),
        Some(cmd) => admin(cfg, cmd, &args[1..]),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn admin(cfg: Config, cmd: &str, args: &[String]) -> Result<(), String> {
    let state = AppState::new(cfg)?;
    let conn = state.db();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let err = |e: rusqlite::Error| e.to_string();
    match (cmd, args) {
        ("role", [uuid, role]) => {
            let uuid = normalize_uuid(uuid).ok_or("invalid UUID")?;
            if !db::ROLES.contains(&role.as_str()) {
                return Err(USAGE.into());
            }
            db::set_role(&conn, &uuid, role, now).map_err(err)?;
            println!("{uuid} is now {role}");
        }
        ("capes", rest) => {
            let status = rest.first().map(String::as_str).unwrap_or("pending");
            for cape in db::capes_with_status(&conn, status, 1000).map_err(err)? {
                println!("{}  owner={}  {}x{}  frames={}  frameMs={}", cape.hash, cape.owner, cape.width, cape.height, cape.frames, cape.frame_ms);
            }
        }
        ("approve" | "reject", [hash]) => {
            let status = if cmd == "approve" { "approved" } else { "rejected" };
            if !db::set_cape_status(&conn, hash, status, now).map_err(err)? {
                return Err("no such cape".into());
            }
            println!("{hash} {status}");
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

fn serve(cfg: Config) -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    runtime.block_on(async move {
        if cfg.dev_insecure_auth {
            tracing::warn!("COSMETICS_DEV_INSECURE_AUTH is on: anyone can sign in as any name. Never use this in production.");
        }
        if cfg.admin_token.is_none() {
            tracing::info!("COSMETICS_ADMIN_TOKEN is not set, so the admin API is disabled (the CLI still works).");
        }
        let bind = cfg.bind;
        let state = AppState::new(cfg)?;
        let sweeper = state.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(60));
            loop {
                tick.tick().await;
                sweeper.sweep();
            }
        });
        let listener = tokio::net::TcpListener::bind(bind).await.map_err(|e| format!("bind {bind}: {e}"))?;
        tracing::info!("cosmetics server listening on {bind}");
        axum::serve(listener, router(state).into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await
            .map_err(|e| e.to_string())
    })
}
