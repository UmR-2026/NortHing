use crate::util::errors::{NortHingError, NortHingResult};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::facts::{Fact, FactConfidence, FactProvenance, FactScope, FactType};

pub(crate) struct MemoryDb {
    conn: Mutex<Connection>,
}

pub(crate) struct ScoredFact {
    pub fact: Fact,
    pub bm25: f64,
    pub keyword_weight: f64,
    pub recency_boost: f64,
    pub score: f64,
}

pub(crate) struct FactReview {
    pub id: String,
    pub fact_id: String,
    pub reviewer: String,
    pub action: String,
    pub reason: Option<String>,
    pub created_at: u64,
}

impl MemoryDb {
    pub(crate) fn open(db_path: &Path) -> NortHingResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                NortHingError::io(format!("Failed to create memory db parent dir: {}", e))
            })?;
        }

        let conn = Connection::open(db_path).map_err(|e| {
            NortHingError::io(format!("Failed to open memory db: {}", e))
        })?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| {
                NortHingError::io(format!("Failed to set WAL mode: {}", e))
            })?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.create_tables()?;

        Ok(db)
    }

    fn create_tables(&self) -> NortHingResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS facts (
                id TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                text_fts TEXT NOT NULL,
                scope TEXT NOT NULL,
                workspace_key TEXT,
                confidence TEXT NOT NULL,
                session_id TEXT NOT NULL,
                turn_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_mentioned_at INTEGER NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
                text_fts,
                content='facts',
                content_rowid='rowid'
            );
            CREATE TRIGGER IF NOT EXISTS facts_ai AFTER INSERT ON facts BEGIN
                INSERT INTO facts_fts(rowid, text_fts) VALUES (new.rowid, new.text_fts);
            END;
            CREATE TRIGGER IF NOT EXISTS facts_ad AFTER DELETE ON facts BEGIN
                INSERT INTO facts_fts(facts_fts, rowid, text_fts) VALUES ('delete', old.rowid, old.text_fts);
            END;
            CREATE TRIGGER IF NOT EXISTS facts_au AFTER UPDATE ON facts BEGIN
                INSERT INTO facts_fts(facts_fts, rowid, text_fts) VALUES ('delete', old.rowid, old.text_fts);
                INSERT INTO facts_fts(rowid, text_fts) VALUES (new.rowid, new.text_fts);
            END;

            CREATE TABLE IF NOT EXISTS keyword_weights (
                keyword TEXT PRIMARY KEY,
                weight REAL NOT NULL DEFAULT 1.0,
                mention_count INTEGER NOT NULL DEFAULT 1,
                last_boosted_at INTEGER NOT NULL,
                related_keywords TEXT NOT NULL DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS judge_mom (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS fact_reviews (
                id TEXT PRIMARY KEY,
                fact_id TEXT NOT NULL,
                reviewer TEXT NOT NULL,
                action TEXT NOT NULL,
                reason TEXT,
                created_at INTEGER NOT NULL
            );",
        )
        .map_err(|e| {
            NortHingError::service(format!("Failed to create memory db tables: {}", e))
        })?;

        Self::migrate_facts_columns(&conn)?;

        let mut has_text_fts = false;
        let cols = conn.prepare("PRAGMA table_info(facts)").ok();
        if let Some(mut stmt) = cols {
            let rows = stmt.query_map([], |row| row.get::<_, String>(1)).ok();
            if let Some(cols) = rows {
                for col in cols.filter_map(|c| c.ok()) {
                    if col == "text_fts" {
                        has_text_fts = true;
                        break;
                    }
                }
            }
        }

        if !has_text_fts {
            conn.execute("ALTER TABLE facts ADD COLUMN text_fts TEXT NOT NULL DEFAULT ''", [])
                .map_err(|e| {
                    NortHingError::service(format!("Failed to add text_fts column: {}", e))
                })?;
            let rows: Vec<(i64, String)> = {
                let mut sel = conn.prepare("SELECT rowid, text FROM facts")
                    .map_err(|e| NortHingError::service(format!("Failed to prepare backfill select: {}", e)))?;
                let mapped = sel.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                    .map_err(|e| NortHingError::service(format!("Failed to query backfill rows: {}", e)))?;
                mapped.filter_map(|r| r.ok()).collect()
            };
            for (rowid, text) in rows {
                conn.execute("UPDATE facts SET text_fts = ?1 WHERE rowid = ?2", params![segment_for_fts(&text), rowid])
                    .map_err(|e| NortHingError::service(format!("Failed to backfill text_fts: {}", e)))?;
            }
        }

        Ok(())
    }

    fn migrate_facts_columns(conn: &Connection) -> NortHingResult<()> {
        let mut has_status = false;
        let mut has_superseded_by = false;
        let mut has_fact_type = false;
        let cols = conn.prepare("PRAGMA table_info(facts)").ok();
        if let Some(mut stmt) = cols {
            let rows = stmt.query_map([], |row| row.get::<_, String>(1)).ok();
            if let Some(cols) = rows {
                for col in cols.filter_map(|c| c.ok()) {
                    if col == "status" { has_status = true; }
                    if col == "superseded_by" { has_superseded_by = true; }
                    if col == "fact_type" { has_fact_type = true; }
                }
            }
        }

        if !has_status {
            conn.execute("ALTER TABLE facts ADD COLUMN status TEXT NOT NULL DEFAULT 'active'", [])
                .map_err(|e| NortHingError::service(format!("Failed to add status column: {}", e)))?;
        }
        if !has_superseded_by {
            conn.execute("ALTER TABLE facts ADD COLUMN superseded_by TEXT", [])
                .map_err(|e| NortHingError::service(format!("Failed to add superseded_by column: {}", e)))?;
        }
        if !has_fact_type {
            conn.execute("ALTER TABLE facts ADD COLUMN fact_type TEXT NOT NULL DEFAULT 'feedback'", [])
                .map_err(|e| NortHingError::service(format!("Failed to add fact_type column: {}", e)))?;
        }

        Ok(())
    }

    pub(crate) fn insert_fact(&self, fact: &Fact, workspace_key: Option<&str>) -> NortHingResult<()> {
        let scope = match fact.scope {
            FactScope::Workspace => "workspace",
            FactScope::Global => "global",
        };
        let confidence = match fact.confidence {
            FactConfidence::High => "high",
            FactConfidence::Med => "med",
            FactConfidence::Low => "low",
        };
        let fact_type = match fact.fact_type {
            FactType::User => "user",
            FactType::Feedback => "feedback",
            FactType::Project => "project",
            FactType::Reference => "reference",
        };

        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        conn.execute(
            "INSERT OR IGNORE INTO facts (id, text, text_fts, scope, workspace_key, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                fact.id,
                fact.text,
                segment_for_fts(&fact.text),
                scope,
                workspace_key,
                confidence,
                fact.provenance.session_id,
                fact.provenance.turn_id,
                fact.created_at as i64,
                fact.created_at as i64,
                fact_type,
            ],
        )
        .map_err(|e| {
            NortHingError::service(format!("Failed to insert fact: {}", e))
        })?;

        Ok(())
    }

    pub(crate) fn get_facts(&self, workspace_key: Option<&str>) -> NortHingResult<Vec<Fact>> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        let mut stmt = if let Some(ws) = workspace_key {
            conn.prepare(
                "SELECT id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type
                 FROM facts
                 WHERE status = 'active' AND (scope = 'global' OR (scope = 'workspace' AND workspace_key = ?1))
                 ORDER BY created_at ASC",
            )
            .map_err(|e| NortHingError::service(format!("Failed to prepare get_facts: {}", e)))?
        } else {
            conn.prepare(
                "SELECT id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type
                 FROM facts
                 WHERE status = 'active' AND scope = 'global'
                 ORDER BY created_at ASC",
            )
            .map_err(|e| NortHingError::service(format!("Failed to prepare get_facts: {}", e)))?
        };

        let rows: Vec<rusqlite::Result<(String, String, String, String, String, String, i64, i64, String)>> =
            if let Some(ws) = workspace_key {
                stmt.query_map(params![ws], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                })
                .map_err(|e| NortHingError::service(format!("Failed to query get_facts: {}", e)))?
                .collect()
            } else {
                stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                    ))
                })
                .map_err(|e| NortHingError::service(format!("Failed to query get_facts: {}", e)))?
                .collect()
            };

        let mut facts = Vec::new();
        for row in rows {
            let (id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type) =
                row.map_err(|e| NortHingError::service(format!("Failed to read fact row: {}", e)))?;

            let scope_enum = match scope.as_str() {
                "workspace" => FactScope::Workspace,
                "global" => FactScope::Global,
                _ => {
                    return Err(NortHingError::service(format!(
                        "Unknown scope: {}",
                        scope
                    )))
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
                    )))
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
                    )))
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

    pub(crate) fn touch_fact(&self, fact_id: &str, at_ms: u64) -> NortHingResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        conn.execute(
            "UPDATE facts SET last_mentioned_at = ?1 WHERE id = ?2",
            params![at_ms as i64, fact_id],
        )
        .map_err(|e| {
            NortHingError::service(format!("Failed to touch fact: {}", e))
        })?;

        Ok(())
    }

    pub(crate) fn delete_fact(&self, fact_id: &str) -> NortHingResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        conn.execute("DELETE FROM facts WHERE id = ?1", params![fact_id])
            .map_err(|e| {
                NortHingError::service(format!("Failed to delete fact: {}", e))
            })?;

        Ok(())
    }

    pub(crate) fn search_facts(
        &self,
        query: &str,
        workspace_key: Option<&str>,
        limit: usize,
    ) -> NortHingResult<Vec<ScoredFact>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let query_tokens: Vec<String> = segment_for_fts(query).split_whitespace().map(String::from).collect();
        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }

        let match_expr = query_tokens
            .iter()
            .map(|t| format!("\"{}\"", t.replace("\"", "\"\"")))
            .collect::<Vec<_>>()
            .join(" OR ");

        let candidate_limit = (limit * 3).max(30) as i64;

        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        let mut stmt = if workspace_key.is_some() {
            conn.prepare(
                "SELECT f.id, f.text, f.scope, f.confidence, f.session_id, f.turn_id, f.created_at, f.last_mentioned_at, f.fact_type,
                        bm25(facts_fts) AS rank
                 FROM facts_fts
                 JOIN facts f ON f.rowid = facts_fts.rowid
                  WHERE facts_fts MATCH ?1
                    AND f.status = 'active'
                    AND (f.scope = 'global' OR f.workspace_key = ?2)
                  ORDER BY rank
                  LIMIT ?3",
            )
            .map_err(|e| NortHingError::service(format!("Failed to prepare search: {}", e)))?
        } else {
            conn.prepare(
                "SELECT f.id, f.text, f.scope, f.confidence, f.session_id, f.turn_id, f.created_at, f.last_mentioned_at, f.fact_type,
                        bm25(facts_fts) AS rank
                 FROM facts_fts
                 JOIN facts f ON f.rowid = facts_fts.rowid
                  WHERE facts_fts MATCH ?1
                    AND f.status = 'active'
                    AND f.scope = 'global'
                  ORDER BY rank
                  LIMIT ?2",
            )
            .map_err(|e| NortHingError::service(format!("Failed to prepare search: {}", e)))?
        };

        let keyword_map = Self::load_keyword_weights(&conn);

        let rows: Vec<rusqlite::Result<(String, String, String, String, String, String, i64, i64, String, f64)>> =
            if let Some(ws) = workspace_key {
                stmt.query_map(params![match_expr, ws, candidate_limit], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, f64>(9)?,
                    ))
                })
                .map_err(|e| NortHingError::service(format!("Failed to search facts: {}", e)))?
                .collect()
            } else {
                stmt.query_map(params![match_expr, candidate_limit], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, f64>(9)?,
                    ))
                })
                .map_err(|e| NortHingError::service(format!("Failed to search facts: {}", e)))?
                .collect()
            };

        let mut results = Vec::new();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        for row in rows {
            let (id, text, scope, confidence, session_id, turn_id, created_at, last_mentioned_at, fact_type, rank) =
                row.map_err(|e| NortHingError::service(format!("Failed to read search row: {}", e)))?;

            let scope_enum = match scope.as_str() {
                "workspace" => FactScope::Workspace,
                "global" => FactScope::Global,
                _ => {
                    return Err(NortHingError::service(format!(
                        "Unknown scope: {}",
                        scope
                    )))
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
                    )))
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
                    )))
                }
            };

            let fact = Fact {
                schema_version: 1,
                id,
                text: text.clone(),
                provenance: FactProvenance {
                    session_id,
                    turn_id,
                },
                confidence: confidence_enum,
                scope: scope_enum,
                fact_type: fact_type_enum,
                created_at: created_at as u64,
            };

            let fact_tokens: std::collections::HashSet<String> =
                segment_for_fts(&text).split_whitespace().map(String::from).collect();
            let keyword_weight = keyword_map
                .iter()
                .filter(|(kw, _)| {
                    kw.chars().count() >= 2
                        && segment_for_fts(kw).split_whitespace().any(|t| fact_tokens.contains(t))
                })
                .map(|(_, w)| *w)
                .fold(1.0, f64::max);

            let bm25_pos = -rank;
            let days = ((now_ms.saturating_sub(last_mentioned_at as u64)) as f64 / 86_400_000.0).max(1.0);
            let recency_boost = 1.0 + 0.1 * (1.0 / days);
            let score = bm25_pos * keyword_weight * recency_boost;

            results.push(ScoredFact {
                fact,
                bm25: rank,
                keyword_weight,
                recency_boost,
                score,
            });
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    fn load_keyword_weights(conn: &Connection) -> std::collections::HashMap<String, f64> {
        let mut stmt = match conn.prepare("SELECT keyword, weight FROM keyword_weights") {
            Ok(s) => s,
            Err(_) => return std::collections::HashMap::new(),
        };

        let rows = match stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        }) {
            Ok(r) => r,
            Err(_) => return std::collections::HashMap::new(),
        };

        let mut map = std::collections::HashMap::new();
        for row in rows {
            if let Ok((kw, weight)) = row {
                map.insert(kw, weight);
            }
        }
        map
    }

    pub(crate) fn boost_keyword(
        &self,
        keyword: &str,
        related: &[String],
        now_ms: u64,
    ) -> NortHingResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        let existing: Option<(f64, i32, String)> = conn
            .query_row(
                "SELECT weight, mention_count, related_keywords FROM keyword_weights WHERE keyword = ?1",
                params![keyword],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        if let Some((weight, count, related_json)) = existing {
            let mut related_set: std::collections::HashSet<String> =
                serde_json::from_str(&related_json).unwrap_or_default();
            for r in related {
                related_set.insert(r.clone());
            }
            let new_weight = (weight + 1.0).min(5.0);
            let new_count = count + 1;
            let new_related = serde_json::to_string(&related_set).map_err(|e| {
                NortHingError::serialization(e.to_string())
            })?;

            conn.execute(
                "UPDATE keyword_weights SET weight = ?1, mention_count = ?2, last_boosted_at = ?3, related_keywords = ?4 WHERE keyword = ?5",
                params![new_weight, new_count, now_ms as i64, new_related, keyword],
            )
            .map_err(|e| {
                NortHingError::service(format!("Failed to boost keyword: {}", e))
            })?;
        } else {
            let related_json = serde_json::to_string(related).map_err(|e| {
                NortHingError::serialization(e.to_string())
            })?;

            conn.execute(
                "INSERT INTO keyword_weights (keyword, weight, mention_count, last_boosted_at, related_keywords)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![keyword, 1.0, 1, now_ms as i64, related_json],
            )
            .map_err(|e| {
                NortHingError::service(format!("Failed to insert keyword: {}", e))
            })?;
        }

        Ok(())
    }

    pub(crate) fn get_keyword_weight(&self, keyword: &str) -> NortHingResult<f64> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        let weight: Option<f64> = conn
            .query_row(
                "SELECT weight FROM keyword_weights WHERE keyword = ?1",
                params![keyword],
                |row| row.get(0),
            )
            .ok();

        Ok(weight.unwrap_or(1.0))
    }

    pub(crate) fn decay_all_weights(&self, factor: f64, floor: f64) -> NortHingResult<usize> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        let affected = conn
            .execute(
                "UPDATE keyword_weights SET weight = MAX(weight * ?1, ?2)",
                params![factor, floor],
            )
            .map_err(|e| {
                NortHingError::service(format!("Failed to decay weights: {}", e))
            })?;

        Ok(affected)
    }

    pub(crate) fn set_keyword_ignored(&self, keyword: &str) -> NortHingResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        conn.execute(
            "UPDATE keyword_weights SET weight = 0.0 WHERE keyword = ?1",
            params![keyword],
        )
        .map_err(|e| {
            NortHingError::service(format!("Failed to ignore keyword: {}", e))
        })?;

        Ok(())
    }

    pub(crate) fn record_fact_review(&self, review: &FactReview) -> NortHingResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        conn.execute(
            "INSERT INTO fact_reviews (id, fact_id, reviewer, action, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                review.id,
                review.fact_id,
                review.reviewer,
                review.action,
                review.reason,
                review.created_at as i64,
            ],
        )
        .map_err(|e| {
            NortHingError::service(format!("Failed to record fact review: {}", e))
        })?;

        Ok(())
    }

    pub(crate) fn reviews_for_fact(&self, fact_id: &str) -> NortHingResult<Vec<FactReview>> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, fact_id, reviewer, action, reason, created_at
             FROM fact_reviews
             WHERE fact_id = ?1
             ORDER BY created_at ASC",
        )
        .map_err(|e| NortHingError::service(format!("Failed to prepare reviews_for_fact: {}", e)))?;

        let rows = stmt
            .query_map(params![fact_id], |row| {
                Ok(FactReview {
                    id: row.get::<_, String>(0)?,
                    fact_id: row.get::<_, String>(1)?,
                    reviewer: row.get::<_, String>(2)?,
                    action: row.get::<_, String>(3)?,
                    reason: row.get::<_, Option<String>>(4)?,
                    created_at: row.get::<_, i64>(5)? as u64,
                })
            })
            .map_err(|e| NortHingError::service(format!("Failed to query reviews_for_fact: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    pub(crate) fn supersede_fact(&self, fact_id: &str, superseded_by: Option<&str>, at_ms: u64) -> NortHingResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        conn.execute(
            "UPDATE facts SET status = 'superseded', superseded_by = ?2 WHERE id = ?1",
            params![fact_id, superseded_by],
        )
        .map_err(|e| {
            NortHingError::service(format!("Failed to supersede fact: {}", e))
        })?;

        Ok(())
    }

    pub(crate) fn get_judge_mom_value(&self, key: &str) -> NortHingResult<Option<String>> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM judge_mom WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok();

        Ok(value)
    }

    pub(crate) fn set_judge_mom_value(&self, key: &str, value: &str, at_ms: u64) -> NortHingResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            NortHingError::service(format!("MemoryDb lock poisoned: {}", e))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO judge_mom (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, at_ms as i64],
        )
        .map_err(|e| {
            NortHingError::service(format!("Failed to set judge_mom value: {}", e))
        })?;

        Ok(())
    }
}

pub(crate) fn default_memory_db_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("northhing")
        .join("memory")
        .join("memory.db")
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'   | // CJK Unified Ideographs
        '\u{3400}'..='\u{4DBF}'   | // CJK Unified Ideographs Extension A
        '\u{F900}'..='\u{FAFF}'     // CJK Compatibility Ideographs
    )
}

fn segment_for_fts(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut ascii = String::new();
    let mut cjk: Vec<char> = Vec::new();
    fn flush_cjk(cjk: &mut Vec<char>, out: &mut Vec<String>) {
        if cjk.len() == 1 {
            out.push(cjk[0].to_string());
        } else {
            for w in cjk.windows(2) {
                out.push(w.iter().collect::<String>());
            }
        }
        cjk.clear();
    }
    for c in text.chars() {
        if is_cjk(c) {
            if !ascii.is_empty() { out.push(std::mem::take(&mut ascii)); }
            cjk.push(c);
        } else if c.is_whitespace() {
            if !ascii.is_empty() { out.push(std::mem::take(&mut ascii)); }
            flush_cjk(&mut cjk, &mut out);
        } else {
            flush_cjk(&mut cjk, &mut out);
            ascii.push(c);
        }
    }
    if !ascii.is_empty() { out.push(ascii); }
    flush_cjk(&mut cjk, &mut out);
    out.join(" ")
}

mod dream;

#[cfg(test)]
#[path = "memory_db_tests.rs"]
mod tests;
