
**Collected Dataset**
- Root: results
- Performance classes:
  - responsive → 20 JSON files
  - durable → 20 JSON files
- Integrity class:
  - integrity → 16 JSON files
- Total machine-readable result files: **56 JSON**

**File Matrix**
- Responsive: 5 profiles × 4 targets = 20  
  Profiles: seq_write, rand_write, realistic_mix, massive_stream, paranoid_db  
- Durable: 5 profiles × 4 targets = 20  
  Same 5 profiles as responsive
- Integrity: 4 profiles × 4 targets = 16  
  Profiles: seq_verify, rand4k_verify, rand64k_verify, fsync4k_verify
- Targets under each class: ext4_mount, bindfs_mount, btrfs_mount, arcfs_mount

**Filename Patterns**
- Performance files: job_class_mount.json  
  Example pattern in ext4_mount
- Integrity files: profile_integrity_mount.json  
  Example pattern in ext4_mount

**Schema Present in Each JSON**
- Top-level run metadata:
  - fio version, timestamp, timestamp_ms, time
  - global options (directory, ioengine, direct, runtime/ramp_time where applicable)
- jobs[0] block:
  - jobname, error, elapsed, job options
  - read, write, trim, sync sections
  - job_runtime, usr_cpu, sys_cpu, ctx, minf/majf
  - iodepth_level / submit / complete distributions
  - latency bucket histograms (latency_ns/us/ms)

**Core Numeric Metrics Available**
- Throughput:
  - write.bw_bytes (bytes/sec), write.bw (KiB/sec)
  - read.bw_bytes, read.bw
- IOPS:
  - write.iops, read.iops
- Latency:
  - write.clat_ns.mean
  - write.clat_ns.percentile: 1, 5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 99.5, 99.9, 99.95, 99.99
  - write.clat_ns.max
- Runtime/CPU:
  - job_runtime (ms), usr_cpu (%), sys_cpu (%), ctx
- Queue behavior:
  - iodepth_level percentages by depth bucket

**Important Class-Specific Meaning**
- Responsive + Durable files are mostly write/read performance runs.
- Durable differs by sync semantics in job options (end_fsync or fsync profile-specific).
- Integrity files include verify settings in global options (verify=crc32c, do_verify=1, verify_fatal=1).
- In integrity files, read metrics can be large because verify pass reads data back (not just write-only).

**Observed Value Ranges (from all collected files)**
- Responsive:
  - write_bw_mb: 130.46 → 9834.71
  - write_iops: 733.39 → 134516.67
  - write_p99_ms: 1.04 → 60.56
  - write_p99.9_ms: 3.39 → 792.72
  - runtime_ms: 45000 → 227668
- Durable:
  - write_bw_mb: 12.62 → 7699.16
  - write_iops: 746.27 → 70664.64
  - write_p99_ms: 2.04 → 51.64
  - write_p99.9_ms: 6.52 → 450.89
  - runtime_ms: 46307 → 182867
- Integrity:
  - write_bw_mb: 0.46 → 840.03
  - write_iops: 117.94 → 40529.38
  - write_p99_ms: 0.0006 → 13.96
  - runtime_ms: 1742 → 69608
  - error: always 0 (all passed)

**Data Caveats You Should Account For in Chart Selection**
- Units differ strongly: BW, IOPS, latency, runtime, CPU cannot share one axis.
- Latency has heavy tails; p99/p99.9 often need log scaling.
- Some runs include both read and write; some are effectively write-only.
- Multi-job aggregated runs can show unusual iodepth percentages (aggregated reporting effect).

If you give me your exact chart spec now (chart type + metric + axis + grouping), I’ll implement it exactly as requested.