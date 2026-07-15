// REFERENCE — copied from tools/plan-compliance-checker/src/report.rs
// Last synced: ec1902e (v3-restructure)
// Mirror only — NOT compiled. Original file lives in src/.
// If you change the source, re-run: node scripts/copy_reference.js

use crate::task::{CheckResult, TaskResult};
use crate::plan::Plan;
use serde::Serialize;

#[derive(Serialize)]
pub struct Report {
    pub plan_path: String,
    pub plan_title: String,
    pub results: Vec<TaskResultJson>,
}

#[derive(Serialize)]
pub struct TaskResultJson {
    pub task_id: String,
    pub status: String,
    pub checks: Vec<CheckResultJson>,
}

#[derive(Serialize)]
pub struct CheckResultJson {
    pub kind: String,
    pub ok: bool,
    pub detail: String,
}

pub fn format_human(plan: &Plan, results: &[TaskResult]) {
    println!("Plan: {} — {}", plan.path.display(), plan.title);
    for r in results {
        let (id, checks, label) = match r {
            TaskResult::Pass { task_id, checks } => (task_id.as_str(), checks, "PASS"),
            TaskResult::Pending { task_id, checks } => (task_id.as_str(), checks, "PENDING"),
            TaskResult::Fail { task_id, checks } => (task_id.as_str(), checks, "FAIL"),
        };
        println!("\n[TASK {}] {}", id, label);
        for c in checks {
            print_check(c);
        }
    }
    let (mut pass, mut pending, mut fail) = (0, 0, 0);
    for r in results {
        match r {
            TaskResult::Pass { .. } => pass += 1,
            TaskResult::Pending { .. } => pending += 1,
            TaskResult::Fail { .. } => fail += 1,
        }
    }
    println!("\nSUMMARY: {} pass / {} pending / {} fail", pass, pending, fail);
}

pub fn format_json(plan: &Plan, results: &[TaskResult]) -> String {
    let report = Report {
        plan_path: plan.path.to_string_lossy().into_owned(),
        plan_title: plan.title.clone(),
        results: results.iter().map(|r| {
            let (task_id, status, checks) = match r {
                TaskResult::Pass { task_id, checks } => (task_id.clone(), "PASS", checks.clone()),
                TaskResult::Pending { task_id, checks } => (task_id.clone(), "PENDING", checks.clone()),
                TaskResult::Fail { task_id, checks } => (task_id.clone(), "FAIL", checks.clone()),
            };
            TaskResultJson {
                task_id,
                status: status.to_string(),
                checks: checks.iter().map(check_to_json).collect(),
            }
        }).collect(),
    };
    serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
}

fn check_to_json(c: &CheckResult) -> CheckResultJson {
    match c {
        CheckResult::FileExists { path, ok } => CheckResultJson {
            kind: "file_exists".to_string(),
            ok: *ok,
            detail: path.clone(),
        },
        CheckResult::FileModified { path, ok, sha } => CheckResultJson {
            kind: "file_modified".to_string(),
            ok: *ok,
            detail: format!("{} (sha={:?})", path, sha),
        },
        CheckResult::CommitPresent { ok, sha } => CheckResultJson {
            kind: "commit_present".to_string(),
            ok: *ok,
            detail: format!("sha={:?}", sha),
        },
        CheckResult::CommitFilesMatch { ok, expected, actual } => CheckResultJson {
            kind: "commit_files_match".to_string(),
            ok: *ok,
            detail: format!("expected={:?} actual={:?}", expected, actual),
        },
        CheckResult::PathConsistency { path, ok, suggestion } => CheckResultJson {
            kind: "path_consistency".to_string(),
            ok: *ok,
            detail: format!("{} (suggestion={:?})", path, suggestion),
        },
    }
}

fn print_check(c: &CheckResult) {
    let (sym, msg) = match c {
        CheckResult::FileExists { path, ok } => (if *ok { "✓" } else { "✗" }, format!("file_exists: {}", path)),
        CheckResult::FileModified { path, ok, sha } => (if *ok { "✓" } else { "✗" }, format!("file_modified: {} (sha={:?})", path, sha)),
        CheckResult::CommitPresent { ok, sha } => (if *ok { "✓" } else { "✗" }, format!("commit_present: sha={:?}", sha)),
        CheckResult::CommitFilesMatch { ok, expected, actual } => (if *ok { "✓" } else { "✗" }, format!("commit_files_match: expected={:?} actual={:?}", expected, actual)),
        CheckResult::PathConsistency { path, ok, suggestion } => (if *ok { "✓" } else { "⚠" }, format!("path_consistency: {} (suggestion={:?})", path, suggestion)),
    };
    println!("  {} {}", sym, msg);
}
