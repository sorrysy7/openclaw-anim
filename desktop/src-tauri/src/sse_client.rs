use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONNECTION};
use std::time::Duration;
use tokio::time::sleep;

/// Minimal payload emitted by the OpenClaw SSE plugin.
/// IMPORTANT: This is already content-free (no params/results).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GatewayAnimEvent {
    #[serde(default)]
    pub ts: u64,
    #[serde(rename = "runId")]
    #[serde(default)]
    pub run_id: Option<String>,
    pub phase: String,
    pub tool: String,
}

#[derive(Debug, Clone)]
pub struct SseClientConfig {
    pub url: String,
    pub bearer_token: String,
    pub connect_timeout: Duration,
    pub read_idle_timeout: Duration,
    pub max_backoff: Duration,
}

/// Stream SSE and invoke `on_event` for each parsed `data: {...}` JSON line.
///
/// Security rules:
/// - Never log the token.
/// - Never log raw SSE lines.
pub async fn run_sse_loop<F>(cfg: SseClientConfig, mut on_event: F) -> Result<()>
where
    F: FnMut(GatewayAnimEvent) + Send + 'static,
{
    let client = reqwest::Client::builder()
        .connect_timeout(cfg.connect_timeout)
        // We intentionally avoid reqwest's overall request timeout; this is a long-lived stream.
        .build()
        .context("build reqwest client")?;

    let mut backoff = Duration::from_millis(500);

    loop {
        let mut headers = HeaderMap::new();
        let auth_value = format!("Bearer {}", cfg.bearer_token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_value).context("auth header")?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));

        let resp = client.get(&cfg.url).headers(headers).send().await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                println!("[sse] connect error: {e}");
                sleep(backoff).await;
                backoff = (backoff * 2).min(cfg.max_backoff);
                continue;
            }
        };

        if !resp.status().is_success() {
            println!("[sse] bad status: {}", resp.status());
            sleep(backoff).await;
            backoff = (backoff * 2).min(cfg.max_backoff);
            continue;
        }

        println!("[sse] connected to {}", cfg.url);
        // Successful connection resets backoff.
        backoff = Duration::from_millis(500);

        let mut stream = resp.bytes_stream();
        let mut buf: Vec<u8> = Vec::new();

        // Simple line-based SSE parser:
        // - accumulate bytes
        // - split by '\n'
        // - process lines starting with "data:"
        // - ignore comments/heartbeats (": ...")
        // This matches our plugin which emits single-line JSON in `data:`.

        loop {
            let next = tokio::time::timeout(cfg.read_idle_timeout, stream.next()).await;

            let chunk_opt = match next {
                Ok(v) => v,
                Err(_elapsed) => {
                    // idle too long -> reconnect
                    break;
                }
            };

            let chunk = match chunk_opt {
                Some(Ok(b)) => b,
                Some(Err(_e)) => break,
                None => break,
            };

            buf.extend_from_slice(&chunk);

            while let Some(pos) = buf.iter().position(|&c| c == b'\n') {
                let mut line = buf.drain(..=pos).collect::<Vec<u8>>();
                // trim trailing \n/\r
                while matches!(line.last(), Some(b'\n' | b'\r')) {
                    line.pop();
                }
                if line.is_empty() {
                    continue;
                }

                // comment / heartbeat
                if line.first() == Some(&b':') {
                    continue;
                }

                const PREFIX: &[u8] = b"data:";
                if line.starts_with(PREFIX) {
                    let json_bytes = line[PREFIX.len()..]
                        .iter()
                        .skip_while(|&&b| b == b' ')
                        .copied()
                        .collect::<Vec<u8>>();
                    if json_bytes.is_empty() {
                        continue;
                    }
                    if let Ok(text) = std::str::from_utf8(&json_bytes) {
                        if let Ok(ev) = serde_json::from_str::<GatewayAnimEvent>(text) {
                            on_event(ev);
                        }
                    }
                }
            }
        }

        // reconnect with backoff
        sleep(backoff).await;
        backoff = (backoff * 2).min(cfg.max_backoff);
    }
}
