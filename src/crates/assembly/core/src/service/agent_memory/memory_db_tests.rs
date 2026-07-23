use super::*;
use std::sync::Mutex;

#[test]
fn open_creates_tables() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");

    let _ = MemoryDb::open(&db_path);

    let conn = Connection::open(&db_path).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' OR type='virtual'")
        .unwrap();

    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(tables.contains(&"facts".to_string()));
    assert!(tables.contains(&"facts_fts".to_string()));
    assert!(tables.contains(&"keyword_weights".to_string()));
    assert!(tables.contains(&"judge_mom".to_string()));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn insert_and_get_fact_round_trip() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let ws_fact = Fact {
        schema_version: 1,
        id: "ws1".to_string(),
        text: "workspace fact".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    let global_fact = Fact {
        schema_version: 1,
        id: "g1".to_string(),
        text: "global fact".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Global,
        created_at: 2000,
    };

    db.insert_fact(&ws_fact, Some("ws_a")).unwrap();
    db.insert_fact(&global_fact, None).unwrap();

    let all = db.get_facts(Some("ws_a")).unwrap();
    assert_eq!(all.len(), 2);

    let global_only = db.get_facts(None).unwrap();
    assert_eq!(global_only.len(), 1);
    assert_eq!(global_only[0].id, "g1");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn insert_duplicate_id_ignored() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let fact = Fact {
        schema_version: 1,
        id: "dup1".to_string(),
        text: "duplicate".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    db.insert_fact(&fact, Some("ws")).unwrap();
    db.insert_fact(&fact, Some("ws")).unwrap();

    let facts = db.get_facts(Some("ws")).unwrap();
    assert_eq!(facts.len(), 1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn fts_search_matches_keyword() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let fact = Fact {
        schema_version: 1,
        id: "f1".to_string(),
        text: "用户喜欢用 pnpm 管理依赖".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    db.insert_fact(&fact, Some("ws")).unwrap();

    let hits = db.search_facts("pnpm", Some("ws"), 10).unwrap();
    assert_eq!(hits.len(), 1);

    let misses = db.search_facts("docker", Some("ws"), 10).unwrap();
    assert!(misses.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn fts_search_chinese_bigram() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let fact = Fact {
        schema_version: 1,
        id: "f1".to_string(),
        text: "以后都用pnpm".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    db.insert_fact(&fact, Some("ws")).unwrap();

    let hits = db.search_facts("以后", Some("ws"), 10).unwrap();
    assert_eq!(hits.len(), 1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn fts_search_two_char_cjk() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let fact = Fact {
        schema_version: 1,
        id: "f1".to_string(),
        text: "用户喜欢用 pnpm 管理依赖".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    db.insert_fact(&fact, Some("ws")).unwrap();

    let hits_like = db.search_facts("喜欢", Some("ws"), 10).unwrap();
    assert_eq!(hits_like.len(), 1);

    let hits_dep = db.search_facts("依赖", Some("ws"), 10).unwrap();
    assert_eq!(hits_dep.len(), 1);

    let hits_mgr = db.search_facts("管理", Some("ws"), 10).unwrap();
    assert_eq!(hits_mgr.len(), 1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn fts_search_respects_workspace_scope() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let ws_a = Fact {
        schema_version: 1,
        id: "wa".to_string(),
        text: "workspace A secret".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    let ws_b = Fact {
        schema_version: 1,
        id: "wb".to_string(),
        text: "workspace B secret".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 2000,
    };

    let global = Fact {
        schema_version: 1,
        id: "g1".to_string(),
        text: "global secret".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Global,
        created_at: 3000,
    };

    db.insert_fact(&ws_a, Some("ws_a")).unwrap();
    db.insert_fact(&ws_b, Some("ws_b")).unwrap();
    db.insert_fact(&global, None).unwrap();

    let a_hits = db.search_facts("secret", Some("ws_a"), 10).unwrap();
    assert_eq!(a_hits.len(), 2);
    assert!(a_hits.iter().any(|s| s.fact.id == "wa"));
    assert!(a_hits.iter().any(|s| s.fact.id == "g1"));

    let b_hits = db.search_facts("secret", Some("ws_b"), 10).unwrap();
    assert_eq!(b_hits.len(), 2);
    assert!(b_hits.iter().any(|s| s.fact.id == "wb"));
    assert!(b_hits.iter().any(|s| s.fact.id == "g1"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn boost_keyword_increases_weight() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    db.boost_keyword("pnpm", &["npm".to_string()], 1000)
        .unwrap();
    db.boost_keyword("pnpm", &["yarn".to_string()], 2000)
        .unwrap();

    assert_eq!(db.get_keyword_weight("pnpm").unwrap(), 2.0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn keyword_weight_affects_scored_fact() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let fact = Fact {
        schema_version: 1,
        id: "f1".to_string(),
        text: "用户喜欢用 pnpm 管理依赖".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    db.insert_fact(&fact, Some("ws")).unwrap();
    db.boost_keyword("pnpm", &[], 1000).unwrap();

    let hits = db.search_facts("pnpm", Some("ws"), 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].score > 0.0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn ranking_fuses_three_factors() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let fact_low = Fact {
        schema_version: 1,
        id: "low".to_string(),
        text: "用户喜欢用 pnpm 管理依赖".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    let fact_high = Fact {
        schema_version: 1,
        id: "high".to_string(),
        text: "用户喜欢用 pnpm 管理依赖".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t2".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 2000,
    };

    db.insert_fact(&fact_low, Some("ws")).unwrap();
    db.insert_fact(&fact_high, Some("ws")).unwrap();

    db.boost_keyword("pnpm", &[], 1000).unwrap();
    db.touch_fact("high", 5000).unwrap();

    let hits = db.search_facts("pnpm", Some("ws"), 10).unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].fact.id, "high");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn segment_for_fts_bigram() {
    assert_eq!(segment_for_fts("以后都用pnpm"), "以后 后都 都用 pnpm");
}

#[test]
fn boost_keyword_respects_cap() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    for _ in 0..12 {
        db.boost_keyword("pnpm", &[], 1000).unwrap();
    }

    let w = db.get_keyword_weight("pnpm").unwrap();
    assert!(w <= 5.0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn decay_weights_respects_floor() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    db.boost_keyword("pnpm", &[], 1000).unwrap();
    db.boost_keyword("pnpm", &[], 2000).unwrap();
    db.boost_keyword("pnpm", &[], 3000).unwrap();

    db.decay_all_weights(0.9, 0.1).unwrap();

    let w = db.get_keyword_weight("pnpm").unwrap();
    assert!(w >= 0.1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn delete_fact_removes_from_fts() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let fact = Fact {
        schema_version: 1,
        id: "del1".to_string(),
        text: "用户喜欢用 pnpm 管理依赖".to_string(),
        provenance: FactProvenance {
            session_id: "s1".to_string(),
            turn_id: "t1".to_string(),
        },
        confidence: FactConfidence::High,
        scope: FactScope::Workspace,
        created_at: 1000,
    };

    db.insert_fact(&fact, Some("ws")).unwrap();
    assert_eq!(db.search_facts("pnpm", Some("ws"), 10).unwrap().len(), 1);

    db.delete_fact("del1").unwrap();
    assert!(db.search_facts("pnpm", Some("ws"), 10).unwrap().is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn empty_query_returns_empty() {
    let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let db_path = temp_dir.join("memory.db");
    let db = MemoryDb::open(&db_path).unwrap();

    let hits = db.search_facts("", Some("ws"), 10).unwrap();
    assert!(hits.is_empty());

    let hits2 = db.search_facts("   ", Some("ws"), 10).unwrap();
    assert!(hits2.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
