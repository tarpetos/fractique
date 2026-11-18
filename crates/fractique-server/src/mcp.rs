use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, MutexGuard, PoisonError};
use std::task::{Context, Poll};

use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::header::{ACCEPT, CACHE_CONTROL, CONTENT_TYPE, ORIGIN};
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use fractique_core::Control;
use fractique_mcp::{Step, answer, failure, handle, outcome, prepare, reporter, tool_call};
use futures_core::Stream;
use serde_json::Value;
use tokio::sync::Notify;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{Shared, Ticket};

// Streamable HTTP came with 2025-03-26; the revision before it has another HTTP transport.
pub(crate) const PROTOCOLS: [&str; 2] = ["2025-06-18", "2025-03-26"];
const MAX_SESSIONS: usize = 1024;
const SESSION: HeaderName = HeaderName::from_static("mcp-session-id");
const VERSION: HeaderName = HeaderName::from_static("mcp-protocol-version");

#[derive(Default)]
struct Halt {
    flag: AtomicBool,
    woken: Notify,
}

impl Halt {
    fn stop(&self) {
        self.flag.store(true, Ordering::Relaxed);
        self.woken.notify_one();
    }
}

struct StopOnDrop(Arc<Halt>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.stop();
    }
}

#[derive(Default)]
struct Session {
    used: u64,
    calls: Vec<(Value, Arc<Halt>)>,
}

impl Session {
    fn stop(&self) {
        self.calls.iter().for_each(|(_, halt)| halt.stop());
    }
}

#[derive(Default)]
pub(crate) struct Sessions {
    open: HashMap<String, Session>,
    clock: u64,
}

impl Sessions {
    fn start(&mut self) -> Result<String, String> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|e| format!("no random numbers for a session: {e}"))?;
        let id: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        if self.open.len() >= MAX_SESSIONS {
            let idlest = self
                .open
                .iter()
                .min_by_key(|(_, session)| (!session.calls.is_empty(), session.used))
                .map(|(id, _)| id.clone());
            if let Some(session) = idlest.and_then(|id| self.open.remove(&id)) {
                session.stop();
            }
        }
        self.clock += 1;
        let session = Session {
            used: self.clock,
            calls: Vec::new(),
        };
        self.open.insert(id.clone(), session);
        Ok(id)
    }

    fn used(&mut self, id: &str) -> Option<&mut Session> {
        self.clock += 1;
        let session = self.open.get_mut(id)?;
        session.used = self.clock;
        Some(session)
    }
}

fn sessions(shared: &Shared) -> MutexGuard<'_, Sessions> {
    shared.sessions.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) struct Refused(StatusCode, i64, String);

impl IntoResponse for Refused {
    fn into_response(self) -> Response {
        (self.0, Json(failure(Value::Null, self.1, self.2))).into_response()
    }
}

fn refuse(status: StatusCode, code: i64, message: impl Into<String>) -> Refused {
    Refused(status, code, message.into())
}

fn accepts(headers: &HeaderMap, kind: &str) -> bool {
    let family = kind.split_once('/').map_or(kind, |(family, _)| family);
    let mut listed = headers
        .get_all(ACCEPT)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(|item| item.split(';').next().unwrap_or_default().trim().to_ascii_lowercase())
        .peekable();
    listed.peek().is_none()
        || listed.any(|item| item == kind || item == "*/*" || item.strip_suffix("/*") == Some(family))
}

fn not_acceptable(kind: &str) -> Refused {
    refuse(
        StatusCode::NOT_ACCEPTABLE,
        -32600,
        format!("the answer is {kind}: list it in the Accept header"),
    )
}

// A page of another site reaches a server on this machine through its visitor's browser (DNS rebinding):
// the browser names the page in Origin, and only a page of this machine, or one the settings name, may call.
fn origin_allowed(headers: &HeaderMap, allowed: &[String]) -> bool {
    let Some(origin) = headers.get(ORIGIN) else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    if allowed.iter().any(|one| one == "*" || one.eq_ignore_ascii_case(origin)) {
        return true;
    }
    let Some(authority) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    let host = match authority.strip_prefix('[') {
        Some(rest) => rest
            .split_once(']')
            .filter(|(_, port)| port.is_empty() || port.starts_with(':'))
            .map(|(host, _)| host),
        None => Some(authority.rsplit_once(':').map_or(authority, |(host, _)| host)),
    };
    host.is_some_and(|host| host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1")
}

fn admit(shared: &Shared, headers: &HeaderMap) -> Result<(), Refused> {
    if !origin_allowed(headers, &shared.config.mcp_origins) {
        return Err(refuse(
            StatusCode::FORBIDDEN,
            -32600,
            "this origin may not call the MCP endpoint",
        ));
    }
    match headers.get(VERSION).map(|version| version.to_str()) {
        Some(Ok(version)) if PROTOCOLS.contains(&version) => Ok(()),
        None => Ok(()),
        Some(_) => Err(refuse(
            StatusCode::BAD_REQUEST,
            -32600,
            format!(
                "the MCP-Protocol-Version header must be one of {}",
                PROTOCOLS.join(", ")
            ),
        )),
    }
}

fn session_of(shared: &Shared, headers: &HeaderMap) -> Result<String, Refused> {
    let Some(id) = headers.get(SESSION).and_then(|id| id.to_str().ok()) else {
        return Err(refuse(
            StatusCode::BAD_REQUEST,
            -32600,
            "the Mcp-Session-Id header is missing: initialize first",
        ));
    };
    match sessions(shared).used(id) {
        Some(_) => Ok(id.to_owned()),
        None => Err(no_session()),
    }
}

fn no_session() -> Refused {
    refuse(StatusCode::NOT_FOUND, -32600, "no such session: initialize again")
}

struct Events {
    messages: UnboundedReceiver<Value>,
    ended: bool,
    _stop: StopOnDrop,
}

impl Stream for Events {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.ended {
            return Poll::Ready(None);
        }
        match self.messages.poll_recv(context) {
            Poll::Ready(Some(message)) if !message.is_null() => {
                self.ended = message.get("method").is_none();
                Poll::Ready(Some(Ok(Bytes::from(format!("event: message\ndata: {message}\n\n")))))
            }
            Poll::Ready(_) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn work(
    shared: &Arc<Shared>,
    name: String,
    arguments: Value,
    token: Option<Value>,
    halt: &Arc<Halt>,
    out: UnboundedSender<Value>,
) -> Result<Vec<Value>, String> {
    let host = shared.clone();
    let step = tokio::task::spawn_blocking(move || prepare(&name, &arguments, &host.host));
    let job = match step.await.map_err(|e| e.to_string())?? {
        Step::Done(content) => return Ok(content),
        Step::Render(job) => job,
    };
    let ticket = Ticket::take(shared).ok_or("the render queue is full; try again later")?;
    let slot = tokio::select! {
        slot = shared.slots.clone().acquire_owned() => slot.map_err(|e| e.to_string())?,
        () = halt.woken.notified() => return Err("cancelled".into()),
    };
    let stop = halt.clone();
    let running = tokio::task::spawn_blocking(move || {
        let _held_until_the_render_ends = (slot, ticket);
        let report = token.as_ref().map(|token| {
            reporter(token, |message| {
                if !stop.flag.load(Ordering::Relaxed) {
                    let _ = out.send(message);
                }
            })
        });
        let control = Control {
            cancel: Some(&stop.flag),
            progress: match &report {
                Some(report) => Some(report),
                None => None,
            },
        };
        job.run(&control)
    });
    match tokio::time::timeout(shared.config.timeout, running).await {
        Ok(done) => done.map_err(|e| e.to_string())?,
        Err(_) => Err(format!(
            "the render took longer than the {} s this server allows",
            shared.config.timeout.as_secs_f64()
        )),
    }
}

fn call(
    shared: Arc<Shared>,
    session: String,
    id: Value,
    mut params: Value,
    headers: &HeaderMap,
) -> Result<Response, Refused> {
    if !accepts(headers, "text/event-stream") {
        return Err(not_acceptable("text/event-stream"));
    }
    let (name, token) = match tool_call(&params) {
        Ok(call) => (call.name.to_owned(), call.token.cloned()),
        Err(e) => return Ok(Json(failure(id, e.code, e.message)).into_response()),
    };
    let arguments = params.get_mut("arguments").map(Value::take).unwrap_or_default();
    let halt = Arc::new(Halt::default());
    let call = (id.clone(), halt.clone());
    sessions(&shared)
        .used(&session)
        .ok_or_else(no_session)?
        .calls
        .push(call);
    let (out, messages) = mpsc::unbounded_channel();
    let events = Events {
        messages,
        ended: false,
        _stop: StopOnDrop(halt.clone()),
    };
    tokio::spawn(async move {
        let result = work(&shared, name, arguments, token, &halt, out.clone()).await;
        // A cancelled request gets no answer; the stream ends instead.
        let cancelled = halt.flag.load(Ordering::Relaxed);
        halt.stop();
        let _ = out.send(if cancelled {
            Value::Null
        } else {
            answer(&id, outcome(result))
        });
        if let Some(session) = sessions(&shared).open.get_mut(&session) {
            session.calls.retain(|(_, other)| !Arc::ptr_eq(other, &halt));
        }
    });
    let kind = [(CONTENT_TYPE, "text/event-stream"), (CACHE_CONTROL, "no-cache")];
    Ok((kind, Body::from_stream(events)).into_response())
}

pub(crate) async fn post(
    State(shared): State<Arc<Shared>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Refused> {
    admit(&shared, &headers)?;
    let json = headers
        .get(CONTENT_TYPE)
        .and_then(|kind| kind.to_str().ok())
        .is_some_and(|kind| kind.trim_start().to_ascii_lowercase().starts_with("application/json"));
    if !json {
        return Err(refuse(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            -32600,
            "the body must be application/json",
        ));
    }
    let mut message: Value =
        serde_json::from_slice(&body).map_err(|e| refuse(StatusCode::BAD_REQUEST, -32700, format!("not JSON: {e}")))?;
    if message.is_array() {
        return Err(refuse(StatusCode::BAD_REQUEST, -32600, "batches are not supported"));
    }
    let id = message.get("id").filter(|id| !id.is_null()).cloned();
    let method = message.get("method").and_then(Value::as_str).map(str::to_owned);
    if method.is_none() && message.get("result").is_none() && message.get("error").is_none() {
        return Err(refuse(StatusCode::BAD_REQUEST, -32600, "not a JSON-RPC message"));
    }
    let params = message.get_mut("params").map(Value::take).unwrap_or_default();
    let reply = |id: Value, method: &str| {
        if !accepts(&headers, "application/json") {
            return Err(not_acceptable("application/json"));
        }
        Ok(Json(match handle(method, &params, &shared.host) {
            Ok(result) => answer(&id, result),
            Err(e) => failure(id, e.code, e.message),
        }))
    };

    if let (Some("initialize"), Some(id)) = (method.as_deref(), &id) {
        let answer = reply(id.clone(), "initialize")?;
        let session = sessions(&shared)
            .start()
            .map_err(|e| refuse(StatusCode::INTERNAL_SERVER_ERROR, -32603, e))?;
        return Ok(([(SESSION, session)], answer).into_response());
    }
    let session = session_of(&shared, &headers)?;
    match (method.as_deref(), id) {
        (Some("notifications/cancelled"), _) => {
            if let Some(id) = params.get("requestId")
                && let Some(session) = sessions(&shared).open.get(&session)
            {
                let named = session.calls.iter().filter(|(call, _)| call == id);
                named.for_each(|(_, halt)| halt.stop());
            }
            Ok(StatusCode::ACCEPTED.into_response())
        }
        (None, _) | (_, None) => Ok(StatusCode::ACCEPTED.into_response()),
        (Some("tools/call"), Some(id)) => call(shared.clone(), session, id, params.clone(), &headers),
        (Some(method), Some(id)) => reply(id, method).map(IntoResponse::into_response),
    }
}

pub(crate) async fn delete(State(shared): State<Arc<Shared>>, headers: HeaderMap) -> Result<StatusCode, Refused> {
    admit(&shared, &headers)?;
    let session = session_of(&shared, &headers)?;
    if let Some(session) = sessions(&shared).open.remove(&session) {
        session.stop();
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(name: HeaderName, value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, value.parse().unwrap());
        headers
    }

    #[test]
    fn only_pages_of_this_machine_or_named_ones_may_call() {
        let named = ["https://fractique.example".to_owned()];
        assert!(origin_allowed(&HeaderMap::new(), &[]));
        for origin in [
            "http://localhost",
            "http://LOCALHOST:8080",
            "https://127.0.0.1:3000",
            "http://[::1]:8080",
        ] {
            assert!(origin_allowed(&with(ORIGIN, origin), &[]), "{origin}");
        }
        for origin in [
            "http://evil.example",
            "http://localhost.evil.example",
            "http://127.0.0.1.evil.example:8080",
            "http://[::1].evil.example",
            "file://localhost",
            "null",
        ] {
            assert!(!origin_allowed(&with(ORIGIN, origin), &named), "{origin}");
        }
        assert!(origin_allowed(&with(ORIGIN, "https://fractique.example"), &named));
        assert!(origin_allowed(&with(ORIGIN, "http://evil.example"), &["*".to_owned()]));
    }

    #[test]
    fn accept_is_read_as_a_list() {
        assert!(accepts(&HeaderMap::new(), "text/event-stream"));
        let both = with(ACCEPT, "application/json, text/event-stream;q=0.9");
        assert!(accepts(&both, "text/event-stream") && accepts(&both, "application/json"));
        assert!(accepts(&with(ACCEPT, "*/*"), "text/event-stream"));
        assert!(accepts(&with(ACCEPT, "text/*"), "text/event-stream"));
        assert!(!accepts(&with(ACCEPT, "application/json"), "text/event-stream"));
    }

    #[test]
    fn the_idlest_session_makes_room() {
        let mut sessions = Sessions::default();
        let first = sessions.start().unwrap();
        let busy = sessions.start().unwrap();
        let halt = Arc::new(Halt::default());
        sessions.used(&busy).unwrap().calls.push((Value::from(1), halt.clone()));
        for _ in 2..MAX_SESSIONS {
            sessions.start().unwrap();
        }
        sessions.used(&first).unwrap();
        let newest = sessions.start().unwrap();
        assert_eq!(sessions.open.len(), MAX_SESSIONS);
        assert!(sessions.used(&first).is_some() && sessions.used(&busy).is_some() && sessions.used(&newest).is_some());
        assert!(!halt.flag.load(Ordering::Relaxed));
        assert_ne!(first, newest);
        assert_eq!(newest.len(), 32);
    }
}
