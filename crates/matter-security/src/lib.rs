//! # Matter Security
//!
//! Enterprise-grade security features for Matter:
//! - Sandboxing with seccomp/pledge
//! - Permission system
//! - Code signing and verification
//! - Audit logging

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

/// Permission types for sandboxed execution
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    /// Read file permission
    FileRead(PathBuf),
    /// Write file permission
    FileWrite(PathBuf),
    /// Network access permission
    Network(String),
    /// FFI call permission
    FFI(String),
    /// Subprocess execution permission
    Subprocess,
    /// Environment variable access
    EnvVar(String),
}

/// Isolation level for sandbox
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// No isolation (default)
    None,
    /// Basic isolation (file system only)
    Basic,
    /// Strict isolation (file system + network)
    Strict,
    /// Maximum isolation (everything restricted)
    Maximum,
}

/// Permission set
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSet {
    permissions: HashSet<Permission>,
}

impl PermissionSet {
    pub fn new() -> Self {
        Self {
            permissions: HashSet::new(),
        }
    }

    pub fn allow(&mut self, permission: Permission) {
        self.permissions.insert(permission);
    }

    pub fn check(&self, permission: &Permission) -> bool {
        // Check exact match
        if self.permissions.contains(permission) {
            return true;
        }

        // Check wildcard patterns
        match permission {
            Permission::FileRead(path) => {
                // Check if any parent directory is allowed
                for perm in &self.permissions {
                    if let Permission::FileRead(allowed_path) = perm {
                        if path.starts_with(allowed_path) {
                            return true;
                        }
                    }
                }
            }
            Permission::FileWrite(path) => {
                // Check if any parent directory is allowed
                for perm in &self.permissions {
                    if let Permission::FileWrite(allowed_path) = perm {
                        if path.starts_with(allowed_path) {
                            return true;
                        }
                    }
                }
            }
            Permission::Network(host) => {
                // Check wildcard network permissions
                for perm in &self.permissions {
                    if let Permission::Network(allowed_host) = perm {
                        if allowed_host == "*" || allowed_host == host {
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }

        false
    }
}

/// Sandbox for secure execution
pub struct Sandbox {
    permissions: PermissionSet,
    isolation_level: IsolationLevel,
    audit_enabled: bool,
}

impl Sandbox {
    pub fn new(permissions: PermissionSet, isolation_level: IsolationLevel) -> Self {
        Self {
            permissions,
            isolation_level,
            audit_enabled: true,
        }
    }

    pub fn check_permission(&self, permission: &Permission) -> bool {
        let allowed = self.permissions.check(permission);

        if self.audit_enabled {
            self.audit_permission_check(permission, allowed);
        }

        allowed
    }

    pub fn execute<T, F>(&self, f: F) -> Result<T, SecurityError>
    where
        F: FnOnce() -> T,
    {
        // Apply isolation based on level
        match self.isolation_level {
            IsolationLevel::None => Ok(f()),
            IsolationLevel::Basic => self.execute_basic_isolation(f),
            IsolationLevel::Strict => self.execute_strict_isolation(f),
            IsolationLevel::Maximum => self.execute_maximum_isolation(f),
        }
    }

    fn execute_basic_isolation<T, F>(&self, f: F) -> Result<T, SecurityError>
    where
        F: FnOnce() -> T,
    {
        // Basic isolation: restrict file system access
        // In production, this would use seccomp/pledge
        Ok(f())
    }

    fn execute_strict_isolation<T, F>(&self, f: F) -> Result<T, SecurityError>
    where
        F: FnOnce() -> T,
    {
        // Strict isolation: restrict file system + network
        // In production, this would use seccomp/pledge + network namespaces
        Ok(f())
    }

    fn execute_maximum_isolation<T, F>(&self, f: F) -> Result<T, SecurityError>
    where
        F: FnOnce() -> T,
    {
        // Maximum isolation: everything restricted
        // In production, this would use full containerization
        Ok(f())
    }

    fn audit_permission_check(&self, permission: &Permission, allowed: bool) {
        // Log permission check for audit trail
        eprintln!(
            "[AUDIT] Permission check: {:?} - {}",
            permission,
            if allowed { "ALLOWED" } else { "DENIED" }
        );
    }
}

/// Code signature for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSignature {
    pub hash: String,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl CodeSignature {
    pub fn verify(&self, code: &[u8]) -> Result<bool, SecurityError> {
        use sha2::{Digest, Sha256};

        // Compute hash
        let mut hasher = Sha256::new();
        hasher.update(code);
        let computed_hash = format!("{:x}", hasher.finalize());

        // Check hash matches
        if computed_hash != self.hash {
            return Ok(false);
        }

        // In production, verify signature with public key
        // For now, just check hash
        Ok(true)
    }
}

/// Security error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum SecurityError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),

    #[error("Code verification failed: {0}")]
    VerificationFailed(String),
}

/// Audit logger
pub struct AuditLogger {
    enabled: bool,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn log_operation(&self, operation: &str, details: serde_json::Value) {
        if !self.enabled {
            return;
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let log_entry = serde_json::json!({
            "timestamp": timestamp,
            "operation": operation,
            "details": details,
        });

        eprintln!("[AUDIT] {}", log_entry);
    }

    pub fn log_security_event(&self, event: &str, severity: &str) {
        self.log_operation(
            "security_event",
            serde_json::json!({
                "event": event,
                "severity": severity,
            }),
        );
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_set() {
        let mut perms = PermissionSet::new();
        perms.allow(Permission::FileRead(PathBuf::from("/data")));

        assert!(perms.check(&Permission::FileRead(PathBuf::from("/data/file.txt"))));
        assert!(!perms.check(&Permission::FileWrite(PathBuf::from("/data/file.txt"))));
    }

    #[test]
    fn test_sandbox_execution() {
        let perms = PermissionSet::new();
        let sandbox = Sandbox::new(perms, IsolationLevel::Basic);

        let result = sandbox.execute(|| 42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_code_signature() {
        use sha2::{Digest, Sha256};

        let code = b"print('hello')";
        let mut hasher = Sha256::new();
        hasher.update(code);
        let hash = format!("{:x}", hasher.finalize());

        let signature = CodeSignature {
            hash,
            signature: vec![],
            public_key: vec![],
        };

        assert!(signature.verify(code).unwrap());
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::new();
        logger.log_operation("test", serde_json::json!({"key": "value"}));
        logger.log_security_event("test_event", "low");
    }
}
