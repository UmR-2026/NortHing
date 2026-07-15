//! Build/listing methods on [`FileTreeService`].
//!
//! Sibling to [`super::tree_search`] (which owns the search walker) and
//! [`super::tree_progress`] (which owns progress reporting).
//!
//! All entry points resolve to either
//! [`FileSystemResult`](crate::filesystem::FileSystemResult) (returning a
//! structured filesystem error) or `Result<_, String>` (the legacy ASCII
//! error surface used by the public `build_tree` /
//! `get_directory_contents` entry points).

use super::super::error::{FileSystemError, FileSystemResult};
use super::tree_types::{FileTreeNode, FileTreeStatistics};
use super::FileTreeService;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::warn;

impl FileTreeService {
    pub async fn build_tree(&self, root_path: &str) -> Result<Vec<FileTreeNode>, String> {
        self.build_tree_with_remote_hint(root_path, None).await
    }

    pub async fn build_tree_with_remote_hint(
        &self,
        root_path: &str,
        _preferred_remote_connection_id: Option<&str>,
    ) -> Result<Vec<FileTreeNode>, String> {
        let root_path_buf = PathBuf::from(root_path);

        if !root_path_buf.exists() {
            return Err("Directory does not exist".to_string());
        }

        if !root_path_buf.is_dir() {
            return Err("Path is not a directory".to_string());
        }

        let mut visited = HashSet::new();
        self.build_tree_recursive(&root_path_buf, &root_path_buf, &mut visited, 0)
            .await
    }

    pub async fn build_tree_with_stats(
        &self,
        root_path: &str,
    ) -> FileSystemResult<(Vec<FileTreeNode>, FileTreeStatistics)> {
        let root_path_buf = PathBuf::from(root_path);

        if !root_path_buf.exists() {
            return Err(FileSystemError::service("Directory does not exist".to_string()));
        }

        if !root_path_buf.is_dir() {
            return Err(FileSystemError::service("Path is not a directory".to_string()));
        }

        let mut visited = HashSet::new();
        let mut stats = FileTreeStatistics {
            total_files: 0,
            total_directories: 0,
            total_size_bytes: 0,
            max_depth_reached: 0,
            file_type_counts: HashMap::new(),
            large_files: Vec::new(),
            symlinks_count: 0,
            hidden_files_count: 0,
        };

        let nodes = self
            .build_tree_recursive_with_stats(&root_path_buf, &root_path_buf, &mut visited, 0, &mut stats)
            .await
            .map_err(FileSystemError::service)?;

        Ok((nodes, stats))
    }

    fn build_tree_recursive<'a>(
        &'a self,
        path: &'a PathBuf,
        root_path: &'a PathBuf,
        visited: &'a mut HashSet<PathBuf>,
        depth: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<FileTreeNode>, String>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(max_depth) = self.options().max_depth {
                if depth > max_depth {
                    return Ok(vec![]);
                }
            }

            // Prevent cycles
            let canonical_path = match path.canonicalize() {
                Ok(p) => p,
                Err(_) => path.clone(),
            };

            if visited.contains(&canonical_path) {
                return Ok(vec![]);
            }
            visited.insert(canonical_path);

            let mut nodes = Vec::new();

            let mut read_dir = fs::read_dir(path)
                .await
                .map_err(|e| format!("Failed to read directory: {}", e))?;

            let mut entries = Vec::new();
            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| format!("Failed to read directory entry: {}", e))?
            {
                entries.push(entry);
            }

            entries.sort_by(|a, b| {
                let a_is_dir = a.path().is_dir();
                let b_is_dir = b.path().is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in entries {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                if self.should_skip_file(&file_name_str) {
                    continue;
                }

                let entry_path = entry.path();
                let relative_path = entry_path
                    .strip_prefix(root_path)
                    .unwrap_or(&entry_path)
                    .to_string_lossy()
                    .to_string();

                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => match std::fs::symlink_metadata(&entry_path) {
                        Ok(metadata) => metadata.file_type(),
                        Err(e) => {
                            warn!("Failed to get file type, skipping: {} ({})", entry_path.display(), e);
                            continue;
                        }
                    },
                };

                let is_directory = file_type.is_dir();
                let is_symlink = file_type.is_symlink();

                let metadata = entry.metadata().await.ok();
                let size = if is_directory {
                    None
                } else {
                    metadata.as_ref().map(|m| m.len())
                };

                if let (Some(size_bytes), Some(max_mb)) = (size, self.options().max_file_size_mb) {
                    if size_bytes > max_mb * 1024 * 1024 {
                        continue;
                    }
                }

                let last_modified = metadata.and_then(|m| {
                    m.modified().ok().map(|t| {
                        let datetime: chrono::DateTime<chrono::Utc> = t.into();
                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                });

                let extension = if !is_directory {
                    entry_path.extension().map(|ext| ext.to_string_lossy().to_string())
                } else {
                    None
                };

                let mime_type = if self.options().include_mime_types && !is_directory {
                    self.detect_mime_type(&entry_path)
                } else {
                    None
                };

                let permissions = self.get_permissions_string(&entry_path).await;

                let mut node = FileTreeNode::new(
                    relative_path,
                    file_name_str.to_string(),
                    entry_path.to_string_lossy().to_string(),
                    is_directory,
                )
                .with_metadata(size, last_modified)
                .with_extension(extension)
                .with_depth(depth)
                .with_enhanced_info(is_symlink, permissions, mime_type, None);

                if is_directory && (!is_symlink || self.options().follow_symlinks) {
                    match self
                        .build_tree_recursive(&entry_path, root_path, visited, depth + 1)
                        .await
                    {
                        Ok(children) => {
                            node = node.with_children(children);
                        }
                        Err(_) => {
                            node = node.with_children(vec![]);
                        }
                    }
                }

                nodes.push(node);
            }

            Ok(nodes)
        })
    }

    fn build_tree_recursive_with_stats<'a>(
        &'a self,
        path: &'a PathBuf,
        root_path: &'a PathBuf,
        visited: &'a mut HashSet<PathBuf>,
        depth: u32,
        stats: &'a mut FileTreeStatistics,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<FileTreeNode>, String>> + Send + 'a>> {
        Box::pin(async move {
            if depth > stats.max_depth_reached {
                stats.max_depth_reached = depth;
            }

            if let Some(max_depth) = self.options().max_depth {
                if depth > max_depth {
                    return Ok(vec![]);
                }
            }

            // Prevent cycles
            let canonical_path = match path.canonicalize() {
                Ok(p) => p,
                Err(_) => path.clone(),
            };

            if visited.contains(&canonical_path) {
                return Ok(vec![]);
            }
            visited.insert(canonical_path);

            let mut nodes = Vec::new();

            let mut read_dir = fs::read_dir(path)
                .await
                .map_err(|e| format!("Failed to read directory: {}", e))?;

            let mut entries = Vec::new();
            while let Some(entry) = read_dir
                .next_entry()
                .await
                .map_err(|e| format!("Failed to read directory entry: {}", e))?
            {
                entries.push(entry);
            }

            entries.sort_by(|a, b| {
                let a_is_dir = a.path().is_dir();
                let b_is_dir = b.path().is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in entries {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                if file_name_str.starts_with('.') {
                    stats.hidden_files_count += 1;
                }

                if self.should_skip_file(&file_name_str) {
                    continue;
                }

                let entry_path = entry.path();
                let relative_path = entry_path
                    .strip_prefix(root_path)
                    .unwrap_or(&entry_path)
                    .to_string_lossy()
                    .to_string();

                let file_type = match entry.file_type().await {
                    Ok(ft) => ft,
                    Err(_) => match std::fs::symlink_metadata(&entry_path) {
                        Ok(metadata) => metadata.file_type(),
                        Err(e) => {
                            warn!("Failed to get file type, skipping: {} ({})", entry_path.display(), e);
                            continue;
                        }
                    },
                };

                let is_directory = file_type.is_dir();
                let is_symlink = file_type.is_symlink();

                if is_directory {
                    stats.total_directories += 1;
                } else {
                    stats.total_files += 1;
                }

                if is_symlink {
                    stats.symlinks_count += 1;
                }

                let metadata = entry.metadata().await.ok();
                let size = if is_directory {
                    None
                } else {
                    metadata.as_ref().map(|m| m.len())
                };

                if let Some(file_size) = size {
                    stats.total_size_bytes += file_size;

                    if file_size > 10 * 1024 * 1024 {
                        stats
                            .large_files
                            .push((entry_path.to_string_lossy().to_string(), file_size));
                    }
                }

                if let (Some(size_bytes), Some(max_mb)) = (size, self.options().max_file_size_mb) {
                    if size_bytes > max_mb * 1024 * 1024 {
                        continue;
                    }
                }

                if !is_directory {
                    if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                        *stats.file_type_counts.entry(ext.to_string()).or_insert(0) += 1;
                    } else {
                        *stats.file_type_counts.entry("no_extension".to_string()).or_insert(0) += 1;
                    }
                }

                let last_modified = metadata.and_then(|m| {
                    m.modified().ok().map(|t| {
                        let datetime: chrono::DateTime<chrono::Utc> = t.into();
                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                });

                let extension = if !is_directory {
                    entry_path.extension().map(|ext| ext.to_string_lossy().to_string())
                } else {
                    None
                };

                let mime_type = if self.options().include_mime_types && !is_directory {
                    self.detect_mime_type(&entry_path)
                } else {
                    None
                };

                let permissions = self.get_permissions_string(&entry_path).await;

                let mut node = FileTreeNode::new(
                    relative_path,
                    file_name_str.to_string(),
                    entry_path.to_string_lossy().to_string(),
                    is_directory,
                )
                .with_metadata(size, last_modified)
                .with_extension(extension)
                .with_depth(depth)
                .with_enhanced_info(is_symlink, permissions, mime_type, None);

                if is_directory && (!is_symlink || self.options().follow_symlinks) {
                    match self
                        .build_tree_recursive_with_stats(&entry_path, root_path, visited, depth + 1, stats)
                        .await
                    {
                        Ok(children) => {
                            node = node.with_children(children);
                        }
                        Err(_) => {
                            node = node.with_children(vec![]);
                        }
                    }
                }

                nodes.push(node);
            }

            Ok(nodes)
        })
    }

    fn should_skip_file(&self, file_name: &str) -> bool {
        // Skip hidden files and directories (unless explicitly included)
        // But .gitignore and .northhing are always shown
        if !self.options().include_hidden
            && file_name.starts_with('.')
            && file_name != ".gitignore"
            && file_name != ".northhing"
        {
            return true;
        }

        self.options().skip_patterns.iter().any(|pattern| {
            if pattern.contains('*') {
                let parts: Vec<&str> = pattern.split('*').collect();
                if parts.len() == 2 {
                    file_name.starts_with(parts[0]) && file_name.ends_with(parts[1])
                } else {
                    file_name.contains(pattern.trim_matches('*'))
                }
            } else {
                file_name == pattern
            }
        })
    }

    pub async fn get_directory_contents(&self, path: &str) -> Result<Vec<FileTreeNode>, String> {
        self.get_directory_contents_with_remote_hint(path, None).await
    }

    /// Keeps the legacy signature; core handles remote routing before delegating
    /// local directory reads to this owner crate.
    pub async fn get_directory_contents_with_remote_hint(
        &self,
        path: &str,
        _preferred_remote_connection_id: Option<&str>,
    ) -> Result<Vec<FileTreeNode>, String> {
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            return Err("Directory does not exist".to_string());
        }

        if !path_buf.is_dir() {
            return Err("Path is not a directory".to_string());
        }

        let mut nodes = Vec::new();

        let mut read_dir = fs::read_dir(&path_buf)
            .await
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read directory entry: {}", e))?
        {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if self.should_skip_file(&file_name_str) {
                continue;
            }

            let entry_path = entry.path();
            let is_directory = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);

            let node = FileTreeNode::new(
                entry_path.to_string_lossy().to_string(),
                file_name_str.to_string(),
                entry_path.to_string_lossy().to_string(),
                is_directory,
            );

            nodes.push(node);
        }

        Ok(nodes)
    }

    fn detect_mime_type(&self, path: &Path) -> Option<String> {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension.to_lowercase().as_str() {
                "txt" | "md" | "rst" => Some("text/plain".to_string()),
                "html" | "htm" => Some("text/html".to_string()),
                "css" => Some("text/css".to_string()),
                "js" => Some("application/javascript".to_string()),
                "json" => Some("application/json".to_string()),
                "xml" => Some("application/xml".to_string()),
                "yaml" | "yml" => Some("application/yaml".to_string()),

                "rs" => Some("text/rust".to_string()),
                "py" => Some("text/python".to_string()),
                "java" => Some("text/java".to_string()),
                "cpp" | "cc" | "cxx" => Some("text/cpp".to_string()),
                "c" => Some("text/c".to_string()),
                "h" | "hpp" => Some("text/c-header".to_string()),
                "go" => Some("text/go".to_string()),
                "php" => Some("text/php".to_string()),
                "rb" => Some("text/ruby".to_string()),
                "ts" => Some("application/typescript".to_string()),

                "png" => Some("image/png".to_string()),
                "jpg" | "jpeg" => Some("image/jpeg".to_string()),
                "gif" => Some("image/gif".to_string()),
                "svg" => Some("image/svg+xml".to_string()),
                "webp" => Some("image/webp".to_string()),

                "pdf" => Some("application/pdf".to_string()),
                "doc" | "docx" => Some("application/msword".to_string()),
                "xls" | "xlsx" => Some("application/excel".to_string()),
                "ppt" | "pptx" => Some("application/powerpoint".to_string()),

                "zip" => Some("application/zip".to_string()),
                "tar" => Some("application/tar".to_string()),
                "gz" => Some("application/gzip".to_string()),
                "rar" => Some("application/rar".to_string()),

                _ => None,
            }
        } else {
            None
        }
    }

    async fn get_permissions_string(&self, path: &Path) -> Option<String> {
        if let Ok(metadata) = fs::metadata(path).await {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = metadata.permissions();
                let mode = perms.mode();

                let user = format!(
                    "{}{}{}",
                    if mode & 0o400 != 0 { "r" } else { "-" },
                    if mode & 0o200 != 0 { "w" } else { "-" },
                    if mode & 0o100 != 0 { "x" } else { "-" }
                );
                let group = format!(
                    "{}{}{}",
                    if mode & 0o040 != 0 { "r" } else { "-" },
                    if mode & 0o020 != 0 { "w" } else { "-" },
                    if mode & 0o010 != 0 { "x" } else { "-" }
                );
                let other = format!(
                    "{}{}{}",
                    if mode & 0o004 != 0 { "r" } else { "-" },
                    if mode & 0o002 != 0 { "w" } else { "-" },
                    if mode & 0o001 != 0 { "x" } else { "-" }
                );

                Some(format!("{}{}{}", user, group, other))
            }

            #[cfg(windows)]
            {
                let readonly = metadata.permissions().readonly();
                Some(if readonly { "r--" } else { "rw-" }.to_string())
            }
        } else {
            None
        }
    }
}
