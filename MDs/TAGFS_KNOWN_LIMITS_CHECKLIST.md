# TagFS Known Limits & Merge Checklist

## Scope Implemented
- Automatic tag derivation from parent folder ancestry.
- Tag-permutation lookup via virtual `@tags` directories.
- Read/write/create through TagFS paths mapped to live inodes.
- Sidecar metadata isolation (`ino_tags:*`, `tag_index:*`) with no core inode/recipe schema migration.

## Known Limits
- Ambiguous filename resolution inside a tag context returns an error when multiple matching inodes share the same filename and tag set intersection.
- `rmdir` operations inside virtual TagFS directories are intentionally rejected to avoid unsafe/ambiguous virtual-tree deletions.
- Tag updates by external CLI while filesystem is mounted can hit sled lock contention; in-mount operations and automatic tagging avoid this path.
- Virtual tag directories are a projection layer; explorer views may show the same logical file in multiple tag paths by design.

## Integration Safety Notes
- Core metadata namespaces (`ino_meta:*`, `ino_recipe:*`, `dirent:*`) remain unchanged.
- Snapshot and CAS behavior remains intact in current regression coverage.
- TagFS data is isolated to sidecar namespaces and does not rewrite legacy records.

## Pre-Merge Verification (Current Branch)
- `cargo test`
- `tests/architecture_compliance.sh`
- `tests/regression_e2e.sh`
- `quick_test.sh` and `test_phase2.sh` wrappers
- `verify_single_backing.sh`
