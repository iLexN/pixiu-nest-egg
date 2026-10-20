# Tasks

## 1. Delta specs

- [x] 1.1 Write one delta spec per listed capability. Each long requirement becomes a MODIFIED block (same header, every scenario unchanged, description trimmed to one behavior), and each moved behavior becomes its own ADDED requirement with a scenario. Verify: `openspec validate split-long-requirements --strict` passes.
- [x] 1.2 Correct the stale `overview.*` setting-key names and the manual-asset cash wording in overview requirements. Verify: a grep for the old names over the synced specs returns nothing.
- [x] 1.3 Lint the prose: complete sentences, scope-preserving qualifiers repeated, no new duplicate scenario names. Verify: `/tmp/lint_split.py` flags only the two pre-existing duplicate scenario names, and the before/after review in `/tmp/split-review.md` has been read.

## 2. Sync and validate

- [x] 2.1 Sync the deltas into `openspec/specs/`, inserting each ADDED requirement directly after its parent. Verify: `openspec validate --specs --strict` reports 16 passed, and every delta block is present verbatim in its main spec.
