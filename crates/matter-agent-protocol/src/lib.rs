#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    Planner,
    Worker,
    Reviewer,
    Runtime,
    UserProxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Proposed,
    InProgress,
    Blocked,
    ReadyForReview,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameReadiness {
    Ready,
    Incomplete,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseKind {
    Accepted,
    NeedsContext,
    Blocked,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentId {
    pub name: String,
    pub role: AgentRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentFrame {
    pub from: AgentId,
    pub to: AgentId,
    pub task_id: String,
    pub state: TaskState,
    pub goal: String,
    pub summary: String,
    pub facts: Vec<String>,
    pub blockers: Vec<String>,
    pub requests: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResponse {
    pub from: AgentId,
    pub to: AgentId,
    pub task_id: String,
    pub kind: ResponseKind,
    pub summary: String,
    pub missing_context: Vec<&'static str>,
    pub blockers: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    Frame(AgentFrame),
    Response(AgentResponse),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    pub session_id: String,
    pub events: Vec<AgentEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionContext {
    pub session_id: String,
    pub event_count: usize,
    pub latest_task_id: String,
    pub latest_goal: String,
    pub latest_summary: String,
    pub facts: Vec<String>,
    pub blockers: Vec<String>,
    pub requests: Vec<String>,
    pub last_response_kind: Option<ResponseKind>,
    pub next_action: String,
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHandoffPacket {
    pub context: AgentSessionContext,
    pub recent_events: Vec<String>,
}

impl AgentId {
    pub fn new(name: impl Into<String>, role: AgentRole) -> Self {
        Self {
            name: name.into(),
            role,
        }
    }
}

impl AgentFrame {
    pub fn new(from: AgentId, to: AgentId, task_id: impl Into<String>) -> Self {
        Self {
            from,
            to,
            task_id: task_id.into(),
            state: TaskState::Proposed,
            goal: String::new(),
            summary: String::new(),
            facts: Vec::new(),
            blockers: Vec::new(),
            requests: Vec::new(),
            next_action: String::new(),
        }
    }

    pub fn with_state(mut self, state: TaskState) -> Self {
        self.state = state;
        self
    }

    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = goal.into();
        self
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn with_next_action(mut self, next_action: impl Into<String>) -> Self {
        self.next_action = next_action.into();
        self
    }

    pub fn add_fact(mut self, fact: impl Into<String>) -> Self {
        self.facts.push(fact.into());
        self
    }

    pub fn add_blocker(mut self, blocker: impl Into<String>) -> Self {
        self.blockers.push(blocker.into());
        self
    }

    pub fn add_request(mut self, request: impl Into<String>) -> Self {
        self.requests.push(request.into());
        self
    }

    pub fn is_actionable(&self) -> bool {
        self.readiness() == FrameReadiness::Ready
    }

    pub fn readiness(&self) -> FrameReadiness {
        if self.state == TaskState::Blocked || !self.blockers.is_empty() {
            FrameReadiness::Blocked
        } else if self.missing_context().is_empty() {
            FrameReadiness::Ready
        } else {
            FrameReadiness::Incomplete
        }
    }

    pub fn missing_context(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.goal.trim().is_empty() {
            missing.push("goal");
        }
        if self.summary.trim().is_empty() {
            missing.push("summary");
        }
        if self.next_action.trim().is_empty() {
            missing.push("next_action");
        }
        if self.state == TaskState::Blocked && self.blockers.is_empty() {
            missing.push("blockers");
        }
        missing
    }

    pub fn handoff_summary(&self) -> String {
        let facts = list_or_none(&self.facts);
        let blockers = list_or_none(&self.blockers);
        let requests = list_or_none(&self.requests);

        [
            format!("task: {}", self.task_id),
            format!("state: {}", task_state_word(self.state)),
            format!("from: {}", self.from.name),
            format!("to: {}", self.to.name),
            format!("goal: {}", empty_word(&self.goal)),
            format!("summary: {}", empty_word(&self.summary)),
            format!("facts: {}", facts),
            format!("blockers: {}", blockers),
            format!("requests: {}", requests),
            format!("next_action: {}", empty_word(&self.next_action)),
        ]
        .join("\n")
    }

    pub fn response_from_receiver(&self) -> AgentResponse {
        AgentResponse::from_frame(self)
    }
}

impl AgentResponse {
    pub fn from_frame(frame: &AgentFrame) -> Self {
        let readiness = frame.readiness();
        let kind = match readiness {
            FrameReadiness::Ready => ResponseKind::Accepted,
            FrameReadiness::Incomplete => ResponseKind::NeedsContext,
            FrameReadiness::Blocked => ResponseKind::Blocked,
        };

        Self {
            from: frame.to.clone(),
            to: frame.from.clone(),
            task_id: frame.task_id.clone(),
            kind,
            summary: response_summary(kind),
            missing_context: frame.missing_context(),
            blockers: frame.blockers.clone(),
            next_action: response_next_action(kind),
        }
    }

    pub fn completed(
        from: AgentId,
        to: AgentId,
        task_id: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            from,
            to,
            task_id: task_id.into(),
            kind: ResponseKind::Completed,
            summary: summary.into(),
            missing_context: Vec::new(),
            blockers: Vec::new(),
            next_action: "none".to_string(),
        }
    }

    pub fn is_terminal(&self) -> bool {
        self.kind == ResponseKind::Completed
    }

    pub fn summary_text(&self) -> String {
        [
            format!("task: {}", self.task_id),
            format!("kind: {}", response_kind_word(self.kind)),
            format!("from: {}", self.from.name),
            format!("to: {}", self.to.name),
            format!("summary: {}", empty_word(&self.summary)),
            format!("missing_context: {}", static_list_or_none(&self.missing_context)),
            format!("blockers: {}", list_or_none(&self.blockers)),
            format!("next_action: {}", empty_word(&self.next_action)),
        ]
        .join("\n")
    }
}

impl AgentSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            events: Vec::new(),
        }
    }

    pub fn add_frame(mut self, frame: AgentFrame) -> Self {
        self.events.push(AgentEvent::Frame(frame));
        self
    }

    pub fn add_response(mut self, response: AgentResponse) -> Self {
        self.events.push(AgentEvent::Response(response));
        self
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn last_response(&self) -> Option<&AgentResponse> {
        self.events.iter().rev().find_map(|event| match event {
            AgentEvent::Response(response) => Some(response),
            AgentEvent::Frame(_) => None,
        })
    }

    pub fn last_frame(&self) -> Option<&AgentFrame> {
        self.events.iter().rev().find_map(|event| match event {
            AgentEvent::Frame(frame) => Some(frame),
            AgentEvent::Response(_) => None,
        })
    }

    pub fn is_terminal(&self) -> bool {
        self.last_response()
            .map(|response| response.is_terminal())
            .unwrap_or(false)
    }

    pub fn next_action(&self) -> &str {
        self.last_response()
            .map(|response| response.next_action.as_str())
            .unwrap_or("start")
    }

    pub fn summary_text(&self) -> String {
        [
            format!("session: {}", self.session_id),
            format!("events: {}", self.event_count()),
            format!("terminal: {}", self.is_terminal()),
            format!("next_action: {}", self.next_action()),
        ]
        .join("\n")
    }

    pub fn context(&self) -> AgentSessionContext {
        let mut facts = Vec::new();
        let mut blockers = Vec::new();
        let mut requests = Vec::new();

        for event in &self.events {
            match event {
                AgentEvent::Frame(frame) => {
                    facts.extend(frame.facts.clone());
                    blockers.extend(frame.blockers.clone());
                    requests.extend(frame.requests.clone());
                }
                AgentEvent::Response(response) => {
                    blockers.extend(response.blockers.clone());
                }
            }
        }

        let latest_frame = self.last_frame();
        let last_response = self.last_response();

        AgentSessionContext {
            session_id: self.session_id.clone(),
            event_count: self.event_count(),
            latest_task_id: latest_frame
                .map(|frame| frame.task_id.clone())
                .or_else(|| last_response.map(|response| response.task_id.clone()))
                .unwrap_or_default(),
            latest_goal: latest_frame
                .map(|frame| frame.goal.clone())
                .unwrap_or_default(),
            latest_summary: last_response
                .map(|response| response.summary.clone())
                .or_else(|| latest_frame.map(|frame| frame.summary.clone()))
                .unwrap_or_default(),
            facts,
            blockers,
            requests,
            last_response_kind: last_response.map(|response| response.kind),
            next_action: self.next_action().to_string(),
            terminal: self.is_terminal(),
        }
    }

    pub fn recent_event_summaries(&self, limit: usize) -> Vec<String> {
        if limit == 0 {
            return Vec::new();
        }

        let mut summaries: Vec<String> = self
            .events
            .iter()
            .rev()
            .take(limit)
            .map(|event| match event {
                AgentEvent::Frame(frame) => format!(
                    "frame {} -> {} ({})",
                    frame.from.name,
                    frame.to.name,
                    frame.task_id
                ),
                AgentEvent::Response(response) => format!(
                    "response {} -> {} ({}) [{}]",
                    response.from.name,
                    response.to.name,
                    response.task_id,
                    response_kind_word(response.kind)
                ),
            })
            .collect();
        summaries.reverse();
        summaries
    }

    pub fn handoff_packet(&self, limit: usize) -> AgentHandoffPacket {
        AgentHandoffPacket {
            context: self.context(),
            recent_events: self.recent_event_summaries(limit),
        }
    }
}

impl AgentHandoffPacket {
    pub fn to_wire(&self) -> String {
        let last_kind = self
            .context
            .last_response_kind
            .map(response_kind_word)
            .unwrap_or("none");

        let lines = [
            "version=1".to_string(),
            format!("session_id={}", encode_field(&self.context.session_id)),
            format!("event_count={}", self.context.event_count),
            format!("latest_task_id={}", encode_field(&self.context.latest_task_id)),
            format!("latest_goal={}", encode_field(&self.context.latest_goal)),
            format!(
                "latest_summary={}",
                encode_field(&self.context.latest_summary)
            ),
            format!("facts={}", encode_list(&self.context.facts)),
            format!("blockers={}", encode_list(&self.context.blockers)),
            format!("requests={}", encode_list(&self.context.requests)),
            format!("last_response_kind={}", last_kind),
            format!("next_action={}", encode_field(&self.context.next_action)),
            format!("terminal={}", self.context.terminal),
            format!("recent_events={}", encode_list(&self.recent_events)),
        ];
        let payload = lines.join("\n");
        let checksum = wire_checksum(&payload);
        format!("{}\nchecksum={}", payload, checksum)
    }

    pub fn from_wire(wire: &str) -> Result<Self, String> {
        let (payload, provided_checksum) = split_payload_and_checksum(wire)?;
        let expected_checksum = wire_checksum(payload);
        let mut pairs = std::collections::HashMap::new();
        for line in payload.lines().filter(|line| !line.trim().is_empty()) {
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("invalid wire line: {}", line))?;
            pairs.insert(key.trim().to_string(), value.to_string());
        }
        if provided_checksum != expected_checksum {
            return Err(format!(
                "wire checksum mismatch: expected {}, got {}",
                expected_checksum, provided_checksum
            ));
        }

        let version = require_pair(&pairs, "version")?;
        if version != "1" {
            return Err(format!("unsupported wire version: {}", version));
        }

        let event_count = require_pair(&pairs, "event_count")?
            .parse::<usize>()
            .map_err(|error| format!("invalid event_count: {}", error))?;
        let terminal = require_pair(&pairs, "terminal")?
            .parse::<bool>()
            .map_err(|error| format!("invalid terminal: {}", error))?;
        let last_response_kind = match require_pair(&pairs, "last_response_kind")? {
            "none" => None,
            value => Some(
                response_kind_from_word(value)
                    .ok_or_else(|| format!("invalid last_response_kind: {}", value))?,
            ),
        };

        Ok(Self {
            context: AgentSessionContext {
                session_id: decode_field(require_pair(&pairs, "session_id")?)?,
                event_count,
                latest_task_id: decode_field(require_pair(&pairs, "latest_task_id")?)?,
                latest_goal: decode_field(require_pair(&pairs, "latest_goal")?)?,
                latest_summary: decode_field(require_pair(&pairs, "latest_summary")?)?,
                facts: decode_list(require_pair(&pairs, "facts")?)?,
                blockers: decode_list(require_pair(&pairs, "blockers")?)?,
                requests: decode_list(require_pair(&pairs, "requests")?)?,
                last_response_kind,
                next_action: decode_field(require_pair(&pairs, "next_action")?)?,
                terminal,
            },
            recent_events: decode_list(require_pair(&pairs, "recent_events")?)?,
        })
    }
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join("; ")
    }
}

fn empty_word(value: &str) -> &str {
    if value.trim().is_empty() {
        "none"
    } else {
        value
    }
}

fn task_state_word(state: TaskState) -> &'static str {
    match state {
        TaskState::Proposed => "proposed",
        TaskState::InProgress => "in_progress",
        TaskState::Blocked => "blocked",
        TaskState::ReadyForReview => "ready_for_review",
        TaskState::Completed => "completed",
    }
}

fn response_kind_word(kind: ResponseKind) -> &'static str {
    match kind {
        ResponseKind::Accepted => "accepted",
        ResponseKind::NeedsContext => "needs_context",
        ResponseKind::Blocked => "blocked",
        ResponseKind::Completed => "completed",
    }
}

fn response_summary(kind: ResponseKind) -> String {
    match kind {
        ResponseKind::Accepted => "Frame accepted for execution".to_string(),
        ResponseKind::NeedsContext => "Frame needs more context before execution".to_string(),
        ResponseKind::Blocked => "Frame is blocked before execution".to_string(),
        ResponseKind::Completed => "Frame completed".to_string(),
    }
}

fn response_next_action(kind: ResponseKind) -> String {
    match kind {
        ResponseKind::Accepted => "execute".to_string(),
        ResponseKind::NeedsContext => "request-context".to_string(),
        ResponseKind::Blocked => "resolve-blockers".to_string(),
        ResponseKind::Completed => "none".to_string(),
    }
}

fn static_list_or_none(values: &[&'static str]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join("; ")
    }
}

fn require_pair<'a>(
    pairs: &'a std::collections::HashMap<String, String>,
    key: &str,
) -> Result<&'a str, String> {
    pairs
        .get(key)
        .map(|value| value.as_str())
        .ok_or_else(|| format!("missing wire field: {}", key))
}

fn encode_list(values: &[String]) -> String {
    if values.is_empty() {
        return String::new();
    }
    values
        .iter()
        .map(|value| encode_field(value))
        .collect::<Vec<_>>()
        .join("|")
}

fn decode_list(value: &str) -> Result<Vec<String>, String> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value.split('|').map(decode_field).collect()
}

fn encode_field(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '\n' => out.push_str("%0A"),
            '|' => out.push_str("%7C"),
            '=' => out.push_str("%3D"),
            _ => out.push(ch),
        }
    }
    out
}

fn decode_field(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut out = String::new();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'%' {
            if idx + 2 >= bytes.len() {
                return Err("invalid escape sequence in wire field".to_string());
            }
            let hex = &value[idx + 1..idx + 3];
            let parsed = u8::from_str_radix(hex, 16)
                .map_err(|error| format!("invalid escape sequence {}: {}", hex, error))?;
            out.push(parsed as char);
            idx += 3;
        } else {
            out.push(bytes[idx] as char);
            idx += 1;
        }
    }
    Ok(out)
}

fn response_kind_from_word(word: &str) -> Option<ResponseKind> {
    match word {
        "accepted" => Some(ResponseKind::Accepted),
        "needs_context" => Some(ResponseKind::NeedsContext),
        "blocked" => Some(ResponseKind::Blocked),
        "completed" => Some(ResponseKind::Completed),
        _ => None,
    }
}

fn split_payload_and_checksum(wire: &str) -> Result<(&str, &str), String> {
    let idx = wire
        .rfind("\nchecksum=")
        .ok_or_else(|| "missing wire checksum".to_string())?;
    let payload = &wire[..idx];
    let checksum = &wire[idx + "\nchecksum=".len()..];
    if checksum.trim().is_empty() {
        return Err("missing wire checksum value".to_string());
    }
    Ok((payload, checksum.trim()))
}

fn wire_checksum(payload: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in payload.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actionable_frame_requires_operational_context() {
        let frame = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "pkg-sync",
        )
        .with_state(TaskState::InProgress)
        .with_goal("Synchronize Matter package dependencies")
        .with_summary("Lockfile is stale and installed packages must be refreshed")
        .add_fact("matter.lock exists")
        .with_next_action("run package sync");

        assert!(frame.is_actionable());
        assert_eq!(frame.readiness(), FrameReadiness::Ready);
        assert!(frame.missing_context().is_empty());
        assert!(frame.handoff_summary().contains("next_action: run package sync"));
    }

    #[test]
    fn blocked_frame_must_name_blockers() {
        let frame = AgentFrame::new(
            AgentId::new("worker", AgentRole::Worker),
            AgentId::new("reviewer", AgentRole::Reviewer),
            "visual-runtime",
        )
        .with_state(TaskState::Blocked)
        .with_goal("Connect visual events back to Matter")
        .with_summary("Event transport exists but runtime callback is missing")
        .with_next_action("inspect runtime event bridge");

        assert!(!frame.is_actionable());
        assert_eq!(frame.readiness(), FrameReadiness::Blocked);
        assert_eq!(frame.missing_context(), vec!["blockers"]);
        assert!(frame.handoff_summary().contains("blockers: none"));
    }

    #[test]
    fn empty_frame_reports_missing_context() {
        let frame = AgentFrame::new(
            AgentId::new("runtime", AgentRole::Runtime),
            AgentId::new("planner", AgentRole::Planner),
            "empty",
        );

        assert_eq!(frame.readiness(), FrameReadiness::Incomplete);
        assert_eq!(
            frame.missing_context(),
            vec!["goal", "summary", "next_action"]
        );
        assert!(frame.handoff_summary().contains("goal: none"));
    }

    #[test]
    fn frame_with_declared_blocker_is_blocked() {
        let frame = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "api-runtime",
        )
        .with_goal("Expose agent frames through the API")
        .with_summary("The protocol exists but the API route is not wired")
        .add_blocker("API crate is currently dirty")
        .with_next_action("wait for clean API integration point");

        assert!(!frame.is_actionable());
        assert_eq!(frame.readiness(), FrameReadiness::Blocked);
        assert!(frame.missing_context().is_empty());
    }

    #[test]
    fn ready_frame_receives_acceptance_response() {
        let frame = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "agent-protocol",
        )
        .with_state(TaskState::InProgress)
        .with_goal("Add agent response frames")
        .with_summary("The receiver needs a formal response channel")
        .with_next_action("implement AgentResponse");

        let response = frame.response_from_receiver();

        assert_eq!(response.kind, ResponseKind::Accepted);
        assert_eq!(response.from.name, "worker");
        assert_eq!(response.to.name, "planner");
        assert_eq!(response.next_action, "execute");
        assert!(response.missing_context.is_empty());
        assert!(response.summary_text().contains("kind: accepted"));
    }

    #[test]
    fn incomplete_frame_requests_context() {
        let frame = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "missing-context",
        )
        .with_summary("The goal and next action are missing");

        let response = frame.response_from_receiver();

        assert_eq!(response.kind, ResponseKind::NeedsContext);
        assert_eq!(response.missing_context, vec!["goal", "next_action"]);
        assert_eq!(response.next_action, "request-context");
        assert!(response
            .summary_text()
            .contains("missing_context: goal; next_action"));
    }

    #[test]
    fn blocked_frame_returns_blocked_response() {
        let frame = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "blocked-agent-task",
        )
        .with_goal("Wire protocol into cloud runtime")
        .with_summary("Runtime integration point is unavailable")
        .add_blocker("cloud runtime crate is not ready")
        .with_next_action("resolve runtime integration point");

        let response = frame.response_from_receiver();

        assert_eq!(response.kind, ResponseKind::Blocked);
        assert_eq!(response.next_action, "resolve-blockers");
        assert_eq!(response.blockers, vec!["cloud runtime crate is not ready"]);
        assert!(response.summary_text().contains("kind: blocked"));
    }

    #[test]
    fn completed_response_is_terminal() {
        let response = AgentResponse::completed(
            AgentId::new("worker", AgentRole::Worker),
            AgentId::new("planner", AgentRole::Planner),
            "done",
            "Agent protocol response was implemented",
        );

        assert!(response.is_terminal());
        assert_eq!(response.kind, ResponseKind::Completed);
        assert_eq!(response.next_action, "none");
        assert!(response.summary_text().contains("kind: completed"));
    }

    #[test]
    fn session_tracks_frame_response_cycle() {
        let frame = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "handoff-cycle",
        )
        .with_goal("Create an agent session transcript")
        .with_summary("Frames and responses need a durable conversation container")
        .add_fact("AgentSession stores ordered events")
        .add_request("Return the next executable action")
        .with_next_action("append response to session");
        let response = frame.response_from_receiver();

        let session = AgentSession::new("session-1")
            .add_frame(frame)
            .add_response(response);

        assert_eq!(session.event_count(), 2);
        assert!(!session.is_terminal());
        assert_eq!(session.next_action(), "execute");
        assert_eq!(
            session.last_response().map(|response| response.kind),
            Some(ResponseKind::Accepted)
        );
        let context = session.context();
        assert_eq!(context.session_id, "session-1");
        assert_eq!(context.event_count, 2);
        assert_eq!(context.latest_task_id, "handoff-cycle");
        assert_eq!(
            context.latest_goal,
            "Create an agent session transcript"
        );
        assert_eq!(context.last_response_kind, Some(ResponseKind::Accepted));
        assert_eq!(context.next_action, "execute");
        assert_eq!(context.facts, vec!["AgentSession stores ordered events"]);
        assert_eq!(context.requests, vec!["Return the next executable action"]);
        assert!(context.blockers.is_empty());
        assert!(!context.terminal);
        assert!(session.summary_text().contains("events: 2"));
    }

    #[test]
    fn session_detects_terminal_completion() {
        let response = AgentResponse::completed(
            AgentId::new("worker", AgentRole::Worker),
            AgentId::new("planner", AgentRole::Planner),
            "handoff-cycle",
            "Session protocol completed",
        );

        let session = AgentSession::new("session-2").add_response(response);

        assert!(session.is_terminal());
        assert_eq!(session.next_action(), "none");
        let context = session.context();
        assert_eq!(context.latest_task_id, "handoff-cycle");
        assert_eq!(context.latest_summary, "Session protocol completed");
        assert_eq!(context.last_response_kind, Some(ResponseKind::Completed));
        assert!(context.terminal);
        assert!(session.summary_text().contains("terminal: true"));
    }

    #[test]
    fn empty_session_starts_without_response() {
        let session = AgentSession::new("empty-session");

        assert_eq!(session.event_count(), 0);
        assert!(session.last_response().is_none());
        assert!(!session.is_terminal());
        assert_eq!(session.next_action(), "start");
        let context = session.context();
        assert_eq!(context.event_count, 0);
        assert!(context.latest_task_id.is_empty());
        assert_eq!(context.next_action, "start");
        assert!(!context.terminal);
    }

    #[test]
    fn session_recent_events_respect_limit_and_order() {
        let frame_one = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "task-1",
        )
        .with_goal("first")
        .with_summary("first summary")
        .with_next_action("first action");
        let response_one = frame_one.response_from_receiver();

        let frame_two = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "task-2",
        )
        .with_goal("second")
        .with_summary("second summary")
        .with_next_action("second action");

        let session = AgentSession::new("session-limit")
            .add_frame(frame_one)
            .add_response(response_one)
            .add_frame(frame_two);

        let recent = session.recent_event_summaries(2);
        assert_eq!(recent.len(), 2);
        assert!(recent[0].contains("response worker -> planner (task-1) [accepted]"));
        assert!(recent[1].contains("frame planner -> worker (task-2)"));
        assert!(session.recent_event_summaries(0).is_empty());
    }

    #[test]
    fn handoff_packet_combines_context_and_recent_events() {
        let frame = AgentFrame::new(
            AgentId::new("planner", AgentRole::Planner),
            AgentId::new("worker", AgentRole::Worker),
            "handoff-packet",
        )
        .with_goal("Create portable handoff packet")
        .with_summary("Another agent should continue without replaying all history")
        .add_fact("session context exists")
        .with_next_action("generate packet");
        let response = frame.response_from_receiver();

        let session = AgentSession::new("session-packet")
            .add_frame(frame)
            .add_response(response);
        let packet = session.handoff_packet(1);

        assert_eq!(packet.context.session_id, "session-packet");
        assert_eq!(packet.context.latest_task_id, "handoff-packet");
        assert_eq!(packet.context.next_action, "execute");
        assert_eq!(packet.recent_events.len(), 1);
        assert!(packet.recent_events[0].contains("response worker -> planner"));
    }

    #[test]
    fn handoff_packet_round_trip_wire_format() {
        let session = AgentSession::new("wire-session")
            .add_frame(
                AgentFrame::new(
                    AgentId::new("planner", AgentRole::Planner),
                    AgentId::new("worker", AgentRole::Worker),
                    "wire-task",
                )
                .with_goal("Sync a|b=c")
                .with_summary("Need newline\nsafe transport")
                .add_fact("fact|one")
                .add_request("request=one")
                .with_next_action("execute"),
            )
            .add_response(AgentResponse::completed(
                AgentId::new("worker", AgentRole::Worker),
                AgentId::new("planner", AgentRole::Planner),
                "wire-task",
                "done=ok|complete",
            ));
        let packet = session.handoff_packet(3);

        let wire = packet.to_wire();
        let decoded = AgentHandoffPacket::from_wire(&wire).unwrap();

        assert_eq!(decoded, packet);
    }

    #[test]
    fn handoff_packet_rejects_unknown_version() {
        let payload = "version=2\nsession_id=s\nevent_count=0\nlatest_task_id=\nlatest_goal=\nlatest_summary=\nfacts=\nblockers=\nrequests=\nlast_response_kind=none\nnext_action=start\nterminal=false\nrecent_events=";
        let wire = format!("{}\nchecksum={}", payload, wire_checksum(payload));
        let error = AgentHandoffPacket::from_wire(&wire).unwrap_err();
        assert!(error.contains("unsupported wire version"));
    }

    #[test]
    fn handoff_packet_rejects_missing_fields() {
        let payload = "version=1\nsession_id=s";
        let wire = format!("{}\nchecksum={}", payload, wire_checksum(payload));
        let error = AgentHandoffPacket::from_wire(&wire).unwrap_err();
        assert!(error.contains("missing wire field"));
    }

    #[test]
    fn handoff_packet_rejects_checksum_mismatch() {
        let payload = "version=1\nsession_id=s\nevent_count=0\nlatest_task_id=\nlatest_goal=\nlatest_summary=\nfacts=\nblockers=\nrequests=\nlast_response_kind=none\nnext_action=start\nterminal=false\nrecent_events=";
        let wire = format!("{}\nchecksum=deadbeef", payload);
        let error = AgentHandoffPacket::from_wire(&wire).unwrap_err();
        assert!(error.contains("wire checksum mismatch"));
    }
}
