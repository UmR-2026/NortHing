#!/usr/bin/env python3
"""Hybrid search: FTS5 keyword + MiniLM semantic + metadata filtering."""

import sqlite3, sys, os, re, json, argparse

DB_PATH = os.path.join(os.path.dirname(__file__), "hm.db")
EMBED_PATH = os.path.join(os.path.dirname(__file__), "embeddings.db")

def query_fts(keyword, limit=10, domain=None, min_conf=None, status="active"):
    """FTS5 keyword search with metadata filtering."""
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    
    # Escape FTS5 special characters — comprehensive list
    escaped = keyword
    for ch in ['(', ')', '"', ':', '*', '^', '-', '{', '}', '[', ']']:
        escaped = escaped.replace(ch, ' ')
    escaped = ' '.join(escaped.split())  # normalize whitespace
    if not escaped:
        return []
    
    sql = "SELECT rowid, file, heading, content, level, confidence, source, domain, status FROM memory WHERE memory MATCH ?"
    params = [escaped]
    
    if domain:
        sql += " AND domain = ?"
        params.append(domain)
    if min_conf:
        sql += " AND CAST(confidence AS REAL) >= ?"
        params.append(min_conf)
    if status:
        sql += " AND status = ?"
        params.append(status)
    
    sql += " LIMIT ?"
    params.append(limit)
    
    try:
        cur.execute(sql, params)
    except Exception as e:
        print(f"Query error: {e}")
        conn.close()
        return []
    
    results = []
    for row in cur.fetchall():
        results.append({
            "rowid": row[0],
            "file": row[1],
            "heading": row[2],
            "content": row[3][:300],
            "level": row[4],
            "confidence": row[5],
            "source": row[6],
            "domain": row[7],
            "status": row[8],
        })
    
    conn.close()
    return results

def query_semantic(query_text, top_k=5):
    """Semantic search using embed_server (HTTP)."""
    import urllib.request
    
    EMBED_SERVER = "http://127.0.0.1:9999"
    
    # Get query embedding from server
    try:
        req = urllib.request.Request(
            f"{EMBED_SERVER}/embed",
            data=json.dumps({"text": query_text}).encode(),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            result = json.loads(resp.read())
            query_embedding = result["embedding"]
    except Exception as e:
        print(f"[error] embed_server unavailable: {e}")
        print("  Start server: py embed_server.py")
        return []
    
    # Load all embeddings from db
    conn = sqlite3.connect(EMBED_PATH)
    cur = conn.cursor()
    cur.execute("SELECT rowid, file, heading, embedding FROM embeddings")
    rows = cur.fetchall()
    conn.close()
    
    # Compute cosine similarity
    import numpy as np
    query_vec = np.array(query_embedding)
    
    results = []
    for rowid, file, heading, embedding_json in rows:
        emb = np.array(json.loads(embedding_json))
        similarity = float(np.dot(query_vec, emb) / (np.linalg.norm(query_vec) * np.linalg.norm(emb)))
        results.append({
            "rowid": rowid,
            "file": file,
            "heading": heading,
            "similarity": similarity,
        })
    
    results.sort(key=lambda x: x["similarity"], reverse=True)
    return results[:top_k]

def hybrid_search(query_text, limit=5, mode="hybrid", domain=None, min_conf=None):
    """Combine FTS5 + semantic search."""
    if mode == "keyword":
        return query_fts(query_text, limit=limit, domain=domain, min_conf=min_conf)
    elif mode == "semantic":
        return query_semantic(query_text, top_k=limit)
    else:
        # Hybrid: merge both, deduplicate by rowid
        fts_results = query_fts(query_text, limit=limit*2, domain=domain, min_conf=min_conf)
        sem_results = query_semantic(query_text, top_k=limit*2)
        
        seen = set()
        merged = []
        
        # Add FTS results
        for r in fts_results:
            if r["rowid"] not in seen:
                r["score"] = 1.0  # FTS gets base score
                merged.append(r)
                seen.add(r["rowid"])
        
        # Add semantic results
        for r in sem_results:
            if r["rowid"] not in seen:
                r["score"] = r.get("similarity", 0)
                merged.append(r)
                seen.add(r["rowid"])
        
        # Sort by score
        merged.sort(key=lambda x: x.get("score", 0), reverse=True)
        return merged[:limit]

def show_meta():
    """Show database statistics."""
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    try:
        cur.execute("SELECT key, value FROM meta")
        for row in cur.fetchall():
            print(f"  {row[0]}: {row[1]}")
    except:
        print("  (no meta table)")
    conn.close()
    
    # Embeddings count
    if os.path.exists(EMBED_PATH):
        conn = sqlite3.connect(EMBED_PATH)
        cur = conn.cursor()
        cur.execute("SELECT COUNT(*) FROM embeddings")
        count = cur.fetchone()[0]
        conn.close()
        print(f"  embeddings: {count}")

def main():
    parser = argparse.ArgumentParser(description="Hybrid memory search")
    parser.add_argument("query", nargs="?", help="Search query")
    parser.add_argument("--mode", choices=["keyword", "semantic", "hybrid"], default="hybrid")
    parser.add_argument("--limit", type=int, default=5)
    parser.add_argument("--domain", help="Filter by domain (L/T/U/S/M)")
    parser.add_argument("--min-conf", type=float, help="Minimum confidence")
    parser.add_argument("--status", default="active", help="Filter by status")
    parser.add_argument("--status-only", action="store_true", help="Show DB stats")
    
    args = parser.parse_args()
    
    if args.status_only or not args.query:
        show_meta()
        return
    
    results = hybrid_search(
        args.query,
        limit=args.limit,
        mode=args.mode,
        domain=args.domain,
        min_conf=args.min_conf,
    )
    
    if not results:
        print(f"[query] 0 results for '{args.query}'")
        return
    
    print(f"[query] {len(results)} result(s) for '{args.query}' (mode={args.mode})\n")
    for r in results:
        score = r.get("score", r.get("similarity", ""))
        if isinstance(score, float):
            score = f"{score:.3f}"
        conf = r.get("confidence", "")
        domain = r.get("domain", "")
        print(f"  [{score}] {r['file']} > {r['heading']}")
        if conf:
            print(f"    conf={conf} domain={domain}")
        if r.get("content"):
            print(f"    {r['content'][:150]}...")
        print()

if __name__ == '__main__':
    main()
