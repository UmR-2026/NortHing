#!/usr/bin/env python3
"""Embedding server — load MiniLM once, serve via HTTP (concurrent)."""

import os, sys, json, time, argparse
from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn
from urllib.parse import urlparse

MODEL_NAME = 'all-MiniLM-L6-v2'
MODEL = None
MAX_TEXT_LENGTH = 8192  # Safety limit

class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
    """Handle requests in separate threads."""
    daemon_threads = True
    
class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed = urlparse(self.path)
        
        if parsed.path == '/health':
            self.send_json({"status": "ok", "model_loaded": MODEL is not None})
        
        elif parsed.path == '/model':
            self.send_json({"model": MODEL_NAME, "dim": 384, "loaded": MODEL is not None})
        
        else:
            self.send_json({"error": "not found"}, status=404)
    
    def do_POST(self):
        parsed = urlparse(self.path)
        content_length = min(int(self.headers.get('Content-Length', 0)), 1_000_000)  # 1MB cap
        body = self.rfile.read(content_length)
        
        try:
            data = json.loads(body) if body else {}
        except json.JSONDecodeError:
            self.send_json({"error": "invalid JSON"}, status=400)
            return
        
        if parsed.path == '/embed':
            text = data.get('text', '')
            if not text:
                self.send_json({"error": "text required"}, status=400)
                return
            if len(text) > MAX_TEXT_LENGTH:
                text = text[:MAX_TEXT_LENGTH]
            t0 = time.time()
            embedding = embed_text(text)
            self.send_json({
                "embedding": embedding,
                "dim": len(embedding),
                "time_ms": round((time.time()-t0)*1000, 1)
            })
        
        elif parsed.path == '/embed_batch':
            texts = data.get('texts', [])
            if not texts or not isinstance(texts, list):
                self.send_json({"error": "texts (array) required"}, status=400)
                return
            texts = [t[:MAX_TEXT_LENGTH] for t in texts[:100]]  # Max 100 items
            t0 = time.time()
            model = load_model()
            embeddings = model.encode(texts, show_progress_bar=False).tolist()
            self.send_json({
                "embeddings": embeddings,
                "count": len(embeddings),
                "dim": len(embeddings[0]) if embeddings else 0,
                "time_ms": round((time.time()-t0)*1000, 1)
            })
        
        elif parsed.path == '/similarity':
            vec1 = data.get('vec1', [])
            vec2 = data.get('vec2', [])
            if not vec1 or not vec2:
                self.send_json({"error": "vec1 and vec2 required"}, status=400)
                return
            sim = compute_similarity(vec1, vec2)
            self.send_json({"similarity": sim})
        
        else:
            self.send_json({"error": "not found"}, status=404)
    
    def send_json(self, obj, status=200):
        response = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', len(response))
        self.end_headers()
        self.wfile.write(response)
    
    def log_message(self, format, *args):
        pass

def load_model():
    """Load sentence-transformers model (one-time)."""
    global MODEL
    if MODEL is None:
        print(f"[server] loading model {MODEL_NAME}...", flush=True)
        from sentence_transformers import SentenceTransformer
        t0 = time.time()
        MODEL = SentenceTransformer(MODEL_NAME)
        print(f"[server] model loaded in {time.time()-t0:.1f}s", flush=True)
    return MODEL

def embed_text(text):
    """Generate embedding for text."""
    model = load_model()
    embedding = model.encode(text, show_progress_bar=False)
    return embedding.tolist()

def compute_similarity(vec1, vec2):
    """Cosine similarity between two vectors."""
    import numpy as np
    a, b = np.array(vec1), np.array(vec2)
    return float(np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b)))

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed = urlparse(self.path)
        
        if parsed.path == '/health':
            self.send_json({"status": "ok", "model_loaded": MODEL is not None})
        
        elif parsed.path == '/model':
            self.send_json({"model": MODEL_NAME, "dim": 384, "loaded": MODEL is not None})
        
        else:
            self.send_json({"error": "not found"}, status=404)
    
    def do_POST(self):
        parsed = urlparse(self.path)
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length)
        
        try:
            data = json.loads(body) if body else {}
        except json.JSONDecodeError:
            self.send_json({"error": "invalid JSON"}, status=400)
            return
        
        if parsed.path == '/embed':
            text = data.get('text', '')
            if not text:
                self.send_json({"error": "text required"}, status=400)
                return
            t0 = time.time()
            embedding = embed_text(text)
            self.send_json({
                "embedding": embedding,
                "dim": len(embedding),
                "time_ms": round((time.time()-t0)*1000, 1)
            })
        
        elif parsed.path == '/similarity':
            vec1 = data.get('vec1', [])
            vec2 = data.get('vec2', [])
            if not vec1 or not vec2:
                self.send_json({"error": "vec1 and vec2 required"}, status=400)
                return
            sim = compute_similarity(vec1, vec2)
            self.send_json({"similarity": sim})
        
        else:
            self.send_json({"error": "not found"}, status=404)
    
    def send_json(self, obj, status=200):
        response = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', len(response))
        self.end_headers()
        self.wfile.write(response)
    
    def log_message(self, format, *args):
        # Suppress default logging
        pass

def main():
    parser = argparse.ArgumentParser(description="Embedding server")
    parser.add_argument("--port", type=int, default=9999, help="Port to bind")
    parser.add_argument("--host", default="127.0.0.1", help="Host to bind")
    args = parser.parse_args()
    
    print(f"[server] starting embed_server on {args.host}:{args.port}", flush=True)
    
    # Pre-load model before serving
    load_model()
    
    server = ThreadedHTTPServer((args.host, args.port), Handler)
    print(f"[server] ready — http://{args.host}:{args.port}", flush=True)
    
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[server] stopping", flush=True)
        server.shutdown()

if __name__ == '__main__':
    main()
