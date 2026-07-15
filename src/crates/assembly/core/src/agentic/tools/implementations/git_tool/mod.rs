mod description;
mod git_branch;
mod git_commit;
mod git_query;
mod git_remote;
mod git_types;

#[cfg(test)]
mod tests;

use crate::util::errors::NortHingResult;

pub struct GitTool;

impl GitTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GitTool {
    fn default() -> Self {
        Self::new()
    }
}
