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
}
