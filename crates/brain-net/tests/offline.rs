use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sekiro_brain_net::{ApiKey, Backend, ConfigError, Failure, HostedBrain, HostedConfig};
use sekiro_sim::brain::{BossBrain, BrainReply, BrainRequest, OWL, Observation, TallySummary};
use serde_json::{Value, json};

const KEY: &str = "test-key-not-real";

struct TestLogger(Mutex<Vec<String>>);
static LOGGER: TestLogger = TestLogger(Mutex::new(Vec::new()));

impl log::Log for TestLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }
    fn log(&self, record: &log::Record<'_>) {
        self.0.lock().unwrap().push(record.args().to_string());
    }
    fn flush(&self) {}
}

struct Response {
    status: u16,
    body: String,
    delay: Duration,
    headers_first: bool,
}

impl Response {
    fn json(body: Value) -> Self {
        Self {
            status: 200,
            body: body.to_string(),
            delay: Duration::ZERO,
            headers_first: false,
        }
    }
}

struct Mock {
    base: String,
    requests: mpsc::Receiver<Value>,
    accepted: Arc<AtomicUsize>,
    started: mpsc::Receiver<()>,
    worker: Option<JoinHandle<()>>,
}

impl Mock {
    fn new(backend: Backend, responses: Vec<Response>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (tx, requests) = mpsc::channel();
        let (start_tx, started) = mpsc::channel();
        let accepted = Arc::new(AtomicUsize::new(0));
        let count = accepted.clone();
        // A read timeout lets the mock accept a replacement if the client closes a connection.
        let worker = thread::spawn(move || {
            let mut connection: Option<BufReader<TcpStream>> = None;
            for response in responses {
                let mut first = String::new();
                loop {
                    if connection.is_none() {
                        listener.set_nonblocking(true).unwrap();
                        let deadline = Instant::now() + Duration::from_secs(3);
                        let stream = loop {
                            match listener.accept() {
                                Ok((stream, _)) => break stream,
                                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    assert!(
                                        Instant::now() < deadline,
                                        "mock accept deadline exceeded"
                                    );
                                    thread::sleep(Duration::from_millis(1));
                                }
                                Err(_) => panic!("mock accept failed"),
                            }
                        };
                        count.fetch_add(1, Ordering::Relaxed);
                        // Windows accepted sockets inherit a nonblocking listener's mode.
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        connection = Some(BufReader::new(stream));
                    }
                    first.clear();
                    match connection.as_mut().unwrap().read_line(&mut first) {
                        Ok(n) if n > 0 => break,
                        _ => connection = None,
                    }
                }
                assert_eq!(
                    first.trim(),
                    format!("POST /v1/{} HTTP/1.1", backend.name())
                );
                let reader = connection.as_mut().unwrap();
                let mut length = None;
                let mut authorized = false;
                let mut content_type = false;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" {
                        break;
                    }
                    let (name, value) = header.split_once(':').unwrap();
                    match name.to_ascii_lowercase().as_str() {
                        "content-length" => length = Some(value.trim().parse::<usize>().unwrap()),
                        // Do not put the received credential into an assertion diagnostic.
                        "authorization" => authorized = value.trim() == format!("Bearer {KEY}"),
                        "content-type" => {
                            content_type =
                                value.trim().split(';').next() == Some("application/json")
                        }
                        _ => (),
                    }
                }
                assert!(authorized, "mock authorization mismatch");
                assert!(content_type, "mock content type mismatch");
                let mut body = vec![0; length.unwrap()];
                reader.read_exact(&mut body).unwrap();
                tx.send(serde_json::from_slice(&body).unwrap()).unwrap();
                let _ = start_tx.send(());
                let message = format!(
                    "HTTP/1.1 {} Mock\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{}",
                    response.status,
                    response.body.len(),
                    response.body
                );
                // Timeouts and drop intentionally close the other end before the mock writes.
                if response.headers_first {
                    let (headers, body) = message.split_once("\r\n\r\n").unwrap();
                    let _ = reader.get_mut().write_all(headers.as_bytes());
                    let _ = reader.get_mut().write_all(b"\r\n\r\n");
                    let _ = reader.get_mut().flush();
                    thread::sleep(response.delay);
                    let _ = reader.get_mut().write_all(body.as_bytes());
                } else {
                    thread::sleep(response.delay);
                    let _ = reader.get_mut().write_all(message.as_bytes());
                }
                let _ = reader.get_mut().flush();
            }
        });
        Self {
            base,
            requests,
            accepted,
            started,
            worker: Some(worker),
        }
    }

    fn config(&self, backend: Backend) -> HostedConfig {
        let mut config = HostedConfig::new(backend, ApiKey::new(KEY));
        config.base_url = self.base.clone();
        config
    }

    fn finish(&mut self) {
        self.worker.take().unwrap().join().unwrap();
    }
}

fn request(id: u64) -> BrainRequest {
    BrainRequest {
        id,
        obs: Observation::default(),
        tally: TallySummary::default(),
    }
}

fn poll(brain: &mut HostedBrain) -> BrainReply {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(reply) = brain.poll(0.0) {
            return reply;
        }
        assert!(Instant::now() < deadline, "reply deadline exceeded");
        thread::sleep(Duration::from_millis(1));
    }
}

fn decisions(value: &str, input: u64, output: u64) -> Value {
    json!({"model": "gpt-6-luna", "answers": [{"name": "owl_action", "type": "choice",
        "choice": value, "confidence": 1.0, "probabilities": [{"value": value, "probability": 1.0}]}],
        "usage": {"input_tokens": input, "output_tokens": output}})
}

fn responses(value: &str) -> Value {
    json!({"model": "gpt-6-luna", "status": "completed", "output": [
        {"type": "message", "role": "assistant", "content": [
            {"type": "output_text", "text": json!({"action": value}).to_string()}]}],
        "usage": {"input_tokens": 12, "output_tokens": 4}})
}

fn check_vocabulary(values: &[String]) {
    let mut expected: Vec<_> = OWL.actions.iter().map(|a| a.id.to_string()).collect();
    expected.extend(["3000+3001+3002".into(), "lua".into()]);
    assert_eq!(values, expected);
}

#[test]
fn decisions_happy_path_lua_usage_log_and_keep_alive() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    let mut mock = Mock::new(
        Backend::Decisions,
        vec![
            Response::json(decisions("3005", 11, 2)),
            Response::json(decisions("lua", 20, 3)),
        ],
    );
    let mut config = mock.config(Backend::Decisions);
    let path = std::env::temp_dir().join(format!(
        "brain-net-{}-{}.jsonl",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    config.log_path = Some(path.clone());
    assert!(!format!("{config:?}").contains(KEY));
    let mut brain = HostedBrain::new(config, &OWL).unwrap();
    let handle = brain.stats_handle();
    assert_eq!(brain.name(), "decisions");
    brain.submit(&request(41));
    let reply = poll(&mut brain);
    assert_eq!(reply.id, 41);
    assert_eq!(reply.plan.unwrap().actions, [3005]);
    assert!(brain.poll(0.0).is_none());
    brain.submit(&request(42));
    let reply = poll(&mut brain);
    assert_eq!(reply.id, 42);
    assert!(reply.plan.is_none());
    mock.finish();
    let diagnostics = LOGGER.0.lock().unwrap();
    assert!(
        diagnostics.iter().all(|line| !line.contains(KEY)),
        "HTTP diagnostics must redact the key"
    );
    assert!(
        log::STATIC_MAX_LEVEL <= log::LevelFilter::Debug,
        "ureq-proto raw wire traces must be compiled out"
    );
    drop(diagnostics);
    assert_eq!(
        mock.accepted.load(Ordering::Relaxed),
        1,
        "agent must reuse its TCP connection"
    );
    for id in [41, 42] {
        let body = mock.requests.recv().unwrap();
        assert_eq!(body["model"], "gpt-6-luna");
        let input: Value = serde_json::from_str(body["input"].as_str().unwrap()).unwrap();
        assert_eq!(input["id"], id);
        assert_eq!(body["input"], serde_json::to_string(&request(id)).unwrap());
        let questions = body["questions"].as_array().unwrap();
        assert_eq!(questions.len(), 1);
        let q = &questions[0];
        assert_eq!(q["type"], "choice");
        assert_eq!(q["name"], "owl_action");
        assert!(q["instructions"].as_str().unwrap().contains("Reflexes"));
        let choices = q["choices"].as_array().unwrap();
        check_vocabulary(
            &choices
                .iter()
                .map(|v| v["value"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>(),
        );
        for (choice, action) in choices.iter().zip(OWL.actions) {
            assert_eq!(choice["description"], action.label);
        }
    }
    let stats = brain.stats();
    assert_eq!(stats.requests, 2);
    assert_eq!(stats.answered, 2);
    assert!(stats.failed.is_empty());
    assert_eq!((stats.input_tokens, stats.output_tokens), (31, 5));
    assert!(stats.mean_round_trip_ms > 0.0);
    assert!(stats.max_round_trip_ms >= stats.mean_round_trip_ms);
    assert!(!format!("{stats:?}").contains(KEY));
    let log = std::fs::read_to_string(&path).unwrap();
    assert!(!log.contains(KEY));
    assert!(!log.to_ascii_lowercase().contains("authorization"));
    let lines: Vec<Value> = log
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["chosen"], "3005");
    assert_eq!(lines[1]["chosen"], "lua");
    assert_eq!(lines[1]["status"], "answered");
    assert_eq!(lines[0]["usage"]["input_tokens"], 11);
    drop(brain);
    assert_eq!(handle.snapshot().answered, 2);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn responses_happy_path_combo_and_exact_schema() {
    let mut mock = Mock::new(
        Backend::Responses,
        vec![Response::json(responses("3000+3001+3002"))],
    );
    let mut config = mock.config(Backend::Responses);
    config.model = "test-model-override".into();
    let mut brain = HostedBrain::new(config, &OWL).unwrap();
    assert_eq!(brain.name(), "responses");
    brain.submit(&request(9));
    let reply = poll(&mut brain);
    assert_eq!(reply.id, 9);
    assert_eq!(reply.plan.unwrap().actions, [3000, 3001, 3002]);
    mock.finish();
    let body = mock.requests.recv().unwrap();
    assert_eq!(body["model"], "test-model-override");
    assert_eq!(body["reasoning"], json!({"effort": "none"}));
    assert_eq!(body["max_output_tokens"], 16);
    assert_eq!(body["store"], false);
    assert_eq!(
        body["input"],
        json!([{"role":"user", "content": serde_json::to_string(&request(9)).unwrap()}])
    );
    let format = &body["text"]["format"];
    assert_eq!(format["type"], "json_schema");
    assert_eq!(format["name"], "owl_action");
    assert_eq!(format["strict"], true);
    assert_eq!(format["schema"]["type"], "object");
    assert_eq!(format["schema"]["properties"]["action"]["type"], "string");
    assert_eq!(format["schema"]["required"], json!(["action"]));
    assert_eq!(format["schema"]["additionalProperties"], false);
    check_vocabulary(
        &format["schema"]["properties"]["action"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        (brain.stats().input_tokens, brain.stats().output_tokens),
        (12, 4)
    );
}

#[test]
fn failures_fall_back_with_specific_reasons_and_never_echo_secrets() {
    let cases = vec![
        (
            Backend::Decisions,
            Response::json(
                json!({"answers": [{"type": "refusal", "name": "owl_action"}], "usage":{"input_tokens":7}}),
            ),
            Failure::Refusal,
        ),
        (
            Backend::Responses,
            Response::json(
                json!({"status": "completed", "output": [{"type": "message", "role": "assistant", "content": [{"type": "refusal", "refusal": KEY}]}]}),
            ),
            Failure::Refusal,
        ),
        (
            Backend::Decisions,
            Response {
                status: 429,
                body: KEY.into(),
                delay: Duration::ZERO,
                headers_first: false,
            },
            Failure::Http(429),
        ),
        (
            Backend::Responses,
            Response {
                status: 500,
                body: KEY.into(),
                delay: Duration::ZERO,
                headers_first: false,
            },
            Failure::Http(500),
        ),
        (
            Backend::Decisions,
            Response {
                status: 200,
                body: KEY.into(),
                delay: Duration::ZERO,
                headers_first: false,
            },
            Failure::MalformedJson,
        ),
        (
            Backend::Responses,
            Response::json(responses("9999")),
            Failure::UnknownAction,
        ),
        (
            Backend::Decisions,
            Response::json(decisions(KEY, 1, 2)),
            Failure::UnknownAction,
        ),
        (
            Backend::Responses,
            Response::json(json!({"status":"incomplete", "output":[]})),
            Failure::Incomplete,
        ),
        (
            Backend::Decisions,
            Response::json(json!({"answers":[]})),
            Failure::InvalidAnswer,
        ),
        (
            Backend::Responses,
            Response::json(
                json!({"status":"completed", "output":[{"type":"message", "role":"assistant", "content":[{"type":"output_text", "text":KEY}]}]}),
            ),
            Failure::InvalidAnswer,
        ),
        (
            Backend::Decisions,
            Response::json(decisions("3000+3001+3002+3000", 0, 0)),
            Failure::InvalidPlan,
        ),
    ];
    for (backend, response, reason) in cases {
        let mut mock = Mock::new(backend, vec![response]);
        let path = std::env::temp_dir().join(format!(
            "brain-net-failure-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut config = mock.config(backend);
        config.log_path = Some(path.clone());
        let mut brain = HostedBrain::new(config, &OWL).unwrap();
        brain.submit(&request(12));
        let reply = poll(&mut brain);
        assert_eq!(reply.id, 12);
        assert!(reply.plan.is_none());
        let stats = brain.stats();
        assert_eq!(stats.requests, 1);
        assert_eq!(stats.answered, 0);
        assert_eq!(stats.failed.get(&reason), Some(&1));
        assert!(!format!("{stats:?}").contains(KEY));
        let log = std::fs::read_to_string(&path).unwrap();
        assert!(!log.contains(KEY));
        let line: Value = serde_json::from_str(log.trim()).unwrap();
        assert_eq!(line["chosen"], Value::Null);
        assert_eq!(line["status"], "failed");
        if reason == Failure::Refusal && backend == Backend::Decisions {
            assert_eq!(stats.input_tokens, 7);
        }
        mock.finish();
        drop(brain);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn slow_http_never_blocks_submit_and_timeout_is_bounded() {
    let timeout = Duration::from_millis(500);
    let mut response = Response::json(decisions("3005", 0, 0));
    response.delay = Duration::from_millis(800);
    let mut mock = Mock::new(Backend::Decisions, vec![response]);
    let mut config = mock.config(Backend::Decisions);
    config.timeout = timeout;
    let mut brain = HostedBrain::new(config, &OWL).unwrap();
    let start = Instant::now();
    brain.submit(&request(77));
    assert!(
        start.elapsed() < Duration::from_millis(5),
        "submit must not wait for HTTP"
    );
    let reply = poll(&mut brain);
    assert!(start.elapsed() < timeout + Duration::from_millis(500));
    assert_eq!(reply.id, 77);
    assert!(reply.plan.is_none());
    assert_eq!(brain.stats().failed.get(&Failure::Timeout), Some(&1));
    mock.finish();
}

#[test]
fn drop_in_flight_is_bounded_and_does_not_drain_queued_requests() {
    let timeout = Duration::from_millis(500);
    let mut response = Response::json(decisions("3005", 0, 0));
    response.delay = Duration::from_millis(800);
    let mut mock = Mock::new(Backend::Decisions, vec![response]);
    let mut config = mock.config(Backend::Decisions);
    config.timeout = timeout;
    let mut brain = HostedBrain::new(config, &OWL).unwrap();
    brain.submit(&request(1));
    mock.started.recv_timeout(Duration::from_secs(2)).unwrap();
    for id in 2..20 {
        let start = Instant::now();
        brain.submit(&request(id));
        assert!(start.elapsed() < Duration::from_millis(5));
    }
    let handle = brain.stats_handle();
    let start = Instant::now();
    drop(brain);
    assert!(start.elapsed() < timeout + Duration::from_millis(500));
    assert_eq!(handle.snapshot().failed.get(&Failure::Timeout), Some(&1));
    mock.finish();
    assert_eq!(mock.accepted.load(Ordering::Relaxed), 1);
}

#[test]
fn global_timeout_also_bounds_reading_the_response_body() {
    let timeout = Duration::from_millis(500);
    let mut response = Response::json(responses("3005"));
    response.delay = Duration::from_millis(800);
    response.headers_first = true;
    let mut mock = Mock::new(Backend::Responses, vec![response]);
    let mut config = mock.config(Backend::Responses);
    config.timeout = timeout;
    let mut brain = HostedBrain::new(config, &OWL).unwrap();
    let start = Instant::now();
    brain.submit(&request(3));
    assert!(poll(&mut brain).plan.is_none());
    assert!(start.elapsed() < timeout + Duration::from_millis(500));
    assert_eq!(brain.stats().failed.get(&Failure::Timeout), Some(&1));
    mock.finish();
}

#[test]
fn configuration_errors_and_debug_are_redacted() {
    let mut config = HostedConfig::new(Backend::Decisions, ApiKey::new(KEY));
    config.model = KEY.into();
    config.base_url = format!("http://127.0.0.1/{KEY}");
    config.log_path = Some(KEY.into());
    assert!(!format!("{config:?}").contains(KEY));
    config.timeout = Duration::ZERO;
    let error = HostedBrain::new(config, &OWL).err().unwrap();
    assert_eq!(error, ConfigError::InvalidConfig);
    for error in [
        error,
        ConfigError::MissingKey,
        ConfigError::LogOpen,
        ConfigError::WorkerStart,
    ] {
        assert!(!error.to_string().contains(KEY));
        assert!(!format!("{error:?}").contains(KEY));
    }
    assert!(!format!("{:?}", ApiKey::new(KEY)).contains(KEY));
    assert!(
        ConfigError::MissingKey
            .to_string()
            .contains("OPENAI_API_KEY")
    );
}

#[test]
fn existing_cli_controllers_still_build_without_credentials() {
    for name in ["lua", "scripted"] {
        let brain = sekiro_brain_net::from_cli(name, None).unwrap();
        assert_eq!(brain.name(), name);
    }
    assert!(sekiro_brain_net::from_cli("unknown", None).is_err());
}
