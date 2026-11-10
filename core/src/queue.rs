use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;

use crate::error::UserError;
use crate::model::{DownloadRequest, JobState, Progress};
use crate::runner::{JobRunner, RunError, RunEvent};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: String,
    pub request: DownloadRequest,
    pub state: JobState,
    pub progress: Option<Progress>,
    pub file_path: Option<String>,
    pub error: Option<UserError>,
}

struct Job {
    snap: JobSnapshot,
    cancel: Option<CancellationToken>,
    /// Identifies the current run so a stale worker cannot overwrite a newer one.
    run_id: u64,
}

struct State {
    jobs: Vec<Job>,
    max_parallel: usize,
    next_id: u64,
    next_run: u64,
}

type OnChange = Arc<dyn Fn(JobSnapshot) + Send + Sync>;

struct Inner {
    runner: Arc<dyn JobRunner>,
    on_change: OnChange,
    state: Mutex<State>,
    handle: Handle,
}

/// Download queue: runs up to `max_parallel` jobs at once and reports every
/// change through `on_change`. Must be created inside a tokio runtime.
#[derive(Clone)]
pub struct Queue {
    inner: Arc<Inner>,
}

fn is_running(s: JobState) -> bool {
    matches!(s, JobState::Downloading | JobState::Processing)
}

/// Run `f` under the lock, then deliver the snapshots it collected after the
/// lock is released (callbacks may call back into the queue).
fn with<R>(inner: &Inner, f: impl FnOnce(&mut State, &mut Vec<JobSnapshot>) -> R) -> R {
    let mut changes = Vec::new();
    let result = {
        let mut st = inner.state.lock().unwrap();
        f(&mut st, &mut changes)
    };
    for c in changes {
        (inner.on_change)(c);
    }
    result
}

fn schedule(inner: &Arc<Inner>) {
    let starts = with(inner, |st, changes| {
        let State { jobs, max_parallel, next_run, .. } = st;
        let mut running = jobs.iter().filter(|j| is_running(j.snap.state)).count();
        let mut starts = Vec::new();
        for job in jobs.iter_mut() {
            if running >= *max_parallel {
                break;
            }
            if job.snap.state == JobState::Queued {
                *next_run += 1;
                let token = CancellationToken::new();
                job.run_id = *next_run;
                job.cancel = Some(token.clone());
                job.snap.state = JobState::Downloading;
                running += 1;
                changes.push(job.snap.clone());
                starts.push((job.snap.id.clone(), job.run_id, job.snap.request.clone(), token));
            }
        }
        starts
    });
    for (id, run_id, req, token) in starts {
        let inner2 = inner.clone();
        inner.handle.spawn(worker(inner2, id, run_id, req, token));
    }
}

async fn worker(inner: Arc<Inner>, id: String, run_id: u64, req: DownloadRequest, token: CancellationToken) {
    let on_event = {
        let (inner, id) = (inner.clone(), id.clone());
        move |event: RunEvent| {
            with(&inner, |st, changes| {
                let Some(job) = st.jobs.iter_mut().find(|j| j.snap.id == id && j.run_id == run_id) else { return };
                if !is_running(job.snap.state) {
                    return;
                }
                match event {
                    RunEvent::Progress(p) => job.snap.progress = Some(p),
                    RunEvent::Processing => job.snap.state = JobState::Processing,
                    RunEvent::File(f) => job.snap.file_path = Some(f.to_string_lossy().into_owned()),
                }
                changes.push(job.snap.clone());
            });
        }
    };
    let result = inner.runner.run(&req, &on_event, token).await;
    with(&inner, |st, changes| {
        let Some(job) = st.jobs.iter_mut().find(|j| j.snap.id == id && j.run_id == run_id) else { return };
        match result {
            Ok(path) => {
                job.snap.state = JobState::Done;
                job.snap.file_path = Some(path.to_string_lossy().into_owned());
                job.snap.error = None;
            }
            Err(RunError::Cancelled) => {}
            Err(RunError::Failed(e)) => {
                if is_running(job.snap.state) {
                    job.snap.state = JobState::Failed;
                    job.snap.error = Some(e);
                }
            }
        }
        job.cancel = None;
        changes.push(job.snap.clone());
    });
    schedule(&inner);
}

impl Queue {
    pub fn new(runner: Arc<dyn JobRunner>, max_parallel: usize, on_change: OnChange) -> Queue {
        Queue {
            inner: Arc::new(Inner {
                runner,
                on_change,
                state: Mutex::new(State { jobs: Vec::new(), max_parallel: max_parallel.max(1), next_id: 0, next_run: 0 }),
                handle: Handle::current(),
            }),
        }
    }

    pub fn add(&self, req: DownloadRequest) -> String {
        let id = with(&self.inner, |st, changes| {
            st.next_id += 1;
            let id = format!("job-{}", st.next_id);
            let snap = JobSnapshot {
                id: id.clone(),
                request: req,
                state: JobState::Queued,
                progress: None,
                file_path: None,
                error: None,
            };
            changes.push(snap.clone());
            st.jobs.push(Job { snap, cancel: None, run_id: 0 });
            id
        });
        schedule(&self.inner);
        id
    }

    fn update(&self, id: &str, f: impl FnOnce(&mut Job)) {
        with(&self.inner, |st, changes| {
            if let Some(job) = st.jobs.iter_mut().find(|j| j.snap.id == id) {
                f(job);
                changes.push(job.snap.clone());
            }
        });
        schedule(&self.inner);
    }

    /// Stop the running process but keep the partial file; `resume` continues it.
    pub fn pause(&self, id: &str) {
        self.update(id, |job| {
            if matches!(job.snap.state, JobState::Queued | JobState::Downloading | JobState::Processing) {
                if let Some(t) = job.cancel.take() {
                    t.cancel();
                }
                job.snap.state = JobState::Paused;
            }
        });
    }

    /// Like `pause`, but the job also leaves the paused list: final until resumed.
    pub fn stop(&self, id: &str) {
        self.update(id, |job| {
            if matches!(
                job.snap.state,
                JobState::Queued | JobState::Downloading | JobState::Processing | JobState::Paused
            ) {
                if let Some(t) = job.cancel.take() {
                    t.cancel();
                }
                job.snap.state = JobState::Stopped;
            }
        });
    }

    pub fn resume(&self, id: &str) {
        self.update(id, |job| {
            if matches!(job.snap.state, JobState::Paused | JobState::Stopped | JobState::Failed) {
                job.snap.state = JobState::Queued;
                job.snap.error = None;
            }
        });
    }

    pub fn remove(&self, id: &str) {
        with(&self.inner, |st, _| {
            if let Some(pos) = st.jobs.iter().position(|j| j.snap.id == id) {
                if let Some(t) = st.jobs[pos].cancel.take() {
                    t.cancel();
                }
                st.jobs.remove(pos);
            }
        });
        schedule(&self.inner);
    }

    /// Stop every unfinished job (used when the app quits). Returns how many were running.
    pub fn stop_all(&self) -> usize {
        with(&self.inner, |st, changes| {
            let mut running = 0;
            for job in st.jobs.iter_mut() {
                if matches!(
                    job.snap.state,
                    JobState::Queued | JobState::Downloading | JobState::Processing | JobState::Paused
                ) {
                    if is_running(job.snap.state) {
                        running += 1;
                    }
                    if let Some(t) = job.cancel.take() {
                        t.cancel();
                    }
                    job.snap.state = JobState::Stopped;
                    changes.push(job.snap.clone());
                }
            }
            running
        })
    }

    pub fn list(&self) -> Vec<JobSnapshot> {
        self.inner.state.lock().unwrap().jobs.iter().map(|j| j.snap.clone()).collect()
    }

    pub fn set_max_parallel(&self, n: usize) {
        self.inner.state.lock().unwrap().max_parallel = n.max(1);
        schedule(&self.inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::model::{AudioOnlyFormat, AudioSelect, Progress, VideoSelect};
    use crate::runner::{JobRunner, RunError, RunEvent};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::Notify;
    use tokio_util::sync::CancellationToken;

    /// Runner that blocks until the test calls `release(url)`.
    #[derive(Default)]
    struct FakeRunner {
        started: Mutex<Vec<String>>,
        cancelled: Mutex<Vec<String>>,
        gates: Mutex<HashMap<String, Arc<Notify>>>,
    }

    impl FakeRunner {
        fn gate(&self, url: &str) -> Arc<Notify> {
            self.gates.lock().unwrap().entry(url.to_string()).or_default().clone()
        }
        fn release(&self, url: &str) {
            self.gate(url).notify_one();
        }
        fn starts(&self, url: &str) -> usize {
            self.started.lock().unwrap().iter().filter(|u| *u == url).count()
        }
    }

    #[async_trait]
    impl JobRunner for FakeRunner {
        async fn run(
            &self,
            req: &DownloadRequest,
            on_event: &(dyn Fn(RunEvent) + Send + Sync),
            cancel: CancellationToken,
        ) -> Result<PathBuf, RunError> {
            self.started.lock().unwrap().push(req.url.clone());
            on_event(RunEvent::Progress(Progress { downloaded: 5, total: Some(10), speed_bps: None, eta_secs: None }));
            let gate = self.gate(&req.url);
            tokio::select! {
                _ = cancel.cancelled() => {
                    self.cancelled.lock().unwrap().push(req.url.clone());
                    Err(RunError::Cancelled)
                }
                _ = gate.notified() => {
                    if req.url.contains("fail") {
                        Err(RunError::Failed(UserError::new(ErrorKind::Network, "offline")))
                    } else {
                        Ok(PathBuf::from(format!("/out/{}.mp4", req.url)))
                    }
                }
            }
        }
    }

    fn req(url: &str) -> DownloadRequest {
        DownloadRequest {
            url: url.into(),
            title: url.into(),
            thumbnail: None,
            video: VideoSelect::Best,
            audio: AudioSelect::Best,
            audio_only_format: AudioOnlyFormat::Mp3,
        }
    }

    type Log = Arc<Mutex<Vec<(String, JobState)>>>;

    fn setup(max: usize) -> (Queue, Arc<FakeRunner>, Log) {
        let runner = Arc::new(FakeRunner::default());
        let log: Log = Arc::default();
        let sink = log.clone();
        let q = Queue::new(runner.clone(), max, Arc::new(move |s: JobSnapshot| {
            sink.lock().unwrap().push((s.id.clone(), s.state));
        }));
        (q, runner, log)
    }

    async fn wait_for(q: &Queue, id: &str, state: JobState) -> JobSnapshot {
        for _ in 0..200 {
            if let Some(s) = q.list().into_iter().find(|s| s.id == id) {
                if s.state == state {
                    return s;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("job {id} never reached {state:?}; have {:?}", q.list().iter().map(|s| (&s.id, s.state)).collect::<Vec<_>>());
    }

    fn state_of(q: &Queue, id: &str) -> JobState {
        q.list().into_iter().find(|s| s.id == id).unwrap().state
    }

    #[tokio::test]
    async fn respects_max_parallel() {
        let (q, runner, _) = setup(2);
        let (a, b, c) = (q.add(req("a")), q.add(req("b")), q.add(req("c")));
        wait_for(&q, &a, JobState::Downloading).await;
        wait_for(&q, &b, JobState::Downloading).await;
        assert_eq!(state_of(&q, &c), JobState::Queued);
        runner.release("a");
        wait_for(&q, &a, JobState::Done).await;
        wait_for(&q, &c, JobState::Downloading).await;
    }

    #[tokio::test]
    async fn done_has_file_path_and_progress_is_recorded() {
        let (q, runner, _) = setup(1);
        let a = q.add(req("a"));
        let running = wait_for(&q, &a, JobState::Downloading).await;
        for _ in 0..100 {
            if q.list()[0].progress.is_some() { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(q.list()[0].progress.as_ref().unwrap().downloaded, 5, "{running:?}");
        runner.release("a");
        let done = wait_for(&q, &a, JobState::Done).await;
        assert_eq!(done.file_path.as_deref(), Some("/out/a.mp4"));
    }

    #[tokio::test]
    async fn pause_then_resume_reruns() {
        let (q, runner, _) = setup(1);
        let a = q.add(req("a"));
        wait_for(&q, &a, JobState::Downloading).await;
        q.pause(&a);
        wait_for(&q, &a, JobState::Paused).await;
        q.resume(&a);
        wait_for(&q, &a, JobState::Downloading).await;
        for _ in 0..100 {
            if runner.starts("a") == 2 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(runner.starts("a"), 2, "resume must start a second run");
        runner.release("a");
        wait_for(&q, &a, JobState::Done).await;
    }

    #[tokio::test]
    async fn pause_frees_the_slot_and_paused_queued_job_does_not_start() {
        let (q, runner, _) = setup(1);
        let (a, b, c) = (q.add(req("a")), q.add(req("b")), q.add(req("c")));
        wait_for(&q, &a, JobState::Downloading).await;
        q.pause(&c);
        q.pause(&a);
        wait_for(&q, &b, JobState::Downloading).await;
        runner.release("b");
        wait_for(&q, &b, JobState::Done).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(state_of(&q, &c), JobState::Paused);
        assert_eq!(runner.starts("c"), 0);
    }

    #[tokio::test]
    async fn stop_is_terminal_until_resume() {
        let (q, runner, _) = setup(1);
        let a = q.add(req("a"));
        wait_for(&q, &a, JobState::Downloading).await;
        q.stop(&a);
        wait_for(&q, &a, JobState::Stopped).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(state_of(&q, &a), JobState::Stopped);
        q.resume(&a);
        wait_for(&q, &a, JobState::Downloading).await;
        runner.release("a");
        wait_for(&q, &a, JobState::Done).await;
    }

    #[tokio::test]
    async fn failure_sets_failed_with_error_and_can_retry() {
        let (q, runner, _) = setup(1);
        let a = q.add(req("fail-a"));
        wait_for(&q, &a, JobState::Downloading).await;
        runner.release("fail-a");
        let failed = wait_for(&q, &a, JobState::Failed).await;
        assert_eq!(failed.error.unwrap().kind, ErrorKind::Network);
        q.resume(&a);
        wait_for(&q, &a, JobState::Downloading).await;
        assert!(q.list()[0].error.is_none());
    }

    #[tokio::test]
    async fn remove_cancels_running_job() {
        let (q, runner, _) = setup(1);
        let a = q.add(req("a"));
        wait_for(&q, &a, JobState::Downloading).await;
        q.remove(&a);
        assert!(q.list().is_empty());
        for _ in 0..100 {
            if !runner.cancelled.lock().unwrap().is_empty() { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(runner.cancelled.lock().unwrap().as_slice(), ["a"]);
    }

    #[tokio::test]
    async fn stop_all_cancels_running_jobs_and_reports_how_many() {
        let (q, runner, _) = setup(2);
        let (a, b, c) = (q.add(req("a")), q.add(req("b")), q.add(req("c")));
        wait_for(&q, &a, JobState::Downloading).await;
        wait_for(&q, &b, JobState::Downloading).await;
        assert_eq!(q.stop_all(), 2);
        for id in [&a, &b, &c] {
            wait_for(&q, id, JobState::Stopped).await;
        }
        for _ in 0..100 {
            if runner.cancelled.lock().unwrap().len() == 2 { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(runner.cancelled.lock().unwrap().len(), 2);
        assert_eq!(q.stop_all(), 0);
    }

    #[tokio::test]
    async fn set_max_parallel_starts_waiting_jobs() {
        let (q, _runner, _) = setup(1);
        let (a, b) = (q.add(req("a")), q.add(req("b")));
        wait_for(&q, &a, JobState::Downloading).await;
        assert_eq!(state_of(&q, &b), JobState::Queued);
        q.set_max_parallel(2);
        wait_for(&q, &b, JobState::Downloading).await;
    }

    #[tokio::test]
    async fn on_change_reports_every_state() {
        let (q, runner, log) = setup(1);
        let a = q.add(req("a"));
        wait_for(&q, &a, JobState::Downloading).await;
        runner.release("a");
        wait_for(&q, &a, JobState::Done).await;
        let states: Vec<JobState> = log.lock().unwrap().iter().map(|(_, s)| *s).collect();
        let first_of = |s: JobState| states.iter().position(|x| *x == s).unwrap();
        assert!(first_of(JobState::Queued) < first_of(JobState::Downloading));
        assert!(first_of(JobState::Downloading) < first_of(JobState::Done));
    }
}
