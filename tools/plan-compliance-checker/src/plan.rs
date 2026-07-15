use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Parser as MdParser, Tag};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plan {
    pub path: PathBuf,
    pub title: String,
    pub tasks: Vec<Task>,
    pub plan_start_sha: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub files: FilesSpec,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesSpec {
    pub create: Vec<PathBuf>,
    pub modify: Vec<ModifyTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyTarget {
    pub path: PathBuf,
    pub range: Option<LineRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub index: usize,
    pub description: String,
    pub expected_outcome: ExpectedOutcome,
    pub verify_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpectedOutcome {
    Pass,
    Fail(String),
    Custom(i32),
}

pub fn parse_plan(input: &str) -> Result<Plan> {
    let mut plan = Plan::default();
    let mut current_task: Option<Task> = None;
    let mut in_task_heading = false;
    let mut in_files_block = false;
    let mut pending_create = false;
    let mut pending_modify = false;
    let mut current_step: Option<Step> = None;
    let mut step_index: usize = 0;
    let mut pending_step_text: bool = false;

    for event in MdParser::new(input) {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) if current_task.is_none() && plan.title.is_empty() => {
                in_task_heading = true;
            }
            Event::Start(Tag::Heading {
                level: HeadingLevel::H3,
                ..
            }) => {
                // Push any pending step before starting new task
                if let (Some(ref mut t), Some(step)) = (current_task.as_mut(), current_step.take()) {
                    t.steps.push(step);
                }
                if let Some(t) = current_task.take() {
                    plan.tasks.push(t);
                }
                current_task = Some(Task::default());
                in_task_heading = true;
                in_files_block = false;
                pending_create = false;
                pending_modify = false;
                current_step = None;
            }
            Event::Start(Tag::Item) => {
                // Reset pending flags when entering a new list item
                pending_create = false;
                pending_modify = false;
                pending_step_text = false;
            }
            Event::Text(text) => {
                let s = text.to_string();
                if in_task_heading {
                    if let Some(ref mut t) = current_task
                        && t.title.is_empty()
                    {
                        let trimmed = s.trim();
                        if let Some(rest) = trimmed.strip_prefix("Task ")
                            && let Some((id, title)) = rest.split_once(':')
                        {
                            t.id = id.trim().to_string();
                            t.title = title.trim().to_string();
                        }
                    } else if plan.title.is_empty() {
                        plan.title = s;
                    }
                    in_task_heading = false;
                } else if let Some(ref mut t) = current_task {
                    if s.trim() == "Files:" || s.starts_with("**Files:**") {
                        in_files_block = true;
                    } else if in_files_block {
                        if s.trim().starts_with("Create:") {
                            pending_create = true;
                            pending_modify = false;
                        } else if s.trim().starts_with("Modify:") {
                            pending_modify = true;
                            pending_create = false;
                        }
                    } else if s.contains("Step ") {
                        // Finish previous step (push to task)
                        if let Some(step) = current_step.take() {
                            t.steps.push(step);
                        }
                        step_index += 1;
                        let desc = s
                            .trim()
                            .trim_start_matches("[ ] ")
                            .trim_start_matches("[x] ")
                            .trim_start_matches("**Step ")
                            .trim_end_matches("**")
                            .trim_end_matches(':')
                            .to_string();
                        current_step = Some(Step {
                            index: step_index,
                            description: desc,
                            expected_outcome: ExpectedOutcome::Pass,
                            verify_command: None,
                        });
                    } else if s.starts_with("Run:") {
                        pending_step_text = true;
                    } else if s.starts_with("Expected:")
                        && let Some(ref mut step) = current_step
                    {
                        step.expected_outcome = parse_expected(&s);
                    }
                }
            }
            Event::Code(code) => {
                if let Some(ref mut t) = current_task {
                    if pending_create {
                        t.files.create.push(PathBuf::from(&*code));
                        pending_create = false;
                    } else if pending_modify {
                        t.files.modify.push(ModifyTarget {
                            path: PathBuf::from(&*code),
                            range: None,
                        });
                        pending_modify = false;
                    } else if pending_step_text {
                        if let Some(ref mut step) = current_step {
                            step.verify_command = Some(code.to_string());
                        }
                        pending_step_text = false;
                    }
                }
            }
            Event::End(pulldown_cmark::TagEnd::Item) => {
                in_files_block = false;
                // Don't push step here - wait until next Step starts or plan ends
            }
            _ => {}
        }
    }
    // Finalize last task and any pending step
    if let (Some(ref mut t), Some(step)) = (current_task.as_mut(), current_step.take()) {
        t.steps.push(step);
    }
    if let Some(t) = current_task {
        plan.tasks.push(t);
    }

    Ok(plan)
}

fn parse_expected(line: &str) -> ExpectedOutcome {
    let body = line.trim_start_matches("Expected:").trim();
    if body == "PASS" || body == "SUCCESS" {
        ExpectedOutcome::Pass
    } else if body.starts_with("FAIL") {
        ExpectedOutcome::Fail(body.to_string())
    } else if let Ok(n) = body.parse::<i32>() {
        ExpectedOutcome::Custom(n)
    } else {
        ExpectedOutcome::Pass
    }
}
