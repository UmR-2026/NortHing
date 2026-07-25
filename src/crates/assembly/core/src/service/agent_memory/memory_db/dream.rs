use crate::util::errors::{NortHingError, NortHingResult};
use rusqlite::params;
use super::MemoryDb;
use crate::service::agent_memory::facts::{Fact, FactConfidence, FactProvenance, FactScope, FactType};

impl MemoryDb {
    pub(crate) fn get_stale_facts(
        &self,
        workspace_key: Option<&str>,
        older_than_ms: u64,
        limit: usize,
    ) -> NortHingResult<Vec<Fact>> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        let mut stmt = if let Some(ws) = workspace_key {
            conn.prepare(
                "SELECT id, text, scope, confidence, session_id, turn_id, created_at, fact_type
                 FROM facts
                 WHERE status = 'active' AND last_mentioned_at < ?1 AND (scope = 'global' OR workspace_key = ?2)
                 ORDER BY last_mentioned_at ASC
                 LIMIT ?3",
            )
            .map_err(|e| NortHingError::service(format!("Failed to prepare get_stale_facts: {}", e)))?
        } else {
            conn.prepare(
                "SELECT id, text, scope, confidence, session_id, turn_id, created_at, fact_type
                 FROM facts
                 WHERE status = 'active' AND last_mentioned_at < ?1 AND scope = 'global'
                 ORDER BY last_mentioned_at ASC
                 LIMIT ?2",
            )
            .map_err(|e| NortHingError::service(format!("Failed to prepare get_stale_facts: {}", e)))?
        };

        let rows: Vec<rusqlite::Result<(String, String, String, String, String, String, i64, String)>> =
            if let Some(ws) = workspace_key {
                stmt.query_map(params![older_than_ms as i64, ws, limit as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                })
                .map_err(|e| NortHingError::service(format!("Failed to query get_stale_facts: {}", e)))?
                .collect()
            } else {
                stmt.query_map(params![older_than_ms as i64, limit as i64], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                })
                .map_err(|e| NortHingError::service(format!("Failed to query get_stale_facts: {}", e)))?
                .collect()
            };

        let mut facts = Vec::new();
        for row in rows {
            let (id, text, scope, confidence, session_id, turn_id, created_at, fact_type) =
                row.map_err(|e| NortHingError::service(format!("Failed to read stale fact row: {}", e)))?;

            let scope_enum = match scope.as_str() {
                "workspace" => FactScope::Workspace,
                "global" => FactScope::Global,
                _ => {
                    return Err(NortHingError::service(format!(
                        "Unknown scope: {}",
                        scope
                    )));
                }
            };

            let confidence_enum = match confidence.as_str() {
                "high" => FactConfidence::High,
                "med" => FactConfidence::Med,
                "low" => FactConfidence::Low,
                _ => {
                    return Err(NortHingError::service(format!(
                        "Unknown confidence: {}",
                        confidence
                    )));
                }
            };

            let fact_type_enum = match fact_type.as_str() {
                "user" => FactType::User,
                "feedback" => FactType::Feedback,
                "project" => FactType::Project,
                "reference" => FactType::Reference,
                _ => {
                    return Err(NortHingError::service(format!(
                        "Unknown fact_type: {}",
                        fact_type
                    )));
                }
            };

            facts.push(Fact {
                schema_version: 1,
                id,
                text,
                provenance: FactProvenance {
                    session_id,
                    turn_id,
                },
                confidence: confidence_enum,
                scope: scope_enum,
                fact_type: fact_type_enum,
                created_at: created_at as u64,
            });
        }

        Ok(facts)
    }
}
