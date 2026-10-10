use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use sekiro_sim::brain::{
    BossActions, BossBrain, BrainReply, BrainRequest, Plan, RequestMode, Weighted,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{Backend, ConfigError, HostedConfig, protocol};

// ureq-proto Trace prints raw wire bytes regardless of a header's sensitive flag.
// Fail compilation if a future dependency change accidentally re-enables those traces.
const _: () = assert!(log::STATIC_MAX_LEVEL as usize <= log::LevelFilter::Debug as usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    Timeout,
    Http(u16),
    Transport,
    MalformedJson,
    Refusal,
    InvalidAnswer,
    Incomplete,
    UnknownAction,
    InvalidPlan,
    Serialization,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl TokenUsage {
    fn from_response(response: &Value) -> Self {
        Self {
            input_tokens: response["usage"]["input_tokens"].as_u64().unwrap_or(0),
            output_tokens: response["usage"]["output_tokens"].as_u64().unwrap_or(0),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HostedStats {
    pub requests: u64,
    pub answered: u64,
    pub failed: BTreeMap<Failure, u64>,
    pub mean_round_trip_ms: f64,
    pub max_round_trip_ms: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Logging failure does not change a valid answer into a gameplay failure.
    pub log_errors: u64,
}

#[derive(Clone, Default)]
pub struct StatsHandle {
    requests: Arc<AtomicU64>,
    completed: Arc<Mutex<HostedStats>>,
}

impl StatsHandle {
    pub fn snapshot(&self) -> HostedStats {
        let mut stats = self
            .completed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        stats.requests = self.requests.load(Ordering::Relaxed);
        stats
    }
}

struct Work {
    id: u64,
    input: Result<String, Failure>,
    excluded: Vec<String>,
}

pub struct HostedBrain {
    backend: Backend,
    tx: Option<mpsc::Sender<Work>>,
    rx: mpsc::Receiver<BrainReply>,
    worker: Option<JoinHandle<()>>,
    stopping: Arc<AtomicBool>,
    stats: StatsHandle,
}

impl HostedBrain {
    pub fn new(config: HostedConfig, actions: &'static BossActions) -> Result<Self, ConfigError> {
        if config.model.trim().is_empty()
            || config.api_key.0.trim().is_empty()
            || config.api_key.0.chars().any(char::is_control)
            || config.timeout.is_zero()
            || Instant::now().checked_add(config.timeout).is_none()
            || !(config.base_url.starts_with("https://") || config.base_url.starts_with("http://"))
            || actions.actions.is_empty()
            || actions.actions.len() > 253
            || actions.max_plan == 0
            || actions
                .actions
                .iter()
                .enumerate()
                .any(|(i, a)| actions.actions[..i].iter().any(|b| b.id == a.id))
        {
            return Err(ConfigError::InvalidConfig);
        }
        let log = match &config.log_path {
            Some(path) => {
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent).map_err(|_| ConfigError::LogOpen)?;
                }
                Some(
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .map_err(|_| ConfigError::LogOpen)?,
                )
            }
            None => None,
        };
        let backend = config.backend;
        let (tx, work) = mpsc::channel();
        let (reply, rx) = mpsc::channel();
        let stats = StatsHandle::default();
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stats = stats.clone();
        let worker_stopping = stopping.clone();
        let worker = thread::Builder::new()
            .name(format!("boss-{}", backend.name()))
            // TLS setup and the handshake use large stack frames; 2 MB overflows on Windows.
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                run(
                    config,
                    actions,
                    work,
                    reply,
                    worker_stats,
                    worker_stopping,
                    log,
                )
            })
            .map_err(|_| ConfigError::WorkerStart)?;
        Ok(Self {
            backend,
            tx: Some(tx),
            rx,
            worker: Some(worker),
            stopping,
            stats,
        })
    }

    pub fn stats(&self) -> HostedStats {
        self.stats.snapshot()
    }

    /// Retain before passing the controller to `Duel::set_brain` to inspect its telemetry.
    pub fn stats_handle(&self) -> StatsHandle {
        self.stats.clone()
    }
}

impl BossBrain for HostedBrain {
    fn name(&self) -> &'static str {
        self.backend.name()
    }

    /// A hosted model is asked at checkpoints and its answer, a plan set, is sampled locally.
    fn default_mode(&self) -> RequestMode {
        RequestMode::Checkpoints
    }

    fn usage(&self) -> (u64, u64) {
        let stats = self.stats.snapshot();
        (stats.input_tokens, stats.output_tokens)
    }

    fn submit(&mut self, request: &BrainRequest) {
        // Unbounded mpsc send never waits for the HTTP worker. No worker-held locks here.
        let input = serde_json::to_string(request).map_err(|_| Failure::Serialization);
        self.stats.requests.fetch_add(1, Ordering::Relaxed);
        if let Some(tx) = &self.tx {
            let _ = tx.send(Work {
                id: request.id,
                input,
                excluded: request.excluded.clone(),
            });
        }
    }

    fn poll(&mut self, _now: f32) -> Option<BrainReply> {
        self.rx.try_recv().ok()
    }
}

impl Drop for HostedBrain {
    fn drop(&mut self) {
        // Do not drain an arbitrary queue on shutdown. Finish only the bounded in-flight call.
        self.stopping.store(true, Ordering::Release);
        self.tx.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn http_failure(error: ureq::Error) -> Failure {
    match error {
        ureq::Error::Timeout(_) => Failure::Timeout,
        ureq::Error::StatusCode(code) => Failure::Http(code),
        _ => Failure::Transport,
    }
}

fn authorization(key: &crate::ApiKey) -> Result<ureq::http::HeaderValue, Failure> {
    let mut value = ureq::http::HeaderValue::from_str(&format!("Bearer {}", key.0))
        .map_err(|_| Failure::Transport)?;
    value.set_sensitive(true);
    Ok(value)
}

fn call(
    agent: &ureq::Agent,
    config: &HostedConfig,
    actions: &BossActions,
    input: &str,
    excluded: &[String],
    usage: &mut TokenUsage,
) -> Result<(Option<Plan>, Vec<Weighted>, String), Failure> {
    let url = format!(
        "{}/v1/{}",
        config.base_url.trim_end_matches('/'),
        config.backend.name()
    );
    // Raw transport errors and response bodies are deliberately never formatted or logged.
    let mut response = agent
        .post(&url)
        .header("Authorization", authorization(&config.api_key)?)
        .send_json(protocol::body(config, actions, input, excluded))
        .map_err(http_failure)?;
    let raw = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_string()
        .map_err(http_failure)?;
    let response: Value = serde_json::from_str(&raw).map_err(|_| Failure::MalformedJson)?;
    *usage = TokenUsage::from_response(&response);
    let value = protocol::choice(config.backend, &response)?;
    let plan = protocol::plan(&value, actions)?;
    let set = protocol::plan_set(config.backend, &response, actions);
    Ok((plan, set, value))
}

/// Warms the route to the host before the first decision: resolves its name and opens a TCP
/// connection, then closes it. Nothing is sent, so no API request is made and no key leaves the
/// machine. (ureq has no public way to pool a connection it did not open itself, so the TLS
/// handshake still happens on the first real request; this takes the DNS lookup and the first
/// route set-up out of it.)
fn warm_up(base_url: &str, timeout: std::time::Duration) {
    use std::net::ToSocketAddrs;
    let rest = base_url.split_once("//").map_or(base_url, |(_, r)| r);
    let authority = rest.split(['/', '?']).next().unwrap_or(rest);
    let port = if base_url.starts_with("https://") {
        443
    } else {
        80
    };
    let target = if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:{port}")
    };
    if let Some(addr) = target.to_socket_addrs().ok().and_then(|mut a| a.next()) {
        let _ = std::net::TcpStream::connect_timeout(&addr, timeout);
    }
}

fn run(
    config: HostedConfig,
    actions: &'static BossActions,
    work: mpsc::Receiver<Work>,
    reply: mpsc::Sender<BrainReply>,
    stats: StatsHandle,
    stopping: Arc<AtomicBool>,
    mut log: Option<File>,
) {
    // Keep one pooled rustls agent for the entire fight. Disable redirects (no credential
    // forwarding or extra provider calls) and environment proxies (loopback tests stay local).
    if config.warm_up {
        warm_up(&config.base_url, config.timeout);
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(config.timeout))
        .max_redirects(0)
        .proxy(None)
        .build()
        .into();
    let mut total_ms = 0.0;
    let mut completed = 0_u64;
    for work in work {
        if stopping.load(Ordering::Acquire) {
            break;
        }
        let start = Instant::now();
        let mut usage = TokenUsage::default();
        let result = work
            .input
            .and_then(|input| call(&agent, &config, actions, &input, &work.excluded, &mut usage));
        let round_trip_ms = start.elapsed().as_secs_f64() * 1000.0;
        let (plan, set, chosen, failure) = match result {
            Ok((plan, set, value)) => (plan, set, Some(value), None),
            Err(reason) => (None, Vec::new(), None, Some(reason)),
        };
        let line = json!({
            "id": work.id, "backend": config.backend.name(), "model": config.redact(&config.model),
            "round_trip_ms": round_trip_ms, "status": if failure.is_some() {"failed"} else {"answered"},
            "failure": failure, "chosen": chosen.map(|s| config.redact(&s)), "usage": usage
        });
        let log_failed = log.as_mut().is_some_and(|file| {
            serde_json::to_writer(&mut *file, &line).is_err()
                || file.write_all(b"\n").is_err()
                || file.flush().is_err()
        });
        if log_failed {
            log = None;
        }
        completed += 1;
        total_ms += round_trip_ms;
        {
            let mut stats = stats.completed.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(reason) = failure {
                *stats.failed.entry(reason).or_default() += 1;
            } else {
                stats.answered += 1;
            }
            stats.mean_round_trip_ms = total_ms / completed as f64;
            stats.max_round_trip_ms = stats.max_round_trip_ms.max(round_trip_ms);
            stats.input_tokens += usage.input_tokens;
            stats.output_tokens += usage.output_tokens;
            stats.log_errors += u64::from(log_failed);
        }
        if reply
            .send(BrainReply {
                id: work.id,
                plan,
                set,
            })
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_debug_is_sensitive() {
        let key = crate::ApiKey::new("test-key-not-real");
        let value = authorization(&key).unwrap();
        assert!(value.is_sensitive());
        assert!(!format!("{value:?}").contains(&key.0));
    }
}
