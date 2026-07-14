//! Capability / local-command policy (Phase 3).
//!
//! Used by the experimental CLI for process spawn. The language-only binary
//! reuses denylist helpers only — it never spawns shell/PowerShell.
//!
//! **This module is not a sandbox.** Allowlists and injection filters reduce
//! accidents; they do not provide OS-level isolation.

// Shared with experimental binary; language-only links the module for denylist + tests.
#![allow(dead_code)]

use std::env;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Default timeout for local command capture (seconds).
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
/// Default max captured stdout+stderr bytes.
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 256 * 1024;

/// Exact allowlist of development commands (experimental edition only).
pub const WHITELIST_EXACT: &[&str] = &[
    "git status --short",
    "cargo check -p matter-cli",
    "cargo check --workspace",
    "cargo test -p matter-cli -q",
    "cargo test --workspace -q",
    "cargo clippy -p matter-cli --all-targets -- -D warnings",
    "cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings",
    "git diff --stat",
];

/// Prefix allowlist: fixed prefix + one path-like argument.
pub const WHITELIST_PREFIX: &[&str] = &[
    "matter project-check-json ",
    "matter project-run-json ",
    "matter-cli project-check-json ",
    "matter-cli project-run-json ",
];

/// CLI commands that language-only must never execute (deny-by-name).
pub const LANGUAGE_ONLY_DENIED_COMMANDS: &[&str] = &[
    "agent-ui",
    "polyglot-status-json",
    "frontier-status-json",
    "world-status-json",
    "shell",
    "exec",
    "run-shell",
    "run-local",
    "powershell",
    "cmd",
    "package-install",
    "package-add",
    "pip-install",
    "npm-install",
    "net-serve",
    "net-get",
    "curl",
    "http-get",
    "bridge-python",
    "bridge-node",
    "bridge-go",
    "bridge-java",
    "tool-pipeline-demo-json",
    "terminal-snapshot",
    "agent-new-app",
    "agent-apply-plan",
];

#[derive(Debug, Clone)]
pub struct LocalCommandPolicy {
    /// When true, non-allowlisted commands may run if they pass injection checks.
    /// Requires explicit env `MATTER_ALLOW_LOCAL_COMMANDS=1` (development only).
    pub allow_arbitrary_dev: bool,
    pub timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for LocalCommandPolicy {
    fn default() -> Self {
        Self {
            allow_arbitrary_dev: false,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
        }
    }
}

impl LocalCommandPolicy {
    pub fn from_env() -> Self {
        let mut p = Self::default();
        p.allow_arbitrary_dev = env_truthy("MATTER_ALLOW_LOCAL_COMMANDS");
        if let Ok(v) = env::var("MATTER_LOCAL_COMMAND_TIMEOUT_SECS") {
            if let Ok(n) = v.parse::<u64>() {
                if n > 0 {
                    p.timeout = Duration::from_secs(n.min(3600));
                }
            }
        }
        if let Ok(v) = env::var("MATTER_LOCAL_COMMAND_MAX_OUTPUT") {
            if let Ok(n) = v.parse::<usize>() {
                if n > 0 {
                    p.max_output_bytes = n.min(16 * 1024 * 1024);
                }
            }
        }
        p
    }
}

fn env_truthy(key: &str) -> bool {
    matches!(
        env::var(key).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

/// Reject shell metacharacters / concatenation / substitution patterns.
pub fn reject_injection(command: &str) -> Result<(), String> {
    let cmd = command.trim();
    if cmd.is_empty() {
        return Err("empty command rejected".into());
    }
    const FORBIDDEN: &[char] = &[
        ';', '|', '&', '`', '\n', '\r', '<', '>', '$', '(', ')', '{', '}', '\0', '"', '\'',
    ];
    if cmd.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(format!(
            "command rejected: shell metacharacters / concatenation not allowed ({:?})",
            cmd
        ));
    }
    if cmd.contains("&&") || cmd.contains("||") {
        return Err("command rejected: unsafe sequence".into());
    }
    Ok(())
}

pub fn is_whitelisted(command: &str) -> bool {
    let cmd = command.trim();
    if WHITELIST_EXACT.iter().any(|w| *w == cmd) {
        return true;
    }
    for prefix in WHITELIST_PREFIX {
        if let Some(rest) = cmd.strip_prefix(prefix) {
            let path = rest.trim();
            if path.is_empty() {
                return false;
            }
            if path.split_whitespace().count() != 1 {
                return false;
            }
            if path.chars().any(|c| ";|&`$<>(){}\"'".contains(c)) {
                return false;
            }
            return true;
        }
    }
    false
}

pub fn is_language_only_denied_command(name: &str) -> bool {
    LANGUAGE_ONLY_DENIED_COMMANDS
        .iter()
        .any(|d| d.eq_ignore_ascii_case(name))
}

/// Capture local command output under policy (structured argv; no PowerShell).
pub fn run_local_command_capture(command: &str) -> Result<String, String> {
    let policy = LocalCommandPolicy::from_env();
    let cmd = command.trim();
    reject_injection(cmd)?;
    if !is_whitelisted(cmd) && !policy.allow_arbitrary_dev {
        return Err(format!(
            "local command denied (not on allowlist): '{}'. \
             Experimental edition is NOT a sandbox. \
             For explicit development override only, set MATTER_ALLOW_LOCAL_COMMANDS=1.",
            cmd
        ));
    }
    run_with_timeout(cmd, &policy)
}

pub fn run_whitelisted_command(command: &str) -> Result<String, String> {
    let cmd = command.trim();
    reject_injection(cmd)?;
    if !is_whitelisted(cmd) {
        return Err(format!(
            "command not allowed: '{}'. Allowlist only (NOT a sandbox). Examples: {}",
            cmd,
            WHITELIST_EXACT.join(" | ")
        ));
    }
    run_with_timeout(cmd, &LocalCommandPolicy::from_env())
}

fn run_with_timeout(cmd: &str, policy: &LocalCommandPolicy) -> Result<String, String> {
    use std::io::Read;

    let mut child_cmd = build_structured_command(cmd)?;
    child_cmd.stdin(Stdio::null());
    child_cmd.stdout(Stdio::piped());
    child_cmd.stderr(Stdio::piped());

    let mut child = child_cmd
        .spawn()
        .map_err(|e| format!("failed to spawn process: {}", e))?;

    let deadline = Instant::now() + policy.timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "local command timed out after {}s",
                        policy.timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(format!("wait failed: {}", e)),
        }
    };

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr);
    }
    let _ = child.wait();

    let keep = policy.max_output_bytes.saturating_add(1) / 2;
    if stdout.len() > keep {
        stdout.truncate(keep);
    }
    if stderr.len() > keep {
        stderr.truncate(keep);
    }

    let out_s = String::from_utf8_lossy(&stdout).to_string();
    let err_s = String::from_utf8_lossy(&stderr).to_string();

    if status.success() {
        Ok(truncate_str(&out_s, policy.max_output_bytes))
    } else if !err_s.trim().is_empty() {
        Err(truncate_str(&err_s, policy.max_output_bytes))
    } else {
        Err(truncate_str(&out_s, policy.max_output_bytes))
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…[truncated {} bytes]", &s[..max], s.len() - max)
    }
}

fn build_structured_command(cmd: &str) -> Result<Command, String> {
    let cmd = cmd.trim();

    for prefix in [
        "matter project-check-json ",
        "matter-cli project-check-json ",
    ] {
        if let Some(path) = cmd.strip_prefix(prefix) {
            return Ok(matter_self_command(&["project-check-json", path.trim()]));
        }
    }
    for prefix in ["matter project-run-json ", "matter-cli project-run-json "] {
        if let Some(path) = cmd.strip_prefix(prefix) {
            return Ok(matter_self_command(&["project-run-json", path.trim()]));
        }
    }

    let argv: Vec<&str> = match cmd {
        "git status --short" => vec!["git", "status", "--short"],
        "cargo check -p matter-cli" => vec!["cargo", "check", "-p", "matter-cli"],
        "cargo check --workspace" => vec!["cargo", "check", "--workspace"],
        "cargo test -p matter-cli -q" => vec!["cargo", "test", "-p", "matter-cli", "-q"],
        "cargo test --workspace -q" => vec!["cargo", "test", "--workspace", "-q"],
        "cargo clippy -p matter-cli --all-targets -- -D warnings" => vec![
            "cargo",
            "clippy",
            "-p",
            "matter-cli",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        "cargo clippy --workspace --exclude matter-llvm --all-targets -- -D warnings" => vec![
            "cargo",
            "clippy",
            "--workspace",
            "--exclude",
            "matter-llvm",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        "git diff --stat" => vec!["git", "diff", "--stat"],
        _ => {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                return Err("empty argv".into());
            }
            let mut c = Command::new(parts[0]);
            if parts.len() > 1 {
                c.args(&parts[1..]);
            }
            return Ok(c);
        }
    };

    let mut c = Command::new(argv[0]);
    if argv.len() > 1 {
        c.args(&argv[1..]);
    }
    Ok(c)
}

fn matter_self_command(args: &[&str]) -> Command {
    let exe = env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("matter-cli"));
    let mut c = Command::new(exe);
    c.args(args);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_semicolon_injection() {
        assert!(reject_injection("git status --short; rm -rf /").is_err());
    }

    #[test]
    fn rejects_pipe_injection() {
        assert!(reject_injection("git status | cat").is_err());
    }

    #[test]
    fn rejects_dollar_subshell() {
        assert!(reject_injection("echo $(whoami)").is_err());
    }

    #[test]
    fn rejects_ampersand_chain() {
        assert!(reject_injection("git status & cargo build").is_err());
    }

    #[test]
    fn allows_clean_whitelist_string() {
        assert!(reject_injection("git status --short").is_ok());
        assert!(is_whitelisted("git status --short"));
    }

    #[test]
    fn language_only_denies_agent_ui() {
        assert!(is_language_only_denied_command("agent-ui"));
        assert!(is_language_only_denied_command("shell"));
        assert!(is_language_only_denied_command("package-install"));
    }

    #[test]
    fn whitelist_prefix_rejects_extra_args() {
        assert!(!is_whitelisted("matter project-check-json foo bar"));
        assert!(is_whitelisted("matter project-check-json ./matter.toml"));
    }

    #[test]
    fn deny_message_mentions_not_sandbox() {
        let err = run_local_command_capture("echo pwned").unwrap_err();
        assert!(err.contains("NOT a sandbox") || err.contains("denied"));
    }
}
