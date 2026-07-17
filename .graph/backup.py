#!/usr/bin/env python3
"""Backup and restore memory system — cross-program portability."""

import os, sys, tarfile, hashlib, json, time, shutil, argparse

WORKSPACE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BACKUP_DIR = os.path.join(WORKSPACE, "backups")
GRAPH_DIR = os.path.join(WORKSPACE, ".graph")
MEMORY_DIR = os.path.join(WORKSPACE, "memory")

def compute_checksum(path):
    """Compute SHA256 checksum of a file."""
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        while True:
            chunk = f.read(8192)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()

def create_backup():
    """Create a timestamped backup of the memory system."""
    os.makedirs(BACKUP_DIR, exist_ok=True)
    
    timestamp = time.strftime('%Y%m%d_%H%M%S')
    backup_name = f"memory_backup_{timestamp}"
    backup_path = os.path.join(BACKUP_DIR, f"{backup_name}.tar.gz")
    
    manifest = {
        "timestamp": timestamp,
        "created_at": time.strftime('%Y-%m-%d %H:%M:%S'),
        "files": {},
    }
    
    with tarfile.open(backup_path, 'w:gz') as tar:
        # 1. FTS5 database
        db_path = os.path.join(GRAPH_DIR, "hm.db")
        if os.path.exists(db_path):
            tar.add(db_path, arcname="hm.db")
            manifest["files"]["hm.db"] = compute_checksum(db_path)
        
        # 2. Embeddings database
        embed_path = os.path.join(GRAPH_DIR, "embeddings.db")
        if os.path.exists(embed_path):
            tar.add(embed_path, arcname="embeddings.db")
            manifest["files"]["embeddings.db"] = compute_checksum(embed_path)
        
        # 3. Memory markdown files
        for fname in os.listdir(MEMORY_DIR):
            if fname.endswith('.md'):
                fpath = os.path.join(MEMORY_DIR, fname)
                tar.add(fpath, arcname=f"memory/{fname}")
                manifest["files"][f"memory/{fname}"] = compute_checksum(fpath)
        
        # 4. Manifest
        manifest_json = json.dumps(manifest, indent=2)
        import io
        info = tarfile.TarInfo(name="manifest.json")
        info.size = len(manifest_json)
        tar.addfile(info, io.BytesIO(manifest_json.encode()))
    
    # Also save manifest separately for quick inspection
    manifest_path = os.path.join(BACKUP_DIR, f"{backup_name}_manifest.json")
    with open(manifest_path, 'w') as f:
        json.dump(manifest, f, indent=2)
    
    print(f"[backup] created: {backup_path}")
    print(f"[backup] files: {len(manifest['files'])}")
    print(f"[backup] manifest: {manifest_path}")
    return backup_path

def list_backups():
    """List all available backups."""
    if not os.path.exists(BACKUP_DIR):
        print("[backup] no backups found")
        return []
    
    backups = []
    for fname in sorted(os.listdir(BACKUP_DIR)):
        if fname.endswith('.tar.gz'):
            path = os.path.join(BACKUP_DIR, fname)
            size = os.path.getsize(path)
            mtime = os.path.getmtime(path)
            backups.append({
                "file": fname,
                "path": path,
                "size": size,
                "mtime": time.strftime('%Y-%m-%d %H:%M', time.localtime(mtime)),
            })
    
    if not backups:
        print("[backup] no backups found")
        return []
    
    print(f"[backup] {len(backups)} backup(s):\n")
    for b in backups:
        size_kb = b["size"] / 1024
        print(f"  {b['file']} ({size_kb:.0f} KB) - {b['mtime']}")
    
    return backups

def verify_backup(backup_name):
    """Verify backup integrity against checksums."""
    backup_path = os.path.join(BACKUP_DIR, backup_name)
    if not os.path.exists(backup_path):
        print(f"[backup] not found: {backup_name}")
        return False
    
    # Load manifest from backup
    with tarfile.open(backup_path, 'r:gz') as tar:
        try:
            manifest_file = tar.getmember("manifest.json")
            manifest_data = tar.extractfile(manifest_file).read()
            manifest = json.loads(manifest_data)
        except:
            print("[backup] manifest corrupted")
            return False
    
    # Verify each file
    all_ok = True
    with tarfile.open(backup_path, 'r:gz') as tar:
        for fname, expected_checksum in manifest["files"].items():
            try:
                f = tar.extractfile(fname)
                if f is None:
                    print(f"  FAIL {fname} (missing)")
                    all_ok = False
                    continue
                content = f.read()
                actual = hashlib.sha256(content).hexdigest()
                if actual == expected_checksum:
                    print(f"  OK {fname}")
                else:
                    print(f"  FAIL {fname} (checksum mismatch)")
                    all_ok = False
            except Exception as e:
                print(f"  FAIL {fname} ({e})")
                all_ok = False
    
    if all_ok:
        print(f"[backup] verification PASSED")
    else:
        print(f"[backup] verification FAILED")
    
    return all_ok

def restore_backup(backup_name, force=False):
    """Restore from a backup."""
    backup_path = os.path.join(BACKUP_DIR, backup_name)
    if not os.path.exists(backup_path):
        print(f"[backup] not found: {backup_name}")
        return False
    
    # Verify first
    if not force:
        if not verify_backup(backup_name):
            print("[backup] verification failed, aborting (use --force to override)")
            return False
    
    # Extract
    with tarfile.open(backup_path, 'r:gz') as tar:
        # Restore FTS5 db
        if "hm.db" in [m.name for m in tar.getmembers()]:
            tar.extract("hm.db", GRAPH_DIR)
            print(f"  restored hm.db")
        
        # Restore embeddings db
        if "embeddings.db" in [m.name for m in tar.getmembers()]:
            tar.extract("embeddings.db", GRAPH_DIR)
            print(f"  restored embeddings.db")
        
        # Restore memory files
        memory_members = [m for m in tar.getmembers() if m.name.startswith("memory/")]
        for m in memory_members:
            # SECURITY: prevent Zip Slip path traversal
            dest_path = os.path.join(WORKSPACE, m.name)
            dest_real = os.path.realpath(dest_path)
            workspace_real = os.path.realpath(WORKSPACE)
            if not dest_real.startswith(workspace_real + os.sep):
                print(f"  SKIP suspicious path: {m.name}")
                continue
            tar.extract(m, WORKSPACE)
            print(f"  restored {m.name}")
    
    print(f"[backup] restore complete from {backup_name}")
    return True

def main():
    parser = argparse.ArgumentParser(description="Backup/restore memory system")
    parser.add_argument("action", choices=["create", "list", "verify", "restore"])
    parser.add_argument("--name", help="Backup name for verify/restore")
    parser.add_argument("--force", action="store_true", help="Force restore without verification")
    
    args = parser.parse_args()
    
    if args.action == "create":
        create_backup()
    elif args.action == "list":
        list_backups()
    elif args.action == "verify":
        if not args.name:
            print("[backup] --name required")
            sys.exit(1)
        verify_backup(args.name)
    elif args.action == "restore":
        if not args.name:
            print("[backup] --name required")
            sys.exit(1)
        restore_backup(args.name, force=args.force)

if __name__ == '__main__':
    main()
