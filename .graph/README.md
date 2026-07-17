# BitFun Memory Graph (FTS5 + Vector + Wiki-Links)

> 自托管知识图谱：FTS5 关键词搜索 + MiniLM 语义搜索 + LLM 关系发现 + 跨程序备份还原

## 架构

```
memory/*.md (带 frontmatter)
    ↓
build.py ──→ FTS5 索引 (关键词, BM25)
    ↓
embed.py ──→ embeddings.db (MiniLM 向量, 384维)
    ↓
relate.py ──→ wiki-links (LLM 自动关系发现)
    ↓
query.py ──→ 混合搜索 (FTS5 + 向量 + 过滤)
    ↓
backup.py ──→ 备份还原 (tar + checksum)
```

## 文件清单

| 文件 | 用途 | 依赖 |
|---|---|---|
| `build.py` | 构建 FTS5 索引 (解析 frontmatter) | stdlib |
| `embed.py` | 生成 MiniLM 向量 embeddings | sentence-transformers |
| `relate.py` | LLM 分析关系，补充 wiki-links | LLM API |
| `query.py` | 混合搜索 + 过滤 | stdlib + numpy |
| `backup.py` | 备份还原 + checksum | stdlib |
| `hm.db` | FTS5 数据库 (自动生成的) | — |
| `embeddings.db` | 向量数据库 (自动生成的) | — |

## 使用

```bash
# 构建完整索引 (FTS5 + 向量)
py build.py              # FTS5 索引
py embed.py              # 向量 embeddings
py relate.py             # 自动关系发现

# 搜索
py query.py "关键词"                    # 混合搜索
py query.py "关键词" --mode semantic    # 纯语义
py query.py "关键词" --mode keyword     # 纯关键词
py query.py "关键词" --domain T         # 领域过滤
py query.py "关键词" --min-conf 0.8     # 置信度过滤

# 备份还原
py backup.py create                    # 创建备份
py backup.py list                      # 列出备份
py backup.py restore <timestamp>       # 还原备份
py backup.py verify <timestamp>        # 验证备份完整性

# 维护
py relate.py --scan                    # 扫描新关系
py embed.py --incremental              # 增量编码
```

## Frontmatter Schema

```yaml
---
confidence: 1.0          # S=用户说(1.0) / C=综合(0.7-0.9) / I=推断(0.5-0.8)
source: S                # S/I/C
domain: T                # L学习/T技术/U用户/S自我/M社群
status: active           # active/superseded/negated/archived
created: 2026-07-16
supersedes: null
tags: [rust, visibility]
---
```

## 演化链

```
旧条目 (status=superseded)
   ↑ supersedes 指针
新版本 (status=active, supersedes=旧条目id)
```

## 备份设计

```
 backups/
 ├── 2026-07-16_180000.tar.gz
 │   ├── hm.db
 │   ├── embeddings.db
 │   ├── memory/
 │   │   └── *.md
 │   └── manifest.json  (checksum + meta)
 └── ...
```

Updated: 2026-07-16
