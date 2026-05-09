/// Matter Package Manager
/// Sistema de pacotes, dependências e versionamento

use std::collections::HashMap;
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

    pub fn save(&self, path: &Path) -> Result<(), String> {
        fs::write(path, self.to_toml()).map_err(|e| e.to_string())
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
