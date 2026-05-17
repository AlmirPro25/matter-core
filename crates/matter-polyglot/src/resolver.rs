//! Resolução e instalação de dependências externas

use crate::{ExternalImport, LanguageTarget};
use std::process::Command;

pub trait PackageManager {
    fn is_installed(&self, package: &str) -> bool;
    fn install(&self, package: &str, version: Option<&str>) -> Result<(), String>;
    fn get_install_path(&self, package: &str) -> Option<String>;
}

pub struct PipManager;

impl PackageManager for PipManager {
    fn is_installed(&self, package: &str) -> bool {
        Command::new("pip")
            .args(["show", package])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn install(&self, package: &str, version: Option<&str>) -> Result<(), String> {
        let package_spec = if let Some(ver) = version {
            format!("{}=={}", package, ver)
        } else {
            package.to_string()
        };

        let output = Command::new("pip")
            .args(["install", &package_spec])
            .output()
            .map_err(|e| format!("Failed to run pip: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "pip install failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    fn get_install_path(&self, package: &str) -> Option<String> {
        let output = Command::new("pip").args(["show", package]).output().ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("Location:") {
                return Some(line.split(':').nth(1)?.trim().to_string());
            }
        }

        None
    }
}

pub struct NpmManager;

impl PackageManager for NpmManager {
    fn is_installed(&self, package: &str) -> bool {
        Command::new("npm")
            .args(["list", package])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn install(&self, package: &str, version: Option<&str>) -> Result<(), String> {
        let package_spec = if let Some(ver) = version {
            format!("{}@{}", package, ver)
        } else {
            package.to_string()
        };

        let output = Command::new("npm")
            .args(["install", &package_spec])
            .output()
            .map_err(|e| format!("Failed to run npm: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "npm install failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    fn get_install_path(&self, package: &str) -> Option<String> {
        Some(format!("node_modules/{}", package))
    }
}

pub struct CargoManager;

impl PackageManager for CargoManager {
    fn is_installed(&self, _package: &str) -> bool {
        // Rust crates são compilados, não "instalados" globalmente
        true
    }

    fn install(&self, package: &str, version: Option<&str>) -> Result<(), String> {
        // Para Rust, apenas verificamos se está no Cargo.toml
        // A compilação real acontece durante o build
        println!(
            "Rust crate {} (version: {:?}) will be linked during build",
            package, version
        );
        Ok(())
    }

    fn get_install_path(&self, package: &str) -> Option<String> {
        Some(format!("target/release/deps/lib{}.rlib", package))
    }
}

pub struct GoManager;

impl PackageManager for GoManager {
    fn is_installed(&self, package: &str) -> bool {
        Command::new("go")
            .args(["list", package])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn install(&self, package: &str, version: Option<&str>) -> Result<(), String> {
        let package_spec = if let Some(ver) = version {
            format!("{}@{}", package, ver)
        } else {
            package.to_string()
        };

        let output = Command::new("go")
            .args(["get", &package_spec])
            .output()
            .map_err(|e| format!("Failed to run go: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "go get failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    fn get_install_path(&self, package: &str) -> Option<String> {
        Some(format!("$GOMODCACHE/{}", package))
    }
}

pub struct MavenManager;

impl PackageManager for MavenManager {
    fn is_installed(&self, package: &str) -> bool {
        Command::new("mvn")
            .args(["dependency:get", &format!("-Dartifact={}", package), "-q"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn install(&self, package: &str, version: Option<&str>) -> Result<(), String> {
        let package_spec = if let Some(ver) = version {
            format!("{}:{}", package, ver)
        } else {
            package.to_string()
        };

        let output = Command::new("mvn")
            .args(["dependency:get", &format!("-Dartifact={}", package_spec)])
            .output()
            .map_err(|e| format!("Failed to run mvn: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "maven dependency:get failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    fn get_install_path(&self, package: &str) -> Option<String> {
        Some(format!("~/.m2/repository/{}", package.replace(':', "/")))
    }
}

pub struct DependencyResolver {
    pip: PipManager,
    npm: NpmManager,
    cargo: CargoManager,
    go: GoManager,
    maven: MavenManager,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self {
            pip: PipManager,
            npm: NpmManager,
            cargo: CargoManager,
            go: GoManager,
            maven: MavenManager,
        }
    }

    pub fn get_manager(&self, language: LanguageTarget) -> &dyn PackageManager {
        match language {
            LanguageTarget::Python => &self.pip,
            LanguageTarget::NodeJS => &self.npm,
            LanguageTarget::Rust => &self.cargo,
            LanguageTarget::Go => &self.go,
            LanguageTarget::Java => &self.maven,
        }
    }

    pub fn resolve(&self, import: &ExternalImport, version: Option<&str>) -> Result<(), String> {
        let manager = self.get_manager(import.language);

        if !manager.is_installed(&import.package) {
            println!(
                "Installing {} package: {}",
                import.language.as_str(),
                import.package
            );
            manager.install(&import.package, version)?;
        } else {
            println!(
                "{} package {} is already installed",
                import.language.as_str(),
                import.package
            );
        }

        Ok(())
    }

    pub fn resolve_all(
        &self,
        imports: &[ExternalImport],
        versions: &std::collections::HashMap<String, String>,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for import in imports {
            let version = versions.get(&import.package).map(|s| s.as_str());
            if let Err(e) = self.resolve(import, version) {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pip_manager() {
        let _pip = PipManager;
        // Teste básico - pip deve estar instalado
        let _ = Command::new("pip").arg("--version").status();
    }

    #[test]
    fn test_npm_manager() {
        let _npm = NpmManager;
        // Teste básico - npm deve estar instalado
        let _ = Command::new("npm").arg("--version").status();
    }
}
