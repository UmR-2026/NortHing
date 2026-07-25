use crate::util::errors::NortHingResult;
use super::memory_db::MemoryDb;

pub(crate) fn get_judge_state(db: &MemoryDb, key: &str) -> NortHingResult<Option<String>> {
    db.get_judge_mom_value(key)
}

pub(crate) fn set_judge_state(db: &MemoryDb, key: &str, value: &str, at_ms: u64) -> NortHingResult<()> {
    db.set_judge_mom_value(key, value, at_ms)
}
