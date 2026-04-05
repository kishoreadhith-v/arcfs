# Worst-Case Performance Suite

This suite is designed to show where ArcFS is expected to perform worse than kernel filesystems.

## Why this exists

ArcFS has strong write-accept and batching behavior, which can make it look very competitive in responsive and amortized durable workloads.

A fair evaluation also needs strict stress cases that amplify user-space overhead and immediate durability penalties.

## Profiles

The suite keeps the same 5 profile names for comparability with other classes, but uses harsher settings.

- `seq_write`: sequential 4K writes with `fsync=1` (`iodepth=1`, `size=512M`)
- `rand_write`: random 4K writes with high contention (`iodepth=64`, `numjobs=8`, `fsync=1`)
- `realistic_mix`: mixed random read/write with small-block bias (`rwmixread=50`, `bssplit=4k/80:64k/20`, `fsync=1`)
- `massive_stream`: sustained incompressible stream with frequent sync pressure (`bs=64k`, `size=4G`, `fsync=1`)
- `paranoid_db`: strict single-thread DB-like durability (`4K randwrite`, `iodepth=1`, `numjobs=1`, `fsync=1`)

## Expected interpretation

This class is intentionally hostile to write-back/coalescing advantages.

Use it to answer:

- How much ArcFS regresses under per-write durability?
- How much tail latency penalty appears vs ext4/btrfs?
- Does ArcFS remain stable (no deadlocks/timeouts) under worst-case pressure?

## Run

This class is included automatically in existing runners:

- `benchmarks/run_benchmarks.sh`
- `benchmarks/run_arcfs_benchmarks.sh`
- `benchmarks/run_bindfs_benchmarks.sh`

## Validate

Use the standard validator:

- `benchmarks/validate_results.sh`

Results are written to:

- `benchmarks/results/worst_case/<mount>/<profile>_worst_case_<mount>.json`
