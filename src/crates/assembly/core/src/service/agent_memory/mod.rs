mod auto_memory;
mod distiller;
mod dream;
mod facts;
mod instruction_context;
mod judge_memory;
mod memory_db;

#[cfg(test)]
pub(crate) mod test_seam;

pub(crate) use auto_memory::build_query_aware_facts_reminder;
pub(crate) use auto_memory::build_workspace_agent_memory_prompt;
pub(crate) use auto_memory::build_workspace_memory_files_context;
pub(crate) use distiller::distill_facts_with_llm;
pub(crate) use dream::run_dream_sweep;
pub(crate) use facts::{
    distill_facts_from_user_message, migrate_facts_jsonl_once, read_facts, select_facts_for_prompt, Fact,
    FactConfidence, FactProvenance, FactScope, FactType,
};
pub(crate) use instruction_context::build_workspace_instruction_files_context;
pub(crate) use judge_memory::{get_judge_state, set_judge_state};
pub(crate) use memory_db::{default_memory_db_path, FactReview, MemoryDb};

#[cfg(test)]
pub(crate) use test_seam::{unique_test_memory_db_path, with_test_memory_db_path, MemoryDbPathGuard};
