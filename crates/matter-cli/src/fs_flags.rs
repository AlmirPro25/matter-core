//! Parse File Capabilities v1 CLI flags.
//!
//! No environment variable grants access. Only explicit:
//!   --allow-fs-read <dir>
//!   --allow-fs-write <dir>
//!   --allow-fs-delete <dir>

use matter_stdlib::FsCapabilityPolicy;

#[derive(Debug, Clone)]
pub struct FsCliOptions {
    pub positional: Vec<String>,
    pub policy: FsCapabilityPolicy,
}

/// Parse argv slice (typically args after the subcommand).
/// Positional non-flag args are collected in order (first is usually the source path).
pub fn parse_fs_cli_args(args: &[String]) -> Result<FsCliOptions, String> {
    let mut policy = FsCapabilityPolicy::deny_all();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--allow-fs-read" => {
                let dir = args
                    .get(i + 1)
                    .ok_or_else(|| "missing directory after --allow-fs-read".to_string())?;
                policy
                    .allow_read_root(dir)
                    .map_err(|e| format!("--allow-fs-read: {}", e))?;
                i += 2;
            }
            "--allow-fs-write" => {
                let dir = args
                    .get(i + 1)
                    .ok_or_else(|| "missing directory after --allow-fs-write".to_string())?;
                policy
                    .allow_write_root(dir)
                    .map_err(|e| format!("--allow-fs-write: {}", e))?;
                i += 2;
            }
            "--allow-fs-delete" => {
                let dir = args
                    .get(i + 1)
                    .ok_or_else(|| "missing directory after --allow-fs-delete".to_string())?;
                policy
                    .allow_delete_root(dir)
                    .map_err(|e| format!("--allow-fs-delete: {}", e))?;
                i += 2;
            }
            // Reject silent env-style pseudo-flags if someone invents them.
            flag if flag.starts_with("--allow-fs") => {
                return Err(format!(
                    "unknown filesystem capability flag '{}'. \
                     Use --allow-fs-read|write|delete <directory>",
                    flag
                ));
            }
            flag if flag.starts_with('-') => {
                // Leave unknown flags to the caller (e.g. -o for compile).
                // For run/eval/run-bytecode we treat unknown as error in the command handlers.
                positional.push(args[i].clone());
                i += 1;
            }
            _ => {
                positional.push(args[i].clone());
                i += 1;
            }
        }
    }
    Ok(FsCliOptions { positional, policy })
}

/// Split positional vs -o style for compile (compile does not need FS policy for program I/O).
pub fn strip_fs_flags_keep_rest(args: &[String]) -> Result<(Vec<String>, FsCapabilityPolicy), String> {
    let mut policy = FsCapabilityPolicy::deny_all();
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--allow-fs-read" => {
                let dir = args
                    .get(i + 1)
                    .ok_or_else(|| "missing directory after --allow-fs-read".to_string())?;
                policy.allow_read_root(dir)?;
                i += 2;
            }
            "--allow-fs-write" => {
                let dir = args
                    .get(i + 1)
                    .ok_or_else(|| "missing directory after --allow-fs-write".to_string())?;
                policy.allow_write_root(dir)?;
                i += 2;
            }
            "--allow-fs-delete" => {
                let dir = args
                    .get(i + 1)
                    .ok_or_else(|| "missing directory after --allow-fs-delete".to_string())?;
                policy.allow_delete_root(dir)?;
                i += 2;
            }
            _ => {
                rest.push(args[i].clone());
                i += 1;
            }
        }
    }
    Ok((rest, policy))
}
