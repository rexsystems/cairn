use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Progress for long jobs (installs, downloads), shown in the task tray.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
    pub id: String,
    pub title: String,
    pub stage: String,
    pub done: u64,
    pub total: u64,
    pub state: String, // running | done | error
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct Task {
    app: Option<AppHandle>,
    pub id: String,
    pub title: String,
}

impl Task {
    pub fn new(app: &AppHandle, title: impl Into<String>) -> Self {
        let t = Task { app: Some(app.clone()), id: uuid::Uuid::new_v4().to_string(), title: title.into() };
        t.stage("Starting", 0, 0);
        t
    }
    /// A task that logs to stderr instead of the UI (tests and the CLI harness).
    pub fn headless(title: impl Into<String>) -> Self {
        Task { app: None, id: uuid::Uuid::new_v4().to_string(), title: title.into() }
    }
    fn emit(&self, stage: &str, done: u64, total: u64, state: &str, error: Option<String>) {
        let ev = TaskEvent { id: self.id.clone(), title: self.title.clone(), stage: stage.into(), done, total, state: state.into(), error };
        match &self.app {
            Some(app) => {
                let _ = app.emit("task", ev);
            }
            None => eprintln!("[task] {} | {} {}/{} {}", ev.title, ev.stage, ev.done, ev.total, ev.error.unwrap_or_default()),
        }
    }
    pub fn stage(&self, stage: &str, done: u64, total: u64) {
        self.emit(stage, done, total, "running", None);
    }
    pub fn finish(&self) {
        self.emit("Finished", 1, 1, "done", None);
    }
    pub fn fail(&self, err: &anyhow::Error) {
        self.emit("Failed", 0, 0, "error", Some(format!("{err:#}")));
    }
    /// Runs a job and reports its outcome to the task tray.
    pub async fn run<T, F>(self, fut: F) -> anyhow::Result<T>
    where
        F: std::future::Future<Output = anyhow::Result<T>>,
    {
        let r = fut.await;
        match &r {
            Ok(_) => self.finish(),
            Err(e) => self.fail(e),
        }
        r
    }
}
