#!/usr/bin/env python3
"""Build FTS5 index from memory/*.md — with frontmatter parsing."""

import os, re, sqlite3, json, time, sys

DB_PATH = os.path.join(os.path.dirname(__file__), "hm.db")
MEMORY_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "memory")

def parse_frontmatter(text):
    """Parse YAML frontmatter from markdown."""
    fm = {}
    if text.startswith("---"):
        parts = text.split("---", 2)
        if len(parts) >= 3:
            for line in parts[1].strip().splitlines():
                if ":" in line:
                    key, _, val = line.partition(":")
                    val = val.strip().strip('"').strip("'")
                    if val.startswith("[") and val.endswith("]"):
                        val = [v.strip() for v in val[1:-1].split(",") if v.strip()]
                    fm[key.strip()] = val
    return fm

def chunk_file(path):
    """Split markdown into ## and ### sections as nodes, with frontmatter."""
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    fm = parse_frontmatter(content)
    # Remove frontmatter from content for chunking
    if content.startswith("---"):
        content = content.split("---", 2)[2]
    
    nodes = []
    sections = re.split(r'^## (.+)$', content, flags=re.MULTILINE)
    
    for i in range(1, len(sections), 2):
        heading = sections[i].strip()
        body = sections[i+1].strip() if i+1 < len(sections) else ""
        
        subsections = re.split(r'^### (.+)$', body, flags=re.MULTILINE)
        if len(subsections) > 1:
            for j in range(1, len(subsections), 2):
                sub_heading = subsections[j].strip()
                sub_body = subsections[j+1].strip() if j+1 < len(subsections) else ""
                nodes.append({
                    "file": os.path.relpath(path, MEMORY_DIR),
                    "heading": f"{heading} > {sub_heading}",
                    "content": sub_body,
                    "level": "h3",
                    "frontmatter": fm,
                })
        else:
            nodes.append({
                "file": os.path.relpath(path, MEMORY_DIR),
                "heading": heading,
                "content": body,
                "level": "h2",
                "frontmatter": fm,
            })
    
    return nodes

def build():
    if os.path.exists(DB_PATH):
        os.remove(DB_PATH)
    
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    
    # FTS5 with metadata columns
    cur.execute("""
        CREATE VIRTUAL TABLE memory USING fts5(
            file, heading, content, level,
            confidence, source, domain, status,
            tags, created, superseded_by,
            tokenize='porter unicode61'
        )
    """)
    
    cur.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT, updated_at REAL)")
    
    total_nodes = 0
    all_nodes = []
    
    for fname in sorted(os.listdir(MEMORY_DIR)):
        if fname.endswith('.md') and not fname.startswith('.'):
            fpath = os.path.join(MEMORY_DIR, fname)
            nodes = chunk_file(fpath)
            all_nodes.extend(nodes)
            total_nodes += len(nodes)
    
    for node in all_nodes:
        fm = node.get("frontmatter", {})
        cur.execute(
            "INSERT INTO memory (file, heading, content, level, confidence, source, domain, status, tags, created, superseded_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                node["file"],
                node["heading"],
                node["content"],
                node["level"],
                fm.get("confidence", ""),
                fm.get("source", ""),
                fm.get("domain", ""),
                fm.get("status", "active"),
                json.dumps(fm.get("tags", [])),
                fm.get("created", ""),
                fm.get("supersedes", ""),
            )
        )
    
    cur.execute("INSERT INTO meta VALUES ('node_count', ?, ?)", (str(total_nodes), time.time()))
    cur.execute("INSERT INTO meta VALUES ('file_count', ?, ?)", (str(len([f for f in os.listdir(MEMORY_DIR) if f.endswith('.md')])), time.time()))
    cur.execute("INSERT INTO meta VALUES ('built_at', ?, ?)", (time.strftime('%Y-%m-%d %H:%M'), time.time()))
    
    conn.commit()
    conn.close()
    
    print(f"[build] {total_nodes} nodes from {len(os.listdir(MEMORY_DIR))} files")
    print(f"[build] DB: {DB_PATH}")

if __name__ == '__main__':
    build()
