/// Matter Package Manager
/// Sistema de pacotes, dependências e versionamento

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Versão semântica
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", s));
        }

        let major = parts[0].parse().map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1].parse().map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2].parse().map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Self { major, minor, patch })
    }

    pub fn is_compatible(&self, requirement: &VersionReq) -> bool {
        match requirement {
            VersionReq::Exact(v) => self == v,
            VersionReq::Caret(v) => {
                // ^1.2.3 = >= 1.2.3, < 2.0.0
                self >= v && self.major == v.major
            }
            VersionReq::Tilde(v) => {
                // ~1.2.3 = >= 1.2.3, < 1.3.0
                self >= v && self.major == v.major && self.minor == v.minor
            }
            VersionReq::Range(min, max) => {
                self >= min && self < max
            }
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Requisito de versão
#[derive(Debug, Clone, PartialEq)]
pub enum VersionReq {
    Exact(Version),
    Caret(Version),  // ^1.0.0
    Tilde(Version),  // ~1.0.0
    Range(Version, Version),
}

impl VersionReq {
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.starts_with('^') {
            let version = Version::parse(&s[1..])?;
            Ok(VersionReq::Caret(version))
        } else if s.starts_with('~') {
            let version = Version::parse(&s[1..])?;
            Ok(VersionReq::Tilde(version))
        } else {
            let version = Version::parse(s)?;
            Ok(VersionReq::Exact(version))
        }
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionReq::Exact(v) => write!(f, "{}", v),
            VersionReq::Caret(v) => write!(f, "^{}", v),
            VersionReq::Tilde(v) => write!(f, "~{}", v),
            VersionReq::Range(min, max) => write!(f, ">= {}, < {}", min, max),
        }
    }
}

/// Manifesto do pacote (matter.toml)
#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: Version,
    pub authors: Vec<String>,
    pub description: String,
    pub license: String,
    pub entry: String,
    pub dependencies: HashMap<String, Dependency>,
}

impl Manifest {
    pub fn new(name: String, version: Version) -> Self {
        Self {
            name,
            version,
            authors: Vec::new(),
            description: String::new(),
            license: "MIT".to_string(),
            entry: "src/main.matter".to_string(),
            dependencies: HashMap::new(),
        }
    }

    pub fn parse(content: &str) -> Result<Self, String> {
        let mut name = String::new();
        let mut version = Version::new(0, 1, 0);
        let authors = Vec::new();
        let mut description = String::new();
        let mut license = "MIT".to_string();
        let mut entry = "src/main.matter".to_string();
        let mut dependencies = HashMap::new();

        let mut current_section = "";

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_section = &line[1..line.len()-1];
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match current_section {
                    "package" => {
                        match key {
                            "name" => name = value.to_string(),
                            "version" => version = Version::parse(value)?,
                            "description" => description = value.to_string(),
                            "license" => license = value.to_string(),
                            "entry" => entry = value.to_string(),
                            _ => {}
                        }
                    }
                    "dependencies" => {
                        let dep = if value.starts_with('{') {
                            // Complex dependency: { version = "1.0.0", path = "../lib" }
                            Dependency::Path(PathBuf::from(value))
                        } else {
                            // Simple version: "1.0.0"
                            let version_req = VersionReq::parse(value)?;
                            Dependency::Registry(version_req)
                        };
                        dependencies.insert(key.to_string(), dep);
                    }
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            return Err("Package name is required".to_string());
        }

        Ok(Self {
            name,
            version,
            authors,
            description,
            license,
            entry,
            dependencies,
        })
    }

    pub fn to_toml(&self) -> String {
        let mut toml = String::new();

        toml.push_str("[package]\n");
        toml.push_str(&format!("name = \"{}\"\n", self.name));
        toml.push_str(&format!("version = \"{}\"\n", self.version));
        if !self.description.is_empty() {
            toml.push_str(&format!("description = \"{}\"\n", self.description));
        }
        toml.push_str(&format!("license = \"{}\"\n", self.license));
        toml.push_str(&format!("entry = \"{}\"\n", self.entry));
        toml.push('\n');

        if !self.dependencies.is_empty() {
            toml.push_str("[dependencies]\n");
            for (name, dep) in &self.dependencies {
                match dep {
                    Dependency::Registry(req) => {
                        toml.push_str(&format!("{} = \"{}\"\n", name, req));
                    }
                    Dependency::Path(path) => {
                        toml.push_str(&format!("{} = {{ path = \"{}\" }}\n", name, path.display()));
                    }
                }
            }
        }

        toml
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let toml = self.to_toml();
        fs::write(path, toml).map_err(|e| e.to_string())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::parse(&content)
    }
}

/// Dependência
#[derive(Debug, Clone)]
pub enum Dependency {
    Registry(VersionReq),
    Path(PathBuf),
}

/// Pacote
#[derive(Debug, Clone)]
pub struct Package {
    pub manifest: Manifest,
    pub root: PathBuf,
}

impl Package {
    pub fn new(manifest: Manifest, root: PathBuf) -> Self {
        Self { manifest, root }
    }

    pub fn load(root: &Path) -> Result<Self, String> {
        let manifest_path = root.join("matter.toml");
        let manifest = Manifest::load(&manifest_path)?;
        Ok(Self {
            manifest,
            root: root.to_path_buf(),
        })
    }

    pub fn entry_path(&self) -> PathBuf {
        self.root.join(&self.manifest.entry)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockEntry {
    pub name: String,
    pub version: Version,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lockfile {
    pub package: String,
    pub version: Version,
    pub entries: Vec<LockEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReport {
    pub lockfile: Lockfile,
    pub installed: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub lockfile: Lockfile,
    pub installed: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    pub verified: Vec<PathBuf>,
}

impl SyncReport {
    pub fn summary(&self) -> String {
        [
            format!("lockfile: {}", self.lockfile.entries.len()),
            format!("installed: {}", self.installed.len()),
            format!("removed: {}", self.removed.len()),
            format!("verified: {}", self.verified.len()),
        ]
        .join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageStatus {
    pub lockfile_ok: bool,
    pub installation_ok: bool,
    pub imports_ok: bool,
    pub errors: Vec<String>,
}

impl PackageStatus {
    pub fn is_ready(&self) -> bool {
        self.lockfile_ok && self.installation_ok && self.imports_ok && self.errors.is_empty()
    }

    pub fn summary(&self) -> String {
        let mut lines = vec![
            format!("lockfile: {}", status_word(self.lockfile_ok)),
            format!("installation: {}", status_word(self.installation_ok)),
            format!("imports: {}", status_word(self.imports_ok)),
        ];

        if self.errors.is_empty() {
            lines.push("errors: none".to_string());
        } else {
            lines.push("errors:".to_string());
            for error in &self.errors {
                lines.push(format!("- {}", error));
            }
        }

        lines.join("\n")
    }
}

fn status_word(ok: bool) -> &'static str {
    if ok {
        "ok"
    } else {
        "error"
    }
}

impl Lockfile {
    pub fn new(package: &Package, mut entries: Vec<LockEntry>) -> Self {
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        Self {
            package: package.manifest.name.clone(),
            version: package.manifest.version.clone(),
            entries,
        }
    }

    pub fn to_toml(&self) -> String {
        let mut toml = String::new();
        toml.push_str("[package]\n");
        toml.push_str(&format!("name = \"{}\"\n", self.package));
        toml.push_str(&format!("version = \"{}\"\n\n", self.version));
        toml.push_str("[[dependencies]]\n");
        if self.entries.is_empty() {
            toml.push_str("# no resolved dependencies\n");
            return toml;
        }

        for (index, entry) in self.entries.iter().enumerate() {
            if index > 0 {
                toml.push_str("\n[[dependencies]]\n");
            }
            toml.push_str(&format!("name = \"{}\"\n", entry.name));
            toml.push_str(&format!("version = \"{}\"\n", entry.version));
            toml.push_str(&format!("source = \"{}\"\n", entry.source));
        }
        toml
    }

    pub fn parse(content: &str) -> Result<Self, String> {
        let mut package = String::new();
        let mut version = Version::new(0, 0, 0);
        let mut entries = Vec::new();
        let mut current_section = "";
        let mut current_entry: Option<LockEntry> = None;

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line == "[package]" {
                if let Some(entry) = current_entry.take() {
                    entries.push(entry);
                }
                current_section = "package";
                continue;
            }

            if line == "[[dependencies]]" {
                if let Some(entry) = current_entry.take() {
                    entries.push(entry);
                }
                current_section = "dependencies";
                current_entry = Some(LockEntry {
                    name: String::new(),
                    version: Version::new(0, 0, 0),
                    source: String::new(),
                });
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"');

            match current_section {
                "package" => match key {
                    "name" => package = value.to_string(),
                    "version" => version = Version::parse(value)?,
                    _ => {}
                },
                "dependencies" => {
                    let entry = current_entry
                        .as_mut()
                        .ok_or_else(|| "matter.lock dependency field outside dependency block".to_string())?;
                    match key {
                        "name" => entry.name = value.to_string(),
                        "version" => entry.version = Version::parse(value)?,
                        "source" => entry.source = value.to_string(),
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if let Some(entry) = current_entry {
            entries.push(entry);
        }

        if package.is_empty() {
            return Err("matter.lock package name is required".to_string());
        }

        entries.retain(|entry| !entry.name.is_empty());
        entries.sort_by(|left, right| left.name.cmp(&right.name));

        Ok(Self {
            package,
            version,
            entries,
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        fs::write(path, self.to_toml()).map_err(|e| e.to_string())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        Self::parse(&content)
    }
}

/// Package Manager
pub struct PackageManager {
    registry: Registry,
}

impl PackageManager {
    pub fn new() -> Self {
        Self {
            registry: Registry::new(),
        }
    }

    pub fn create_package(&self, name: &str, path: &Path, is_lib: bool) -> Result<Package, String> {
        // Create directory structure
        fs::create_dir_all(path).map_err(|e| e.to_string())?;

        let src_dir = path.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| e.to_string())?;

        // Create manifest
        let mut manifest = Manifest::new(name.to_string(), Version::new(0, 1, 0));
        manifest.entry = if is_lib {
            "src/lib.matter".to_string()
        } else {
            "src/main.matter".to_string()
        };

        // Save manifest
        let manifest_path = path.join("matter.toml");
        manifest.save(&manifest_path)?;

        // Create entry file
        let entry_path = path.join(&manifest.entry);
        let entry_content = if is_lib {
            "# Library\n\nfn hello() {\n    return \"Hello from library!\"\n}\n"
        } else {
            "# Main\n\nprint \"Hello, Matter!\"\n"
        };
        fs::write(&entry_path, entry_content).map_err(|e| e.to_string())?;

        // Create README
        let readme_path = path.join("README.md");
        let readme_content = format!("# {}\n\n{}\n", name, manifest.description);
        fs::write(&readme_path, readme_content).map_err(|e| e.to_string())?;

        Ok(Package::new(manifest, path.to_path_buf()))
    }

    pub fn add_dependency(&self, package: &mut Package, name: &str, version: &str) -> Result<(), String> {
        let version_req = VersionReq::parse(version)?;
        package.manifest.dependencies.insert(name.to_string(), Dependency::Registry(version_req));

        let manifest_path = package.root.join("matter.toml");
        package.manifest.save(&manifest_path)?;

        Ok(())
    }

    pub fn register_package(&mut self, package: Package) {
        self.registry.register(package);
    }

    pub fn resolve_dependencies(&self, package: &Package) -> Result<Vec<Package>, String> {
        let mut resolved = Vec::new();

        for (name, dep) in &package.manifest.dependencies {
            match dep {
                Dependency::Registry(req) => {
                    if let Some(pkg) = self.registry.find_package(name, req) {
                        resolved.push(pkg);
                    } else {
                        return Err(format!("Package '{}' not found in registry", name));
                    }
                }
                Dependency::Path(path) => {
                    let dep_path = package.root.join(path);
                    let dep_pkg = Package::load(&dep_path)?;
                    resolved.push(dep_pkg);
                }
            }
        }

        Ok(resolved)
    }

    pub fn lock_dependencies(&self, package: &Package) -> Result<Lockfile, String> {
        let resolved = self.resolve_dependencies(package)?;
        let entries = resolved
            .into_iter()
            .map(|resolved_package| LockEntry {
                name: resolved_package.manifest.name,
                version: resolved_package.manifest.version,
                source: resolved_package.root.display().to_string(),
            })
            .collect();

        Ok(Lockfile::new(package, entries))
    }

    pub fn save_lockfile(&self, package: &Package) -> Result<Lockfile, String> {
        let lockfile = self.lock_dependencies(package)?;
        lockfile.save(&package.root.join("matter.lock"))?;
        Ok(lockfile)
    }

    pub fn ensure_lockfile(&self, package: &Package) -> Result<Lockfile, String> {
        let lock_path = package.root.join("matter.lock");
        if lock_path.exists() {
            let lockfile = Lockfile::load(&lock_path)?;
            self.verify_lockfile(package, &lockfile)?;
            Ok(lockfile)
        } else {
            self.save_lockfile(package)
        }
    }

    pub fn install_dependencies(&self, package: &Package) -> Result<InstallReport, String> {
        let lockfile = self.ensure_lockfile(package)?;
        let resolved = self.resolve_dependencies(package)?;
        let install_root = package.root.join(".matter").join("packages");
        fs::create_dir_all(&install_root).map_err(|e| e.to_string())?;

        let mut installed = Vec::new();
        for dependency in resolved {
            let target = install_root.join(&dependency.manifest.name);
            if target.exists() {
                fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
            }
            copy_dir_all(&dependency.root, &target)?;
            installed.push(target);
        }
        installed.sort();

        Ok(InstallReport {
            lockfile,
            installed,
        })
    }

    pub fn sync_dependencies(&self, package: &Package) -> Result<SyncReport, String> {
        let install_report = self.install_dependencies(package)?;
        let removed = self.prune_installed_packages(package)?;
        let verified = self.verify_installation(package)?;

        Ok(SyncReport {
            lockfile: install_report.lockfile,
            installed: install_report.installed,
            removed,
            verified,
        })
    }

    pub fn package_status(&self, package: &Package) -> PackageStatus {
        let mut errors = Vec::new();

        let lockfile_ok = match Lockfile::load(&package.root.join("matter.lock")) {
            Ok(lockfile) => match self.verify_lockfile(package, &lockfile) {
                Ok(()) => true,
                Err(error) => {
                    errors.push(error);
                    false
                }
            },
            Err(error) => {
                errors.push(format!("matter.lock unavailable: {}", error));
                false
            }
        };

        let installation_ok = match self.verify_installation(package) {
            Ok(_) => true,
            Err(error) => {
                errors.push(error);
                false
            }
        };

        let imports_ok = match self.resolve_all_imports(package) {
            Ok(_) => true,
            Err(error) => {
                errors.push(error);
                false
            }
        };

        PackageStatus {
            lockfile_ok,
            installation_ok,
            imports_ok,
            errors,
        }
    }

    pub fn resolve_import(&self, package: &Package, name: &str) -> Result<PathBuf, String> {
        if !package.manifest.dependencies.contains_key(name) {
            return Err(format!(
                "Package '{}' does not declare dependency '{}'",
                package.manifest.name, name
            ));
        }

        let installed_root = package.root.join(".matter").join("packages").join(name);
        if !installed_root.exists() {
            return Err(format!(
                "Dependency '{}' is not installed; run package install first",
                name
            ));
        }

        let dependency = Package::load(&installed_root)?;
        let entry_path = dependency.entry_path();
        if !entry_path.exists() {
            return Err(format!(
                "Dependency '{}' entry '{}' does not exist",
                name,
                dependency.manifest.entry
            ));
        }

        Ok(entry_path)
    }

    pub fn resolve_all_imports(&self, package: &Package) -> Result<HashMap<String, PathBuf>, String> {
        let mut imports = HashMap::new();
        let mut names = package
            .manifest
            .dependencies
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        names.sort();

        for name in names {
            let entry_path = self.resolve_import(package, &name)?;
            imports.insert(name, entry_path);
        }

        Ok(imports)
    }

    pub fn verify_installation(&self, package: &Package) -> Result<Vec<PathBuf>, String> {
        let lockfile = Lockfile::load(&package.root.join("matter.lock"))?;
        self.verify_lockfile(package, &lockfile)?;

        let mut installed = Vec::new();
        for entry in lockfile.entries {
            let installed_root = package
                .root
                .join(".matter")
                .join("packages")
                .join(&entry.name);
            if !installed_root.exists() {
                return Err(format!("Dependency '{}' is missing from .matter/packages", entry.name));
            }

            let dependency = Package::load(&installed_root)?;
            if dependency.manifest.version != entry.version {
                return Err(format!(
                    "Dependency '{}' installed version {} does not match lock {}",
                    entry.name, dependency.manifest.version, entry.version
                ));
            }

            let entry_path = dependency.entry_path();
            if !entry_path.exists() {
                return Err(format!(
                    "Dependency '{}' installed entry '{}' is missing",
                    entry.name, dependency.manifest.entry
                ));
            }
            installed.push(installed_root);
        }
        installed.sort();

        Ok(installed)
    }

    pub fn prune_installed_packages(&self, package: &Package) -> Result<Vec<PathBuf>, String> {
        let lockfile = Lockfile::load(&package.root.join("matter.lock"))?;
        self.verify_lockfile(package, &lockfile)?;

        let expected = lockfile
            .entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<HashSet<_>>();
        let install_root = package.root.join(".matter").join("packages");
        if !install_root.exists() {
            return Ok(Vec::new());
        }

        let mut removed = Vec::new();
        for entry in fs::read_dir(&install_root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !expected.contains(name) {
                fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
                removed.push(path);
            }
        }
        removed.sort();

        Ok(removed)
    }

    pub fn verify_lockfile(&self, package: &Package, lockfile: &Lockfile) -> Result<(), String> {
        let expected = self.lock_dependencies(package)?;
        if &expected == lockfile {
            return Ok(());
        }

        let expected_names = expected
            .entries
            .iter()
            .map(|entry| format!("{}@{}", entry.name, entry.version))
            .collect::<Vec<_>>()
            .join(", ");
        let actual_names = lockfile
            .entries
            .iter()
            .map(|entry| format!("{}@{}", entry.name, entry.version))
            .collect::<Vec<_>>()
            .join(", ");

        Err(format!(
            "matter.lock is stale for package '{}': expected [{}], got [{}]",
            package.manifest.name, expected_names, actual_names
        ))
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let entry_path = entry.path();
        let target_path = destination.join(entry.file_name());
        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &target_path)?;
        } else {
            fs::copy(&entry_path, &target_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

impl Default for PackageManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry local
pub struct Registry {
    packages: HashMap<String, Vec<Package>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    pub fn register(&mut self, package: Package) {
        let name = package.manifest.name.clone();
        self.packages.entry(name).or_insert_with(Vec::new).push(package);
    }

    pub fn find_package(&self, name: &str, req: &VersionReq) -> Option<Package> {
        if let Some(packages) = self.packages.get(name) {
            for pkg in packages {
                if pkg.manifest.version.is_compatible(req) {
                    return Some(pkg.clone());
                }
            }
        }
        None
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_compatibility() {
        let v = Version::new(1, 2, 3);

        // Exact
        let req = VersionReq::Exact(Version::new(1, 2, 3));
        assert!(v.is_compatible(&req));

        // Caret
        let req = VersionReq::Caret(Version::new(1, 0, 0));
        assert!(v.is_compatible(&req));

        // Tilde
        let req = VersionReq::Tilde(Version::new(1, 2, 0));
        assert!(v.is_compatible(&req));
    }

    #[test]
    fn test_version_req_parse() {
        let req = VersionReq::parse("^1.0.0").unwrap();
        assert!(matches!(req, VersionReq::Caret(_)));

        let req = VersionReq::parse("~1.2.0").unwrap();
        assert!(matches!(req, VersionReq::Tilde(_)));

        let req = VersionReq::parse("1.0.0").unwrap();
        assert!(matches!(req, VersionReq::Exact(_)));
    }

    #[test]
    fn test_manifest_parse() {
        let toml = r#"
[package]
name = "test-package"
version = "0.1.0"
description = "A test package"
license = "MIT"
entry = "src/main.matter"

[dependencies]
math-utils = "^1.0.0"
"#;

        let manifest = Manifest::parse(toml).unwrap();
        assert_eq!(manifest.name, "test-package");
        assert_eq!(manifest.version, Version::new(0, 1, 0));
        assert_eq!(manifest.dependencies.len(), 1);
    }

    #[test]
    fn test_manifest_to_toml() {
        let mut manifest = Manifest::new("test".to_string(), Version::new(0, 1, 0));
        manifest.description = "Test package".to_string();

        let toml = manifest.to_toml();
        assert!(toml.contains("name = \"test\""));
        assert!(toml.contains("version = \"0.1.0\""));
    }

    #[test]
    fn test_lock_dependencies_writes_stable_lockfile() {
        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, PathBuf::from("app"));

        let utils = Package::new(
            Manifest::new("utils".to_string(), Version::new(1, 2, 0)),
            PathBuf::from("registry/utils"),
        );

        let mut manager = PackageManager::new();
        manager.register_package(utils);
        let lockfile = manager.lock_dependencies(&app).unwrap();
        let toml = lockfile.to_toml();

        assert!(toml.contains("name = \"app\""));
        assert!(toml.contains("[[dependencies]]"));
        assert!(toml.contains("name = \"utils\""));
        assert!(toml.contains("version = \"1.2.0\""));
        assert!(toml.contains("source = \"registry/utils\""));
    }

    #[test]
    fn test_lockfile_round_trips_through_toml() {
        let lockfile = Lockfile {
            package: "app".to_string(),
            version: Version::new(0, 1, 0),
            entries: vec![LockEntry {
                name: "utils".to_string(),
                version: Version::new(1, 2, 0),
                source: "registry/utils".to_string(),
            }],
        };

        let parsed = Lockfile::parse(&lockfile.to_toml()).unwrap();
        assert_eq!(parsed, lockfile);
    }

    #[test]
    fn test_lockfile_loads_from_disk() {
        let path = std::env::temp_dir().join("matter_package_lock_load_test.lock");
        let lockfile = Lockfile {
            package: "app".to_string(),
            version: Version::new(0, 1, 0),
            entries: vec![LockEntry {
                name: "utils".to_string(),
                version: Version::new(1, 2, 0),
                source: "registry/utils".to_string(),
            }],
        };

        lockfile.save(&path).unwrap();
        let loaded = Lockfile::load(&path).unwrap();
        assert_eq!(loaded, lockfile);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_ensure_lockfile_creates_and_verifies_existing_lock() {
        let root = std::env::temp_dir().join("matter_package_ensure_lock_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());

        let utils = Package::new(
            Manifest::new("utils".to_string(), Version::new(1, 2, 0)),
            PathBuf::from("registry/utils"),
        );

        let mut manager = PackageManager::new();
        manager.register_package(utils);
        let created = manager.ensure_lockfile(&app).unwrap();
        assert!(root.join("matter.lock").exists());

        let verified = manager.ensure_lockfile(&app).unwrap();
        assert_eq!(verified, created);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_ensure_lockfile_rejects_stale_disk_lock() {
        let root = std::env::temp_dir().join("matter_package_stale_lock_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());

        let utils = Package::new(
            Manifest::new("utils".to_string(), Version::new(1, 2, 0)),
            PathBuf::from("registry/utils"),
        );
        let stale = Lockfile::new(
            &app,
            vec![LockEntry {
                name: "utils".to_string(),
                version: Version::new(1, 0, 0),
                source: "registry/utils".to_string(),
            }],
        );
        stale.save(&root.join("matter.lock")).unwrap();

        let mut manager = PackageManager::new();
        manager.register_package(utils);
        let error = manager.ensure_lockfile(&app).unwrap_err();
        assert!(error.contains("matter.lock is stale"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_install_dependencies_materializes_local_packages() {
        let root = std::env::temp_dir().join("matter_package_install_test");
        let registry_root = std::env::temp_dir().join("matter_package_install_registry");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&registry_root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(registry_root.join("utils").join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let utils_manifest = Manifest::new("utils".to_string(), Version::new(1, 2, 0));
        let utils_root = registry_root.join("utils");
        utils_manifest.save(&utils_root.join("matter.toml")).unwrap();
        fs::write(utils_root.join("src").join("lib.matter"), "fn util() { return 1 }\n").unwrap();
        let utils = Package::new(utils_manifest, utils_root.clone());

        let mut manager = PackageManager::new();
        manager.register_package(utils);
        let report = manager.install_dependencies(&app).unwrap();

        let installed_root = root.join(".matter").join("packages").join("utils");
        assert_eq!(report.installed, vec![installed_root.clone()]);
        assert!(root.join("matter.lock").exists());
        assert!(installed_root.join("matter.toml").exists());
        assert!(installed_root.join("src").join("lib.matter").exists());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(registry_root);
    }

    #[test]
    fn test_resolve_import_returns_installed_entry_path() {
        let root = std::env::temp_dir().join("matter_package_import_test");
        let registry_root = std::env::temp_dir().join("matter_package_import_registry");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&registry_root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(registry_root.join("utils").join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let mut utils_manifest = Manifest::new("utils".to_string(), Version::new(1, 2, 0));
        utils_manifest.entry = "src/lib.matter".to_string();
        let utils_root = registry_root.join("utils");
        utils_manifest.save(&utils_root.join("matter.toml")).unwrap();
        fs::write(utils_root.join("src").join("lib.matter"), "fn util() { return 1 }\n").unwrap();
        let utils = Package::new(utils_manifest, utils_root.clone());

        let mut manager = PackageManager::new();
        manager.register_package(utils);
        manager.install_dependencies(&app).unwrap();

        let import_path = manager.resolve_import(&app, "utils").unwrap();
        assert_eq!(
            import_path,
            root.join(".matter")
                .join("packages")
                .join("utils")
                .join("src")
                .join("lib.matter")
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(registry_root);
    }

    #[test]
    fn test_resolve_import_requires_declared_dependency() {
        let app = Package::new(
            Manifest::new("app".to_string(), Version::new(0, 1, 0)),
            PathBuf::from("app"),
        );
        let manager = PackageManager::new();
        let error = manager.resolve_import(&app, "utils").unwrap_err();
        assert!(error.contains("does not declare dependency"));
    }

    #[test]
    fn test_resolve_all_imports_returns_declared_dependency_entries() {
        let root = std::env::temp_dir().join("matter_package_all_imports_test");
        let registry_root = std::env::temp_dir().join("matter_package_all_imports_registry");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&registry_root);
        fs::create_dir_all(root.join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        for name in ["math", "utils"] {
            app_manifest.dependencies.insert(
                name.to_string(),
                Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
            );
        }
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let mut manager = PackageManager::new();
        for name in ["math", "utils"] {
            let package_root = registry_root.join(name);
            fs::create_dir_all(package_root.join("src")).unwrap();
            let mut manifest = Manifest::new(name.to_string(), Version::new(1, 0, 0));
            manifest.entry = "src/lib.matter".to_string();
            manifest.save(&package_root.join("matter.toml")).unwrap();
            fs::write(
                package_root.join("src").join("lib.matter"),
                format!("fn {}() {{ return 1 }}\n", name),
            )
            .unwrap();
            manager.register_package(Package::new(manifest, package_root));
        }

        manager.install_dependencies(&app).unwrap();
        let imports = manager.resolve_all_imports(&app).unwrap();

        assert_eq!(imports.len(), 2);
        assert_eq!(
            imports.get("math"),
            Some(
                &root
                    .join(".matter")
                    .join("packages")
                    .join("math")
                    .join("src")
                    .join("lib.matter")
            )
        );
        assert_eq!(
            imports.get("utils"),
            Some(
                &root
                    .join(".matter")
                    .join("packages")
                    .join("utils")
                    .join("src")
                    .join("lib.matter")
            )
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(registry_root);
    }

    #[test]
    fn test_verify_installation_accepts_installed_locked_packages() {
        let root = std::env::temp_dir().join("matter_package_verify_install_test");
        let registry_root = std::env::temp_dir().join("matter_package_verify_install_registry");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&registry_root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(registry_root.join("utils").join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let mut utils_manifest = Manifest::new("utils".to_string(), Version::new(1, 2, 0));
        utils_manifest.entry = "src/lib.matter".to_string();
        let utils_root = registry_root.join("utils");
        utils_manifest.save(&utils_root.join("matter.toml")).unwrap();
        fs::write(utils_root.join("src").join("lib.matter"), "fn util() { return 1 }\n").unwrap();

        let mut manager = PackageManager::new();
        manager.register_package(Package::new(utils_manifest, utils_root));
        manager.install_dependencies(&app).unwrap();

        let installed = manager.verify_installation(&app).unwrap();
        assert_eq!(
            installed,
            vec![root.join(".matter").join("packages").join("utils")]
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(registry_root);
    }

    #[test]
    fn test_verify_installation_rejects_missing_installed_package() {
        let root = std::env::temp_dir().join("matter_package_missing_install_test");
        let registry_root = std::env::temp_dir().join("matter_package_missing_install_registry");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&registry_root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(registry_root.join("utils").join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let utils_manifest = Manifest::new("utils".to_string(), Version::new(1, 2, 0));
        let utils_root = registry_root.join("utils");
        utils_manifest.save(&utils_root.join("matter.toml")).unwrap();
        fs::write(utils_root.join("src").join("main.matter"), "print 1\n").unwrap();

        let mut manager = PackageManager::new();
        manager.register_package(Package::new(utils_manifest, utils_root));
        manager.install_dependencies(&app).unwrap();
        fs::remove_dir_all(root.join(".matter").join("packages").join("utils")).unwrap();

        let error = manager.verify_installation(&app).unwrap_err();
        assert!(error.contains("missing from .matter/packages"));

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(registry_root);
    }

    #[test]
    fn test_prune_installed_packages_removes_unlocked_directories() {
        let root = std::env::temp_dir().join("matter_package_prune_test");
        let registry_root = std::env::temp_dir().join("matter_package_prune_registry");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&registry_root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(registry_root.join("utils").join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let utils_manifest = Manifest::new("utils".to_string(), Version::new(1, 2, 0));
        let utils_root = registry_root.join("utils");
        utils_manifest.save(&utils_root.join("matter.toml")).unwrap();
        fs::write(utils_root.join("src").join("main.matter"), "print 1\n").unwrap();

        let mut manager = PackageManager::new();
        manager.register_package(Package::new(utils_manifest, utils_root));
        manager.install_dependencies(&app).unwrap();

        let extra = root.join(".matter").join("packages").join("old");
        fs::create_dir_all(&extra).unwrap();
        fs::write(extra.join("matter.toml"), "[package]\nname = \"old\"\nversion = \"0.1.0\"\n").unwrap();

        let removed = manager.prune_installed_packages(&app).unwrap();
        assert_eq!(removed, vec![extra.clone()]);
        assert!(!extra.exists());
        assert!(root.join(".matter").join("packages").join("utils").exists());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(registry_root);
    }

    #[test]
    fn test_sync_dependencies_installs_prunes_and_verifies() {
        let root = std::env::temp_dir().join("matter_package_sync_test");
        let registry_root = std::env::temp_dir().join("matter_package_sync_registry");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&registry_root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(registry_root.join("utils").join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let mut utils_manifest = Manifest::new("utils".to_string(), Version::new(1, 2, 0));
        utils_manifest.entry = "src/lib.matter".to_string();
        let utils_root = registry_root.join("utils");
        utils_manifest.save(&utils_root.join("matter.toml")).unwrap();
        fs::write(utils_root.join("src").join("lib.matter"), "fn util() { return 1 }\n").unwrap();

        let extra = root.join(".matter").join("packages").join("old");
        fs::create_dir_all(&extra).unwrap();
        fs::write(extra.join("matter.toml"), "[package]\nname = \"old\"\nversion = \"0.1.0\"\n").unwrap();

        let mut manager = PackageManager::new();
        manager.register_package(Package::new(utils_manifest, utils_root));
        let report = manager.sync_dependencies(&app).unwrap();
        let installed = root.join(".matter").join("packages").join("utils");

        assert_eq!(report.installed, vec![installed.clone()]);
        assert_eq!(report.verified, vec![installed.clone()]);
        assert_eq!(report.removed, vec![extra.clone()]);
        assert_eq!(
            report.summary(),
            "lockfile: 1\ninstalled: 1\nremoved: 1\nverified: 1"
        );
        assert!(!extra.exists());
        assert!(installed.join("src").join("lib.matter").exists());

        let removed = manager.prune_installed_packages(&app).unwrap();
        assert!(removed.is_empty());

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(registry_root);
    }

    #[test]
    fn test_package_status_reports_ready_project() {
        let root = std::env::temp_dir().join("matter_package_status_ready_test");
        let registry_root = std::env::temp_dir().join("matter_package_status_ready_registry");
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&registry_root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(registry_root.join("utils").join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let mut utils_manifest = Manifest::new("utils".to_string(), Version::new(1, 2, 0));
        utils_manifest.entry = "src/lib.matter".to_string();
        let utils_root = registry_root.join("utils");
        utils_manifest.save(&utils_root.join("matter.toml")).unwrap();
        fs::write(utils_root.join("src").join("lib.matter"), "fn util() { return 1 }\n").unwrap();

        let mut manager = PackageManager::new();
        manager.register_package(Package::new(utils_manifest, utils_root));
        manager.sync_dependencies(&app).unwrap();
        let status = manager.package_status(&app);

        assert!(status.lockfile_ok);
        assert!(status.installation_ok);
        assert!(status.imports_ok);
        assert!(status.errors.is_empty());
        assert!(status.is_ready());
        assert_eq!(
            status.summary(),
            "lockfile: ok\ninstallation: ok\nimports: ok\nerrors: none"
        );

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(registry_root);
    }

    #[test]
    fn test_package_status_reports_missing_lock_and_installation() {
        let root = std::env::temp_dir().join("matter_package_status_missing_test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();

        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, root.clone());
        app.manifest.save(&root.join("matter.toml")).unwrap();

        let manager = PackageManager::new();
        let status = manager.package_status(&app);

        assert!(!status.lockfile_ok);
        assert!(!status.installation_ok);
        assert!(!status.imports_ok);
        assert!(!status.is_ready());
        assert!(status.summary().contains("lockfile: error"));
        assert!(status.summary().contains("installation: error"));
        assert!(status.summary().contains("imports: error"));
        assert!(status.summary().contains("errors:\n- "));
        assert!(status
            .errors
            .iter()
            .any(|error| error.contains("matter.lock unavailable")));
        assert!(status
            .errors
            .iter()
            .any(|error| error.contains("not installed")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_verify_lockfile_detects_stale_dependencies() {
        let mut app_manifest = Manifest::new("app".to_string(), Version::new(0, 1, 0));
        app_manifest.dependencies.insert(
            "utils".to_string(),
            Dependency::Registry(VersionReq::Caret(Version::new(1, 0, 0))),
        );
        let app = Package::new(app_manifest, PathBuf::from("app"));

        let utils = Package::new(
            Manifest::new("utils".to_string(), Version::new(1, 2, 0)),
            PathBuf::from("registry/utils"),
        );

        let mut manager = PackageManager::new();
        manager.register_package(utils);
        let lockfile = manager.lock_dependencies(&app).unwrap();
        assert!(manager.verify_lockfile(&app, &lockfile).is_ok());

        let stale_lockfile = Lockfile::new(
            &app,
            vec![LockEntry {
                name: "utils".to_string(),
                version: Version::new(1, 0, 0),
                source: "registry/utils".to_string(),
            }],
        );
        let error = manager.verify_lockfile(&app, &stale_lockfile).unwrap_err();
        assert!(error.contains("matter.lock is stale"));
        assert!(error.contains("expected [utils@1.2.0]"));
        assert!(error.contains("got [utils@1.0.0]"));
    }
}
