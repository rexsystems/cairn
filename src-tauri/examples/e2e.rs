//! End-to-end check against the real services: creates an instance, installs the loader and game,
//! optionally installs Modrinth content, then launches with an offline identity and watches the log.
//! Usage: cargo run --example e2e -- <mc-version|latest> <loader> [modrinth-project ...]
//! Offline identity here is a test harness only; the app itself gates offline accounts behind a Microsoft sign-in.
use cairn_lib::instance::{self, Instance};
use cairn_lib::mc::{launch, version};
use cairn_lib::paths::Paths;
use cairn_lib::settings::Settings;
use cairn_lib::tasks::Task;
use cairn_lib::{game, modrinth};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths = Paths::new();
    let settings = Settings::load(&paths.settings_file());
    let mut mc = args.first().cloned().unwrap_or_else(|| "latest".into());
    if mc == "latest" {
        let m = version::manifest(&paths).await?;
        mc = m.latest["release"].as_str().unwrap().to_string();
    }
    let loader = args.get(1).cloned().unwrap_or_else(|| "vanilla".into());
    let name = format!("E2E {mc} {loader}");
    let existing = match std::env::var("E2E_INSTANCE") {
        Ok(id) => Some(instance::get(&paths, &id)?),
        Err(_) => instance::list(&paths).into_iter().find(|i| i.name == name),
    };
    let mut inst = match existing {
        Some(i) => i,
        None => instance::create(&paths, Instance { name: name.clone(), mc_version: mc.clone(), loader: loader.clone(), memory_mb: Some(2048), ..Default::default() })?,
    };
    eprintln!("instance {} at {}", inst.id, inst.dir(&paths).display());

    for pid in args.iter().skip(2) {
        let t = Task::headless(format!("install {pid}"));
        let r = modrinth::install_project(&paths, &inst, pid, None, &t).await?;
        for i in r {
            eprintln!("installed {} -> {}", i.title, i.file_name);
        }
    }

    let t0 = Instant::now();
    let task = Task::headless(format!("prepare {name}"));
    let (java, vid, installed) = game::prepare(&paths, &settings, &mut inst, &task).await?;
    eprintln!("prepared {vid} with {java} in {:?}", t0.elapsed());

    let auth = launch::LaunchAuth { name: "CairnTest".into(), uuid: launch::offline_uuid("CairnTest").replace('-', ""), access_token: "0".into(), user_type: "legacy".into(), xuid: "0".into() };
    let opts = launch::LaunchOptions {
        memory_mb: 2048,
        extra_jvm_args: vec![],
        width: Some(1280),
        height: Some(720),
        game_dir: inst.dir(&paths),
        natives_dir: installed.natives_dir.clone(),
        jar_id: installed.jar_id.clone(),
        quick_play_world: None,
        quick_play_server: None,
    };
    let a = launch::build_args(&paths, &installed.json, &auth, &opts);
    let mut child = tokio::process::Command::new(&java)
        .args(&a)
        .current_dir(inst.dir(&paths))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    tokio::spawn(async move {
        let mut l = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = l.next_line().await {
            println!("[stderr] {line}");
        }
    });
    let mut lines = BufReader::new(stdout).lines();
    let deadline = Instant::now() + Duration::from_secs(std::env::var("E2E_WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(240));
    let mut ready = false;
    loop {
        let next = tokio::time::timeout(Duration::from_secs(5), lines.next_line()).await;
        match next {
            Ok(Ok(Some(line))) => {
                println!("{line}");
                // The texture atlas is built once the title screen resources are loaded.
                if line.contains("Created:") && line.contains("minecraft:textures/atlas") || line.contains("Sound engine started") {
                    ready = true;
                }
            }
            Ok(_) => break,
            Err(_) => {}
        }
        if ready {
            break;
        }
        if Instant::now() > deadline {
            break;
        }
    }
    if let Ok(hold) = std::env::var("E2E_HOLD") {
        // Keep draining stdout so the game never blocks on a full pipe.
        let _ = tokio::time::timeout(Duration::from_secs(hold.parse().unwrap_or(20)), async {
            while let Ok(Some(line)) = lines.next_line().await {
                println!("{line}");
            }
        })
        .await;
    }
    let _ = child.kill().await;
    eprintln!("RESULT ready={ready} after {:?}", t0.elapsed());
    if !ready {
        std::process::exit(2);
    }
    Ok(())
}
