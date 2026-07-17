#!/usr/bin/env python3
"""LLM-based relationship discovery — auto-generate wiki-links between memory entries."""

import os, sys, sqlite3, json, time, re

DB_PATH = os.path.join(os.path.dirname(__file__), "hm.db")
MEMORY_DIR = os.path.join(os.path.dirname(os.path.dirname(__file__)), "memory")

def load_nodes():
    """Load all memory nodes."""
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    cur.execute("SELECT rowid, file, heading, content, domain FROM memory WHERE status='active'")
    rows = cur.fetchall()
    conn.close()
    return rows

def find_relations_via_llm(nodes, batch_size=10):
    """Use LLM to discover relationships between nodes."""
    # Build a compact representation for LLM
    node_list = []
    for rowid, file, heading, content, domain in nodes:
        # Truncate content for LLM context
        short_content = content[:200] if len(content) > 200 else content
        node_list.append(f"[{rowid}] {file}#{heading} ({domain}): {short_content}")
    
    # Query LLM for relationships
    prompt = f"""Analyze these memory entries and find relationships between them.
Output JSON format: [{{"from": rowid, "to": rowid, "relation": "related_to/supersedes/extends", "reason": "one line"}}]

Only output relationships with strong semantic connection. Max 20 relationships.

Entries:
{chr(10).join(node_list[:50])}
"""
    
    # Use a simple LLM call (placeholder - would use actual LLM API)
    # For now, use heuristic-based matching
    relations = find_relations_heuristic(nodes)
    return relations

def find_relations_heuristic(nodes):
    """Find relationships using heuristics (no LLM cost)."""
    relations = []
    
    # Group by domain
    domain_groups = {}
    for rowid, file, heading, content, domain in nodes:
        if domain not in domain_groups:
            domain_groups[domain] = []
        domain_groups[domain].append((rowid, file, heading, content))
    
    # Within same domain, find keyword overlap
    for domain, group in domain_groups.items():
        for i, (rid1, f1, h1, c1) in enumerate(group):
            for j, (rid2, f2, h2, c2) in enumerate(group):
                if i >= j:
                    continue
                # Simple keyword overlap
                words1 = set(h1.lower().split() + c1.lower().split()[:20])
                words2 = set(h2.lower().split() + c2.lower().split()[:20])
                overlap = words1 & words2
                
                # Filter common words
                stopwords = {"the", "a", "an", "is", "are", "was", "were", "in", "on", "at", "to", "for", "of", "and", "or", "this", "that", "it", "be", "as", "by", "with", "from"}
                meaningful = overlap - stopwords
                
                if len(meaningful) >= 3:
                    relations.append({
                        "from": rid1,
                        "to": rid2,
                        "relation": "related_to",
                        "reason": f"shared: {', '.join(list(meaningful)[:5])}",
                    })
    
    return relations

def apply_relations(relations):
    """Apply discovered relations as wiki-links in memory files."""
    if not relations:
        print("[relate] no new relations found")
        return
    
    print(f"[relate] {len(relations)} relations discovered")
    
    # Group by source file
    file_relations = {}
    for rel in relations:
        from_id = rel["from"]
        # Load file path for this node
        conn = sqlite3.connect(DB_PATH)
        cur = conn.cursor()
        cur.execute("SELECT file, heading FROM memory WHERE rowid=?", (from_id,))
        row = cur.fetchone()
        conn.close()
        
        if row:
            file, heading = row
            if file not in file_relations:
                file_relations[file] = []
            file_relations[file].append(rel)
    
    # Print report
    for file, rels in file_relations.items():
        print(f"\n  {file}:")
        for r in rels:
            print(f"    [{r['from']}] → [{r['to']}] ({r['relation']}): {r['reason']}")

def scan_new_content():
    """Scan for new/unprocessed content."""
    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    cur.execute("SELECT COUNT(*) FROM memory WHERE status='active'")
    count = cur.fetchone()[0]
    conn.close()
    return count

if __name__ == '__main__':
    print("[relate] scanning memory nodes...")
    nodes = load_nodes()
    print(f"[relate] {len(nodes)} active nodes loaded")
    
    relations = find_relations_via_llm(nodes)
    apply_relations(relations)
    
    print("\n[relate] done")
