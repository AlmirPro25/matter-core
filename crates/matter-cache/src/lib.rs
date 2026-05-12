// Matter Compilation Cache System
// Builds instantâneos via cache inteligente

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Sistema de cache de compilação
///
/// Features:
/// - Hash-based caching (BLAKE3)
/// - LZ4 compression
/// - Incremental compilation
/// - Dependency tracking
/// - Cache invalidation
pub struct CompilationCache {
    /// Diretório de cache
    cache_dir: PathBuf,
    /// Entradas de cache
    entries: HashMap<String, CacheEntry>,
    /// Estatísticas
    stats: CacheStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Hash do source code
    pub source_hash: String,
    /// Hash das dependências
    pub deps_hash: String,
    /// Timestamp da compilação
    pub timestamp: u64,
    /// Caminho do artefato compilado
    pub artifact_path: PathBuf,
    /// Tamanho do artefato (bytes)
    pub artifact_size: u64,
    /// Tempo de compilação (ms)
    pub compile_time_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total de hits
    pub hits: u64,
    /// Total de misses
    pub misses: u64,
    /// Bytes economizados
    pub bytes_saved: u64,
    /// Tempo economizado (ms)
    pub time_saved_ms: u64,
}

impl CompilationCache {
    /// Cria novo cache
    pub fn new(cache_dir: PathBuf) -> Result<Self, String> {
        // Cria diretório se não existir
        fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;

        let mut cache = Self {
            cache_dir,
            entries: HashMap::new(),
            stats: CacheStats::default(),
        };

        // Carrega entradas existentes
        cache.load_entries()?;

        Ok(cache)
    }

    /// Calcula hash do source code
    pub fn hash_source(&self, source: &str) -> String {
        let hash = blake3::hash(source.as_bytes());
        hash.to_hex().to_string()
    }

    /// Calcula hash das dependências
    pub fn hash_dependencies(&self, deps: &[String]) -> String {
        let mut hasher = blake3::Hasher::new();
        for dep in deps {
            hasher.update(dep.as_bytes());
        }
        hasher.finalize().to_hex().to_string()
    }

    /// Verifica se há cache válido
    pub fn get(&mut self, source_hash: &str, deps_hash: &str) -> Option<&CacheEntry> {
        if let Some(entry) = self.entries.get(source_hash) {
            // Verifica se dependências mudaram
            if entry.deps_hash == deps_hash {
                // Verifica se artefato existe
                if entry.artifact_path.exists() {
                    self.stats.hits += 1;
                    self.stats.bytes_saved += entry.artifact_size;
                    self.stats.time_saved_ms += entry.compile_time_ms;
                    return Some(entry);
                }
            }
        }

        self.stats.misses += 1;
        None
    }

    /// Adiciona entrada ao cache
    pub fn put(
        &mut self,
        source_hash: String,
        deps_hash: String,
        artifact_path: PathBuf,
        compile_time_ms: u64,
    ) -> Result<(), String> {
        // Obtém tamanho do artefato
        let artifact_size = fs::metadata(&artifact_path)
            .map_err(|e| format!("Failed to get artifact size: {}", e))?
            .len();

        // Comprime artefato (LZ4)
        let compressed_path = self.compress_artifact(&artifact_path)?;

        let entry = CacheEntry {
            source_hash: source_hash.clone(),
            deps_hash,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            artifact_path: compressed_path,
            artifact_size,
            compile_time_ms,
        };

        self.entries.insert(source_hash, entry);
        self.save_entries()?;

        Ok(())
    }

    /// Comprime artefato com LZ4
    fn compress_artifact(&self, artifact_path: &Path) -> Result<PathBuf, String> {
        let data =
            fs::read(artifact_path).map_err(|e| format!("Failed to read artifact: {}", e))?;

        let compressed = lz4::block::compress(&data, None, false)
            .map_err(|e| format!("Failed to compress: {}", e))?;

        let compressed_path = self.cache_dir.join(format!(
            "{}.lz4",
            artifact_path.file_name().unwrap().to_str().unwrap()
        ));

        fs::write(&compressed_path, compressed)
            .map_err(|e| format!("Failed to write compressed: {}", e))?;

        Ok(compressed_path)
    }

    /// Descomprime artefato
    pub fn decompress_artifact(
        &self,
        entry: &CacheEntry,
        output_path: &Path,
    ) -> Result<(), String> {
        let compressed = fs::read(&entry.artifact_path)
            .map_err(|e| format!("Failed to read compressed: {}", e))?;

        let decompressed = lz4::block::decompress(&compressed, None)
            .map_err(|e| format!("Failed to decompress: {}", e))?;

        fs::write(output_path, decompressed)
            .map_err(|e| format!("Failed to write decompressed: {}", e))?;

        Ok(())
    }

    /// Limpa cache antigo
    pub fn clean_old_entries(&mut self, max_age_days: u64) -> Result<u64, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let max_age_secs = max_age_days * 24 * 60 * 60;
        let mut removed = 0;

        self.entries.retain(|_, entry| {
            let age = now - entry.timestamp;
            if age > max_age_secs {
                // Remove artefato
                let _ = fs::remove_file(&entry.artifact_path);
                removed += 1;
                false
            } else {
                true
            }
        });

        if removed > 0 {
            self.save_entries()?;
        }

        Ok(removed)
    }

    /// Obtém estatísticas
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Taxa de hit
    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.hits + self.stats.misses;
        if total == 0 {
            0.0
        } else {
            self.stats.hits as f64 / total as f64
        }
    }

    /// Carrega entradas do disco
    fn load_entries(&mut self) -> Result<(), String> {
        let index_path = self.cache_dir.join("index.json");

        if index_path.exists() {
            let data = fs::read_to_string(&index_path)
                .map_err(|e| format!("Failed to read index: {}", e))?;

            self.entries =
                serde_json::from_str(&data).map_err(|e| format!("Failed to parse index: {}", e))?;
        }

        Ok(())
    }

    /// Salva entradas no disco
    fn save_entries(&self) -> Result<(), String> {
        let index_path = self.cache_dir.join("index.json");

        let data = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| format!("Failed to serialize index: {}", e))?;

        fs::write(&index_path, data).map_err(|e| format!("Failed to write index: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_creation() {
        let dir = tempdir().unwrap();
        let cache = CompilationCache::new(dir.path().to_path_buf());
        assert!(cache.is_ok());
    }

    #[test]
    fn test_hash_source() {
        let dir = tempdir().unwrap();
        let cache = CompilationCache::new(dir.path().to_path_buf()).unwrap();

        let hash1 = cache.hash_source("let x = 10");
        let hash2 = cache.hash_source("let x = 10");
        let hash3 = cache.hash_source("let x = 20");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_cache_miss() {
        let dir = tempdir().unwrap();
        let mut cache = CompilationCache::new(dir.path().to_path_buf()).unwrap();

        let result = cache.get("nonexistent", "deps");
        assert!(result.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn test_hit_rate() {
        let dir = tempdir().unwrap();
        let mut cache = CompilationCache::new(dir.path().to_path_buf()).unwrap();

        cache.stats.hits = 8;
        cache.stats.misses = 2;

        assert_eq!(cache.hit_rate(), 0.8);
    }
}
