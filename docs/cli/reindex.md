# engram reindex

Rebuild the search index.

## Usage

```bash
engram reindex
```

## Description

Rebuilds the Tantivy full-text search index from scratch. The index is stored at `.git/engram-index/`. Use this after manual ref changes or if the index becomes corrupted.

The index is normally updated incrementally when creating or importing engrams. A full rebuild is only needed if:

- You fetched engrams manually (use `engram pull` instead to auto-reindex)
- The index file was deleted or corrupted
- You want to ensure the index is fully in sync

## Examples

```bash
engram reindex
```

## See Also

- [search](search.md) -- Uses the search index
- [pull](pull.md) -- Automatically reindexes after fetching
