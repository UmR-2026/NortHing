#!/usr/bin/env python3
"""Generate MiniLM embeddings for semantic search."""

import os, sys, sqlite3, json, time

DB_PATH = os.path.join(os.path.dirname(__file__), "hm.db")
EMBED_PATH = os.path.join(os.path.dirname(__file__), "embeddings.db")

MODEL = None

def load_model():
    """Load sentence-transformers MiniLM model (one-time, global cache)."""
    global MODEL
    if MODEL is None:
        print(f"[embed] loading model...", flush=True)
        try:
            from sentence_transformers import SentenceTransformer
        except ImportError:
            print("[error] sentence-transformers not installed.")
            print("  pip install sentence-transformers")
            sys.exit(1)
        t0 = time.time()
        MODEL = SentenceTransformer('all-MiniLM-L6-v2')  # 384 dimensions
        print(f"[embed] model loaded in {time.time()-t0:.1f}s", flush=True)
    return MODEL

def embed_text(model, text):
    """Generate embedding vector for text."""
    embedding = model.encode(text, show_progress_bar=False)
    return embedding.tolist()

def build_embeddings(force=False):
    """Generate embeddings for all nodes in FTS5."""
    model = load_model()
    
    # Load nodes from FTS5
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    cur.execute("SELECT rowid, file, heading, content FROM memory")
    rows = cur.fetchall()
    conn.close()
    
    # Store embeddings
    if os.path.exists(EMBED_PATH) and force:
        os.remove(EMBED_PATH)
    
    embed_conn = sqlite3.connect(EMBED_PATH)
    embed_cur = embed_conn.cursor()
    embed_cur.execute("CREATE TABLE IF NOT EXISTS embeddings (rowid INTEGER PRIMARY KEY, file TEXT, heading TEXT, embedding BLOB, created_at REAL)")
    embed_cur.execute("CREATE INDEX IF NOT EXISTS idx_rowid ON embeddings(rowid)")
    
    new_count = 0
    for rowid, file, heading, content in rows:
        # Check if already embedded
        embed_cur.execute("SELECT rowid FROM embeddings WHERE rowid=?", (rowid,))
        if embed_cur.fetchone() and not force:
            continue
        
        # Combine heading + content for embedding
        text = f"{heading}\n{content}"
        if len(text) > 2000:
            text = text[:2000]
        
        embedding = embed_text(model, text)
        
        # Store as JSON blob
        embed_cur.execute(
            "INSERT OR REPLACE INTO embeddings VALUES (?, ?, ?, ?, ?)",
            (rowid, file, heading, json.dumps(embedding), time.time())
        )
        new_count += 1
        
        if new_count % 50 == 0:
            print(f"  embedded {new_count} nodes...")
    
    embed_conn.commit()
    embed_conn.close()
    
    print(f"[embed] {new_count} new embeddings generated ({384}d)")
    print(f"[embed] DB: {EMBED_PATH}")

def search(query_text, top_k=5):
    """Semantic search using vector similarity."""
    import numpy as np
    
    model = load_model()
    query_embedding = np.array(embed_text(model, query_text))
    
    # Load all embeddings
    conn = sqlite3.connect(EMBED_PATH)
    cur = conn.cursor()
    cur.execute("SELECT rowid, file, heading, embedding FROM embeddings")
    rows = cur.fetchall()
    conn.close()
    
    # Compute cosine similarity
    results = []
    for rowid, file, heading, embedding_json in rows:
        emb = np.array(json.loads(embedding_json))
        similarity = np.dot(query_embedding, emb) / (np.linalg.norm(query_embedding) * np.linalg.norm(emb))
        results.append({
            "rowid": rowid,
            "file": file,
            "heading": heading,
            "similarity": float(similarity),
        })
    
    # Sort by similarity (descending)
    results.sort(key=lambda x: x["similarity"], reverse=True)
    return results[:top_k]

if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == "--search":
        query = " ".join(sys.argv[2:])
        results = search(query)
        for r in results:
            print(f"  [{r['similarity']:.3f}] {r['file']} > {r['heading']}")
    else:
        force = "--force" in sys.argv
        build_embeddings(force=force)
