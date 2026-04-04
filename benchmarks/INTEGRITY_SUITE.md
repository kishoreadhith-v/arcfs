# Integrity-Only Verification Suite

This suite is intentionally separate from performance benchmarking.

## Why this exists

The durable performance class focuses on persistence semantics (`end_fsync` / `fsync`) and throughput/latency behavior.

Inline `fio verify` can fail on mixed/random rewrite workloads (especially with mixed block sizes), even when the filesystem is healthy, because the write pattern can invalidate deterministic verify-header expectations.

This integrity suite uses **fixed block-size write patterns** and **single-job deterministic access** to make `verify=crc32c` checks meaningful and stable.

## Profiles

- `seq_verify`: sequential write + verify (`bs=128k`, `size=2G`)
- `rand4k_verify`: random 4K write + verify (`size=1G`)
- `rand64k_verify`: random 64K write + verify (`size=1G`)
- `fsync4k_verify`: random 4K write + verify with periodic sync (`fsync=16`, `size=32M`)

All profiles use:
- `ioengine=io_uring`
- `direct=1`
- `verify=crc32c`
- `do_verify=1`
- `verify_fatal=1`

## Run

```bash
benchmarks/run_integrity_suite.sh
```

Run a single profile/target (useful while debugging slow paths):

```bash
INTEGRITY_PROFILE=fsync4k_verify INTEGRITY_TARGET=arcfs_mount benchmarks/run_integrity_suite.sh
```

## Validate

```bash
benchmarks/validate_integrity_results.sh
```

## Output Layout

Results are written to:

- `benchmarks/results/integrity/<mount>/<profile>_integrity_<mount>.json`

Example:

- `benchmarks/results/integrity/ext4_mount/rand4k_verify_integrity_ext4_mount.json`
