use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Progress for one stage is sent at most this often. Asset downloads report thousands of
/// files, and every event re-renders the task tray.
const MIN_GAP: Duration = Duration::from_millis(120);

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
    last: Arc<Mutex<Option<(Instant, String)>>>,
}

/// Whether a progress update is worth sending: always for a new stage, the first or last
/// step, otherwise once per `MIN_GAP`.
fn due(last: &Option<(Instant, String)>, stage: &str, done: u64, total: u64, now: Instant) -> bool {
    match last {
        None => true,
        Some((at, s)) => s != stage || done == 0 || done >= total || now.duration_since(*at) >= MIN_GAP,
    }
}

impl Task {
    pub fn new(app: &AppHandle, title: impl Into<String>) -> Self {
        let t = Task { app: Some(app.clone()), id: uuid::Uuid::new_v4().to_string(), title: title.into(), last: Arc::default() };
        t.stage("Starting", 0, 0);
        t
    }
    /// A task that logs to stderr instead of the UI (tests and the CLI harness).
    pub fn headless(title: impl Into<String>) -> Self {
        Task { app: None, id: uuid::Uuid::new_v4().to_string(), title: title.into(), last: Arc::default() }
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
        let now = Instant::now();
        {
            let Ok(mut last) = self.last.lock() else { return };
            if !due(&last, stage, done, total, now) {
                return;
            }
            *last = Some((now, stage.to_string()));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_throttled_within_a_stage() {
        let t0 = Instant::now();
        let last = Some((t0, "Downloading".to_string()));
        assert!(due(&None, "Downloading", 5, 10, t0));
        assert!(!due(&last, "Downloading", 5, 10, t0 + Duration::from_millis(10)));
        assert!(due(&last, "Downloading", 6, 10, t0 + MIN_GAP));
        assert!(due(&last, "Downloading", 10, 10, t0), "the last step always shows");
        assert!(due(&last, "Unpacking", 1, 10, t0), "a new stage always shows");
        assert!(due(&last, "Downloading", 0, 0, t0), "indeterminate stages always show");
    }
}
