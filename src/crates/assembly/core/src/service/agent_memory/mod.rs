mod auto_memory;
mod facts;
mod instruction_context;
mod memory_db;

pub(crate) use auto_memory::build_query_aware_facts_reminder;
pub(crate) use auto_memory::build_workspace_agent_memory_prompt;
pub(crate) use auto_memory::build_workspace_memory_files_context;
pub(crate) use facts::{append_facts, append_facts_dedup, distill_facts_from_user_message, read_facts, select_facts_for_prompt, Fact};
pub(crate) use instruction_context::build_workspace_instruction_files_context;
pub(crate) use memory_db::{default_memory_db_path, MemoryDb};
