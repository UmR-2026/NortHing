// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Dioxus consult-room file tree + preview module (W9-6).
//
// The right drawer (`work_app_root` in `windows.rs`) renders four
// sections: routing / planner / diff / **files**. This file owns the
// "files" section: a lazily-expanding directory tree and a text preview
// panel. All IO goes through `super::api`, whose facade implementation is
// gated by the workspace path fence (no `..` / absolute / symlink escapes).

use dioxus::prelude::*;
use northhing_kernel_api::error::KernelError;
use northhing_kernel_api::platform::FileTreeEntryDto;
use std::collections::HashSet;
use std::rc::Rc;

use super::api_fs::{format_size_bytes as fs_format_size_bytes, list_workspace_tree, read_workspace_file};
use super::i18n::{keys, LocalePack};

/// Snapshot of a single preview read. Used as the value of the preview
/// signal so reloads and failures can be distinguished at the render site.
#[derive(Debug, Clone)]
enum PreviewState {
    Idle,
    Loading,
    Loaded(String),
    Error(String),
}

#[derive(Debug, Clone)]
enum TreeState {
    Loading,
    Ready(Vec<FileTreeEntryDto>),
    Error(String),
}

/// Per-directory sub-tree cache. Keys are workspace-relative paths ("" for
/// root); values are the most recent listing result. Mutations go through
/// `&self` clone-of-Rc<RefCell<…>> so subdirectory loaders can mutate
/// without owning the parent's signal.
#[derive(Debug, Clone, Default)]
struct SubtreeCache(Rc<std::cell::RefCell<std::collections::HashMap<String, TreeState>>>);

impl SubtreeCache {
    fn new() -> Self {
        Self::default()
    }
    fn get(&self, dir: &str) -> Option<TreeState> {
        self.0.borrow().get(dir).cloned()
    }
    fn insert(&self, dir: String, state: TreeState) {
        self.0.borrow_mut().insert(dir, state);
    }
}

/// Render the "files" section that lives in the right drawer alongside
/// routing / planner / diff.
pub fn render_files_section(locale: &Rc<LocalePack>, mut folded: Signal<bool>) -> Element {
    // SubtreeCache carries the actual data; the revision cell bumps on
    // every mutation so the Signal's subscription re-renders.
    let cache = use_signal(SubtreeCache::new);
    let expanded = use_signal(HashSet::<String>::new);
    let selected_path = use_signal(String::new);
    let preview = use_signal(|| PreviewState::Idle);

    use_future(move || {
        let cache = cache.peek().clone();
        async move {
            if cache.get("").is_some() {
                return;
            }
            cache.insert("".to_string(), TreeState::Loading);
            match list_workspace_tree("", Some(0)).await {
                Ok(entries) => {
                    cache.insert("".to_string(), TreeState::Ready(entries));
                }
                Err(e) => {
                    cache.insert("".to_string(), TreeState::Error(format_kernel_error(&e)));
                }
            }
        }
    });

    rsx! {
        div {
            class: if folded() { "side-section is-folded" } else { "side-section" },
            div {
                class: "side-title",
                onclick: move |_| { folded.toggle(); },
                "{locale.t(keys::FILES_SECTION_TITLE)} "
                em { "{locale.t(keys::FILES_SECTION_EM)}" }
                span { class: "fold-caret", if folded() { "\u{25B8}" } else { "\u{25BE}" } }
            }
            div { class: "files-tree",
                div {
                    class: "files-list",
                    {render_tree_branch(
                        locale.clone(),
                        String::new(),
                        cache.clone(),
                        expanded.clone(),
                        selected_path.clone(),
                        preview.clone(),
                    )}
                }
            }
            div { class: "files-preview",
                {render_preview(locale.clone(), selected_path, preview)}
            }
        }
    }
}

/// Render one branch (root or a sub-directory) of the tree. Reading the
/// cache's underlying counter via `cache.peek().rev.get()` re-runs the
/// render on every insertion (the counter bumps via `SubtreeCache::insert`).
#[allow(clippy::too_many_arguments)]
fn render_tree_branch(
    locale: Rc<LocalePack>,
    path: String,
    cache: Signal<SubtreeCache>,
    expanded: Signal<HashSet<String>>,
    selected_path: Signal<String>,
    preview: Signal<PreviewState>,
) -> Element {
    let entries = match cache.peek().get(&path) {
        Some(TreeState::Ready(v)) => v,
        Some(TreeState::Error(msg)) => {
            return rsx! {
                div { class: "files-row files-row-error", title: "{msg}",
                    span { class: "files-icon", "!" }
                    "{msg}"
                }
            };
        }
        _ => {
            let label = locale.t(keys::FILES_LOADING).to_string();
            return rsx! {
                div { class: "files-row",
                    span { class: "files-icon", "\u{231B}" }
                    "{label}"
                }
            };
        }
    };

    if entries.is_empty() {
        return rsx! {
            div { class: "files-row files-row-empty",
                span { class: "files-icon", "\u{00B7}" }
                "{locale.t(keys::FILES_EMPTY)}"
            }
        };
    }

    // Pre-sort directories first, then by name case-insensitive.
    let mut sorted: Vec<FileTreeEntryDto> = entries.clone();
    sorted.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    rsx! {
        for entry in sorted.iter() {
            {
                let entry_clone = entry.clone();
                let entry_path = entry.path.clone();
                let entry_name = entry.name.clone();
                let is_dir = entry.is_dir;
                let is_expanded_now = expanded.peek().contains(&entry.path);
                let is_selected_now = selected_path.peek().as_str() == entry.path.as_str();
                let entry_path_for_toggle = entry_path.clone();
                let entry_path_for_select = entry_path.clone();
                let cache_for_child = cache.clone();
                let cache_for_load = cache.clone();
                let expanded_for_child = expanded.clone();
                let selected_for_child = selected_path.clone();
                let preview_for_child = preview.clone();
                let locale_for_child = locale.clone();

                let row_class = if is_selected_now {
                    "files-row files-selected"
                } else if is_dir {
                    "files-row files-dir"
                } else {
                    "files-row files-file"
                };

                rsx! { Fragment {
                    div {
                        class: "{row_class}",
                        onclick: move |_| {
                            if is_dir {
                                let mut set = expanded.peek().clone();
                                if set.contains(&entry_path_for_toggle) {
                                    set.remove(&entry_path_for_toggle);
                                } else {
                                    set.insert(entry_path_for_toggle.clone());
                                    trigger_subdir_load(entry_path_for_toggle.clone(), cache_for_load.clone());
                                }
                                expanded.clone().set(set);
                            } else {
                                selected_path.clone().set(entry_path_for_select.clone());
                                trigger_preview_load(entry_path_for_select.clone(), preview.clone());
                            }
                        },
                        span {
                            class: "files-icon",
                            if is_dir {
                                if is_expanded_now { "\u{25BE}" } else { "\u{25B8}" }
                            } else { "\u{00B7}" }
                        }
                        span { class: "files-name", "{entry_name}" }
                        if !is_dir {
                            if let Some(sz) = entry_clone.size_bytes {
                                span {
                                    class: "files-size",
                                    title: format!("{sz} bytes"),
                                    "{fs_format_size_bytes(sz)}"
                                }
                            }
                        }
                    }
                    if is_dir && is_expanded_now {
                        div { class: "files-children",
                            {render_tree_branch(
                                locale_for_child,
                                entry.path.clone(),
                                cache_for_child,
                                expanded_for_child,
                                selected_for_child,
                                preview_for_child,
                            )}
                        }
                    }
                }}
            }
        }
    }
}

/// Fire-and-forget fetch for a directory's listing. Cache mutations are
/// visible to subsequent renders because they go through the same Rc<RefCell>.
fn trigger_subdir_load(dir: String, cache: Signal<SubtreeCache>) {
    spawn(async move {
        let snap = cache.peek().clone();
        if matches!(snap.get(&dir), Some(TreeState::Ready(_))) {
            return;
        }
        snap.insert(dir.clone(), TreeState::Loading);
        match list_workspace_tree(&dir, Some(1)).await {
            Ok(entries) => {
                cache.peek().insert(dir, TreeState::Ready(entries));
            }
            Err(e) => {
                cache.peek().insert(dir, TreeState::Error(format_kernel_error(&e)));
            }
        }
    });
}

/// Fire-and-forget fetch for a file's preview text.
fn trigger_preview_load(path: String, mut preview: Signal<PreviewState>) {
    preview.set(PreviewState::Loading);
    spawn(async move {
        match read_workspace_file(&path, None).await {
            Ok(text) => {
                preview.set(PreviewState::Loaded(text));
            }
            Err(e) => {
                preview.set(PreviewState::Error(format_kernel_error(&e)));
            }
        }
    });
}

/// Render the preview panel under the tree.
fn render_preview(locale: Rc<LocalePack>, selected_path: Signal<String>, preview: Signal<PreviewState>) -> Element {
    let cur = selected_path.peek().clone();
    let preview_snap = preview.peek().clone();
    let body = match preview_snap {
        PreviewState::Idle => {
            let label = locale.t(keys::FILES_PREVIEW_PLACEHOLDER).to_string();
            rsx! { div { class: "files-preview-placeholder", "{label}" } }
        }
        PreviewState::Loading => {
            let label = locale.t(keys::FILES_PREVIEW_LOADING).to_string();
            rsx! { div { class: "files-preview-loading", "{label}" } }
        }
        PreviewState::Loaded(text) => {
            if text.is_empty() {
                let label = locale.t(keys::FILES_PREVIEW_EMPTY).to_string();
                rsx! { div { class: "files-preview-empty", "{label}" } }
            } else {
                rsx! { pre { class: "files-preview-text", "{text}" } }
            }
        }
        PreviewState::Error(msg) => {
            rsx! { div { class: "files-preview-error", "{msg}" } }
        }
    };

    let title = if cur.is_empty() {
        locale.t(keys::FILES_SECTION_TITLE).to_string()
    } else {
        cur.clone()
    };

    rsx! {
        div { class: "files-preview-shell",
            div { class: "files-preview-title",
                span { class: "files-icon", "\u{00B7}" }
                "{title}"
            }
            {body}
        }
    }
}

/// Map a `KernelError` to a short Chinese label.
fn format_kernel_error(err: &KernelError) -> String {
    match err {
        KernelError::NotFound(_) => "文件不存在".to_string(),
        KernelError::Validation(m) => classify_validation(m),
        KernelError::Runtime(_) | KernelError::Internal(_) => "预览加载失败".to_string(),
        _ => "预览加载失败".to_string(),
    }
}

fn classify_validation(msg: &str) -> String {
    let lower = msg.to_lowercase();
    if lower.contains("binary") {
        "二进制文件不支持预览".to_string()
    } else if lower.contains("too large") {
        "文件过大，无法预览".to_string()
    } else if lower.contains("non-utf8") {
        "非 UTF-8 文本不支持预览".to_string()
    } else if lower.contains("escapes workspace") || lower.contains("absolute") {
        "路径超出工作目录".to_string()
    } else {
        "预览加载失败".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_validation, format_kernel_error};
    use northhing_kernel_api::error::KernelError;

    fn v(s: &str) -> KernelError {
        KernelError::Validation(s.to_string())
    }
    fn nf(s: &str) -> KernelError {
        KernelError::NotFound(s.to_string())
    }
    fn rt(s: &str) -> KernelError {
        KernelError::Runtime(s.to_string())
    }

    #[test]
    fn message_binary() {
        assert_eq!(
            classify_validation("binary file not previewable"),
            "二进制文件不支持预览"
        );
    }
    #[test]
    fn message_too_large() {
        assert_eq!(
            classify_validation("file too large: 1 MiB (cap 256 KiB)"),
            "文件过大，无法预览"
        );
    }
    #[test]
    fn message_escape() {
        assert_eq!(classify_validation("path escapes workspace: ../x"), "路径超出工作目录");
        assert_eq!(
            classify_validation("absolute paths are not allowed: /x"),
            "路径超出工作目录"
        );
    }
    #[test]
    fn message_not_found() {
        assert_eq!(format_kernel_error(&nf("missing")), "文件不存在");
    }
    #[test]
    fn message_other_runtime() {
        assert_eq!(format_kernel_error(&rt("io err")), "预览加载失败");
    }
    #[test]
    fn message_other_validation() {
        assert_eq!(classify_validation("other reason"), "预览加载失败");
    }
}
