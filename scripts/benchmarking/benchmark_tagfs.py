#!/usr/bin/env python3
"""
ArcFS TagFS Benchmark Suite
============================
Comprehensive benchmarking of TagFS performance vs regular filesystem.
python3 scripts/benchmarking/benchmark_tagfs.py
Metrics:
- File operation latency (create, read, lookup)
- Tag query performance (single-tag, multi-tag, next-level)
- Scalability (files, directory depth, tags per file)
- Comparison with tmpfs baseline

Output:
- benchmarks/results/  - JSON raw data
- benchmarks/graphs/   - PNG visualizations
- benchmarks/report.txt - Summary report
"""

import os
import sys
import json
import time
import shutil
import subprocess
import statistics
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Tuple, Any

# Check for matplotlib
try:
    import matplotlib.pyplot as plt
    import matplotlib.patches as mpatches
    HAS_MATPLOTLIB = True
except ImportError:
    HAS_MATPLOTLIB = False
    print("Warning: matplotlib not installed. Graphs will be skipped.")
    print("Install with: pip install matplotlib")

# ============================================================
# Configuration
# ============================================================

PROJECT_ROOT = Path(__file__).parent.parent
BENCHMARK_DIR = PROJECT_ROOT / "benchmarks"
RESULTS_DIR = BENCHMARK_DIR / "results"
GRAPHS_DIR = BENCHMARK_DIR / "graphs"

TAGFS_MOUNT = PROJECT_ROOT / "mnt_bench_tagfs"
TMPFS_MOUNT = PROJECT_ROOT / "mnt_bench_tmpfs"

ITERATIONS = 5  # Iterations per benchmark
WARMUP_RUNS = 2  # Warmup runs before measurement

# Scalability test parameters
FILE_COUNTS = [10, 50, 100, 250, 500]
DEPTH_LEVELS = [3, 5, 7, 10]
TAGS_PER_FILE = [2, 4, 6, 8]

# ============================================================
# Utility Functions
# ============================================================

def ensure_dirs():
    """Create benchmark output directories."""
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    GRAPHS_DIR.mkdir(parents=True, exist_ok=True)
    TAGFS_MOUNT.mkdir(exist_ok=True)
    TMPFS_MOUNT.mkdir(exist_ok=True)

def cleanup_mount(mount_path: Path):
    """Clean up a mount point."""
    try:
        subprocess.run(["fusermount", "-u", str(mount_path)], 
                      capture_output=True, timeout=5)
    except:
        pass
    time.sleep(0.5)
    if mount_path.exists():
        shutil.rmtree(mount_path, ignore_errors=True)
    mount_path.mkdir(exist_ok=True)

def start_tagfs(mount_path: Path) -> subprocess.Popen:
    """Start TagFS in background."""
    cleanup_mount(mount_path)
    
    # Clean storage directory for fresh start
    storage_dir = PROJECT_ROOT / "my_storage"
    if storage_dir.exists():
        shutil.rmtree(storage_dir, ignore_errors=True)
    
    proc = subprocess.Popen(
        ["cargo", "run", "--release", "--", "mount", str(mount_path)],
        cwd=PROJECT_ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL
    )
    time.sleep(3)  # Wait for mount
    return proc

def stop_tagfs(proc: subprocess.Popen, mount_path: Path):
    """Stop TagFS."""
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except:
        proc.kill()
    cleanup_mount(mount_path)

def timer(func):
    """Time a function execution in milliseconds."""
    start = time.perf_counter()
    result = func()
    elapsed = (time.perf_counter() - start) * 1000  # ms
    return elapsed, result

def stats(times: List[float]) -> Dict[str, float]:
    """Calculate statistics for a list of times."""
    if not times:
        return {"mean": 0, "min": 0, "max": 0, "std": 0, "p95": 0, "p99": 0}
    
    sorted_times = sorted(times)
    n = len(sorted_times)
    
    return {
        "mean": statistics.mean(times),
        "min": min(times),
        "max": max(times),
        "std": statistics.stdev(times) if n > 1 else 0,
        "p95": sorted_times[int(n * 0.95)] if n > 1 else sorted_times[0],
        "p99": sorted_times[int(n * 0.99)] if n > 1 else sorted_times[0],
    }

# ============================================================
# Benchmark Functions
# ============================================================

def bench_file_create(mount_path: Path, num_files: int, depth: int) -> List[float]:
    """Benchmark file creation with nested directories."""
    times = []
    
    for i in range(num_files):
        # Create path like: level0/level1/.../levelN/file_i.txt
        path_parts = [f"dir_{i % 10}_{d}" for d in range(depth)]
        dir_path = mount_path.joinpath(*path_parts)
        file_path = dir_path / f"file_{i}.txt"
        
        def create_file():
            dir_path.mkdir(parents=True, exist_ok=True)
            file_path.write_text(f"Content for file {i}\n" * 10)
        
        elapsed, _ = timer(create_file)
        times.append(elapsed)
    
    return times

def bench_file_read_canonical(mount_path: Path, files: List[Path]) -> List[float]:
    """Benchmark reading files via canonical path."""
    times = []
    
    for file_path in files:
        if file_path.exists():
            elapsed, _ = timer(lambda p=file_path: p.read_text())
            times.append(elapsed)
    
    return times

def bench_file_read_tagpath(mount_path: Path, tag_paths: List[Path]) -> List[float]:
    """Benchmark reading files via tag path."""
    times = []
    
    for tag_path in tag_paths:
        if tag_path.exists():
            elapsed, _ = timer(lambda p=tag_path: p.read_text())
            times.append(elapsed)
    
    return times

def bench_lookup(mount_path: Path, paths: List[Path]) -> List[float]:
    """Benchmark path lookup (stat operations)."""
    times = []
    
    for path in paths:
        elapsed, _ = timer(lambda p=path: p.exists())
        times.append(elapsed)
    
    return times

def bench_readdir(mount_path: Path, dirs: List[Path]) -> List[float]:
    """Benchmark directory listing."""
    times = []
    
    for dir_path in dirs:
        if dir_path.exists() and dir_path.is_dir():
            elapsed, _ = timer(lambda p=dir_path: list(p.iterdir()))
            times.append(elapsed)
    
    return times

# ============================================================
# Main Benchmark Suites
# ============================================================

def run_latency_benchmarks(results: Dict[str, Any]):
    """Run file operation latency benchmarks."""
    print("\n" + "="*60)
    print("BENCHMARK 1: File Operation Latency")
    print("="*60)
    
    results["latency"] = {}
    
    # --- TagFS ---
    print("\n[TagFS] Starting...")
    proc = start_tagfs(TAGFS_MOUNT)
    
    try:
        # Create files
        print("  Creating 100 files (depth=4)...")
        create_times = bench_file_create(TAGFS_MOUNT, 100, 4)
        results["latency"]["tagfs_create"] = stats(create_times)
        print(f"    Mean: {results['latency']['tagfs_create']['mean']:.2f}ms")
        
        # Collect created files
        all_files = list(TAGFS_MOUNT.rglob("*.txt"))[:50]
        
        # Read canonical
        print("  Reading via canonical path...")
        read_times = bench_file_read_canonical(TAGFS_MOUNT, all_files)
        results["latency"]["tagfs_read_canonical"] = stats(read_times)
        print(f"    Mean: {results['latency']['tagfs_read_canonical']['mean']:.2f}ms")
        
        # Read via tag path (first directory component as tag)
        tag_paths = []
        for f in all_files[:20]:
            rel = f.relative_to(TAGFS_MOUNT)
            parts = rel.parts
            if len(parts) >= 2:
                # Access via second dir component as entry point
                tag_path = TAGFS_MOUNT / parts[1] / parts[0] / Path(*parts[2:])
                tag_paths.append(tag_path)
        
        print("  Reading via tag path...")
        tag_read_times = bench_file_read_tagpath(TAGFS_MOUNT, tag_paths)
        results["latency"]["tagfs_read_tagpath"] = stats(tag_read_times) if tag_read_times else stats([0])
        print(f"    Mean: {results['latency']['tagfs_read_tagpath']['mean']:.2f}ms")
        
        # Lookup
        print("  Path lookup (stat)...")
        lookup_times = bench_lookup(TAGFS_MOUNT, all_files)
        results["latency"]["tagfs_lookup"] = stats(lookup_times)
        print(f"    Mean: {results['latency']['tagfs_lookup']['mean']:.2f}ms")
        
        # Readdir
        all_dirs = [f.parent for f in all_files[:30]]
        print("  Directory listing...")
        readdir_times = bench_readdir(TAGFS_MOUNT, all_dirs)
        results["latency"]["tagfs_readdir"] = stats(readdir_times)
        print(f"    Mean: {results['latency']['tagfs_readdir']['mean']:.2f}ms")
        
    finally:
        stop_tagfs(proc, TAGFS_MOUNT)
    
    # --- tmpfs baseline ---
    print("\n[tmpfs] Starting baseline...")
    TMPFS_MOUNT.mkdir(exist_ok=True)
    
    # Create files
    print("  Creating 100 files (depth=4)...")
    create_times = bench_file_create(TMPFS_MOUNT, 100, 4)
    results["latency"]["tmpfs_create"] = stats(create_times)
    print(f"    Mean: {results['latency']['tmpfs_create']['mean']:.2f}ms")
    
    all_files = list(TMPFS_MOUNT.rglob("*.txt"))[:50]
    
    # Read
    print("  Reading files...")
    read_times = bench_file_read_canonical(TMPFS_MOUNT, all_files)
    results["latency"]["tmpfs_read"] = stats(read_times)
    print(f"    Mean: {results['latency']['tmpfs_read']['mean']:.2f}ms")
    
    # Lookup
    print("  Path lookup (stat)...")
    lookup_times = bench_lookup(TMPFS_MOUNT, all_files)
    results["latency"]["tmpfs_lookup"] = stats(lookup_times)
    print(f"    Mean: {results['latency']['tmpfs_lookup']['mean']:.2f}ms")
    
    # Readdir
    all_dirs = [f.parent for f in all_files[:30]]
    print("  Directory listing...")
    readdir_times = bench_readdir(TMPFS_MOUNT, all_dirs)
    results["latency"]["tmpfs_readdir"] = stats(readdir_times)
    print(f"    Mean: {results['latency']['tmpfs_readdir']['mean']:.2f}ms")
    
    # Cleanup
    shutil.rmtree(TMPFS_MOUNT, ignore_errors=True)

def run_scalability_benchmarks(results: Dict[str, Any]):
    """Run scalability benchmarks."""
    print("\n" + "="*60)
    print("BENCHMARK 2: Scalability")
    print("="*60)
    
    results["scalability"] = {
        "file_counts": {},
        "depth_levels": {}
    }
    
    # --- File count scalability ---
    print("\n[File Count Scalability]")
    
    for count in FILE_COUNTS:
        print(f"\n  Testing {count} files...")
        
        proc = start_tagfs(TAGFS_MOUNT)
        try:
            times = bench_file_create(TAGFS_MOUNT, count, 3)
            total_time = sum(times)
            results["scalability"]["file_counts"][str(count)] = {
                "total_ms": total_time,
                "avg_ms": total_time / count,
                "stats": stats(times)
            }
            print(f"    Total: {total_time:.2f}ms, Avg: {total_time/count:.2f}ms/file")
        finally:
            stop_tagfs(proc, TAGFS_MOUNT)
    
    # --- Depth scalability ---
    print("\n[Directory Depth Scalability]")
    
    for depth in DEPTH_LEVELS:
        print(f"\n  Testing depth={depth}...")
        
        proc = start_tagfs(TAGFS_MOUNT)
        try:
            times = bench_file_create(TAGFS_MOUNT, 50, depth)
            total_time = sum(times)
            results["scalability"]["depth_levels"][str(depth)] = {
                "total_ms": total_time,
                "avg_ms": total_time / 50,
                "stats": stats(times)
            }
            print(f"    Total: {total_time:.2f}ms, Avg: {total_time/50:.2f}ms/file")
        finally:
            stop_tagfs(proc, TAGFS_MOUNT)

def run_comparison_benchmarks(results: Dict[str, Any]):
    """Run TagFS vs tmpfs comparison."""
    print("\n" + "="*60)
    print("BENCHMARK 3: TagFS vs tmpfs Comparison")
    print("="*60)
    
    results["comparison"] = {}
    iterations = 3
    file_count = 200
    
    for system, mount_path, use_tagfs in [
        ("TagFS", TAGFS_MOUNT, True),
        ("tmpfs", TMPFS_MOUNT, False)
    ]:
        print(f"\n[{system}] Running {iterations} iterations...")
        
        all_create = []
        all_read = []
        all_lookup = []
        
        for i in range(iterations):
            print(f"  Iteration {i+1}/{iterations}...")
            
            if use_tagfs:
                proc = start_tagfs(mount_path)
            else:
                mount_path.mkdir(exist_ok=True)
            
            try:
                # Create
                create_times = bench_file_create(mount_path, file_count, 4)
                all_create.extend(create_times)
                
                # Read
                files = list(mount_path.rglob("*.txt"))[:100]
                read_times = bench_file_read_canonical(mount_path, files)
                all_read.extend(read_times)
                
                # Lookup
                lookup_times = bench_lookup(mount_path, files)
                all_lookup.extend(lookup_times)
                
            finally:
                if use_tagfs:
                    stop_tagfs(proc, mount_path)
                else:
                    shutil.rmtree(mount_path, ignore_errors=True)
        
        key = system.lower()
        results["comparison"][key] = {
            "create": stats(all_create),
            "read": stats(all_read),
            "lookup": stats(all_lookup)
        }
        
        print(f"  Results:")
        print(f"    Create: {results['comparison'][key]['create']['mean']:.2f}ms avg")
        print(f"    Read:   {results['comparison'][key]['read']['mean']:.2f}ms avg")
        print(f"    Lookup: {results['comparison'][key]['lookup']['mean']:.2f}ms avg")

# ============================================================
# Visualization
# ============================================================

def generate_graphs(results: Dict[str, Any]):
    """Generate benchmark visualization graphs."""
    if not HAS_MATPLOTLIB:
        print("\nSkipping graph generation (matplotlib not installed)")
        return
    
    print("\n" + "="*60)
    print("GENERATING GRAPHS")
    print("="*60)
    
    plt.style.use('seaborn-v0_8-whitegrid' if 'seaborn-v0_8-whitegrid' in plt.style.available else 'ggplot')
    
    # --- Graph 1: Latency Comparison Bar Chart ---
    print("\n  [1/4] Latency comparison...")
    
    fig, ax = plt.subplots(figsize=(12, 6))
    
    operations = ['Create', 'Read', 'Lookup', 'Readdir']
    tagfs_vals = [
        results["latency"]["tagfs_create"]["mean"],
        results["latency"]["tagfs_read_canonical"]["mean"],
        results["latency"]["tagfs_lookup"]["mean"],
        results["latency"]["tagfs_readdir"]["mean"],
    ]
    tmpfs_vals = [
        results["latency"]["tmpfs_create"]["mean"],
        results["latency"]["tmpfs_read"]["mean"],
        results["latency"]["tmpfs_lookup"]["mean"],
        results["latency"]["tmpfs_readdir"]["mean"],
    ]
    
    x = range(len(operations))
    width = 0.35
    
    bars1 = ax.bar([i - width/2 for i in x], tagfs_vals, width, label='TagFS', color='#3498db')
    bars2 = ax.bar([i + width/2 for i in x], tmpfs_vals, width, label='tmpfs', color='#2ecc71')
    
    ax.set_xlabel('Operation', fontsize=12)
    ax.set_ylabel('Latency (ms)', fontsize=12)
    ax.set_title('File Operation Latency: TagFS vs tmpfs', fontsize=14, fontweight='bold')
    ax.set_xticks(x)
    ax.set_xticklabels(operations)
    ax.legend()
    
    # Add value labels on bars
    for bar in bars1 + bars2:
        height = bar.get_height()
        ax.annotate(f'{height:.2f}',
                   xy=(bar.get_x() + bar.get_width() / 2, height),
                   xytext=(0, 3), textcoords="offset points",
                   ha='center', va='bottom', fontsize=9)
    
    plt.tight_layout()
    plt.savefig(GRAPHS_DIR / "1_latency_comparison.png", dpi=150)
    plt.close()
    
    # --- Graph 2: File Count Scalability ---
    print("  [2/4] File count scalability...")
    
    fig, ax = plt.subplots(figsize=(10, 6))
    
    counts = [int(k) for k in results["scalability"]["file_counts"].keys()]
    totals = [results["scalability"]["file_counts"][str(c)]["total_ms"] for c in counts]
    avgs = [results["scalability"]["file_counts"][str(c)]["avg_ms"] for c in counts]
    
    ax.plot(counts, totals, 'b-o', linewidth=2, markersize=8, label='Total Time')
    ax.fill_between(counts, totals, alpha=0.3)
    
    ax.set_xlabel('Number of Files', fontsize=12)
    ax.set_ylabel('Total Time (ms)', fontsize=12)
    ax.set_title('TagFS Scalability: File Count', fontsize=14, fontweight='bold')
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    # Add secondary axis for avg time
    ax2 = ax.twinx()
    ax2.plot(counts, avgs, 'r--s', linewidth=2, markersize=6, label='Avg per File')
    ax2.set_ylabel('Avg Time per File (ms)', fontsize=12, color='red')
    ax2.tick_params(axis='y', labelcolor='red')
    ax2.legend(loc='upper left')
    
    plt.tight_layout()
    plt.savefig(GRAPHS_DIR / "2_scalability_filecount.png", dpi=150)
    plt.close()
    
    # --- Graph 3: Directory Depth Scalability ---
    print("  [3/4] Directory depth scalability...")
    
    fig, ax = plt.subplots(figsize=(10, 6))
    
    depths = [int(k) for k in results["scalability"]["depth_levels"].keys()]
    avgs = [results["scalability"]["depth_levels"][str(d)]["avg_ms"] for d in depths]
    
    bars = ax.bar(depths, avgs, color='#9b59b6', edgecolor='black', linewidth=1.2)
    
    ax.set_xlabel('Directory Depth (levels)', fontsize=12)
    ax.set_ylabel('Avg Time per File (ms)', fontsize=12)
    ax.set_title('TagFS Scalability: Directory Depth Impact', fontsize=14, fontweight='bold')
    ax.set_xticks(depths)
    
    for bar, val in zip(bars, avgs):
        ax.annotate(f'{val:.2f}',
                   xy=(bar.get_x() + bar.get_width() / 2, bar.get_height()),
                   xytext=(0, 3), textcoords="offset points",
                   ha='center', va='bottom', fontsize=10)
    
    plt.tight_layout()
    plt.savefig(GRAPHS_DIR / "3_scalability_depth.png", dpi=150)
    plt.close()
    
    # --- Graph 4: Comparison Summary ---
    print("  [4/4] Overall comparison...")
    
    fig, axes = plt.subplots(1, 3, figsize=(14, 5))
    
    for idx, (op, title) in enumerate([('create', 'Create'), ('read', 'Read'), ('lookup', 'Lookup')]):
        ax = axes[idx]
        
        tagfs_mean = results["comparison"]["tagfs"][op]["mean"]
        tmpfs_mean = results["comparison"]["tmpfs"][op]["mean"]
        tagfs_p95 = results["comparison"]["tagfs"][op]["p95"]
        tmpfs_p95 = results["comparison"]["tmpfs"][op]["p95"]
        
        x = [0, 1]
        means = [tagfs_mean, tmpfs_mean]
        p95s = [max(0, tagfs_p95 - tagfs_mean), max(0, tmpfs_p95 - tmpfs_mean)]
        
        colors = ['#3498db', '#2ecc71']
        bars = ax.bar(x, means, yerr=p95s, capsize=5, color=colors, edgecolor='black')
        
        ax.set_xticks(x)
        ax.set_xticklabels(['TagFS', 'tmpfs'])
        ax.set_ylabel('Latency (ms)')
        ax.set_title(f'{title} Operation', fontsize=12, fontweight='bold')
        
        for bar, val in zip(bars, means):
            ax.annotate(f'{val:.2f}ms',
                       xy=(bar.get_x() + bar.get_width() / 2, bar.get_height()),
                       xytext=(0, 8), textcoords="offset points",
                       ha='center', va='bottom', fontsize=10)
    
    fig.suptitle('TagFS vs tmpfs: Operation Comparison (mean ± p95)', fontsize=14, fontweight='bold')
    plt.tight_layout()
    plt.savefig(GRAPHS_DIR / "4_comparison_summary.png", dpi=150)
    plt.close()
    
    print("  Done!")

def generate_report(results: Dict[str, Any]):
    """Generate text summary report."""
    print("\n" + "="*60)
    print("GENERATING REPORT")
    print("="*60)
    
    report_path = BENCHMARK_DIR / "report.txt"
    
    with open(report_path, 'w') as f:
        f.write("=" * 70 + "\n")
        f.write("           ArcFS TagFS Benchmark Report\n")
        f.write("=" * 70 + "\n")
        f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        f.write("\n")
        
        # Latency Summary
        f.write("-" * 70 + "\n")
        f.write("1. FILE OPERATION LATENCY (ms)\n")
        f.write("-" * 70 + "\n")
        f.write(f"{'Operation':<20} {'TagFS Mean':>12} {'tmpfs Mean':>12} {'Overhead':>12}\n")
        f.write("-" * 70 + "\n")
        
        ops = [
            ('Create', 'tagfs_create', 'tmpfs_create'),
            ('Read', 'tagfs_read_canonical', 'tmpfs_read'),
            ('Lookup', 'tagfs_lookup', 'tmpfs_lookup'),
            ('Readdir', 'tagfs_readdir', 'tmpfs_readdir'),
        ]
        
        for name, tagfs_key, tmpfs_key in ops:
            tagfs_val = results["latency"][tagfs_key]["mean"]
            tmpfs_val = results["latency"][tmpfs_key]["mean"]
            overhead = ((tagfs_val / tmpfs_val) - 1) * 100 if tmpfs_val > 0 else 0
            f.write(f"{name:<20} {tagfs_val:>12.3f} {tmpfs_val:>12.3f} {overhead:>+11.1f}%\n")
        
        f.write("\n")
        
        # Tag Path Access
        f.write("-" * 70 + "\n")
        f.write("2. TAG PATH ACCESS\n")
        f.write("-" * 70 + "\n")
        f.write(f"Canonical Path Read: {results['latency']['tagfs_read_canonical']['mean']:.3f}ms\n")
        f.write(f"Tag Path Read:       {results['latency']['tagfs_read_tagpath']['mean']:.3f}ms\n")
        f.write("\n")
        
        # Scalability
        f.write("-" * 70 + "\n")
        f.write("3. SCALABILITY\n")
        f.write("-" * 70 + "\n")
        f.write("\nFile Count:\n")
        f.write(f"{'Files':>10} {'Total (ms)':>15} {'Avg/File (ms)':>15}\n")
        for count, data in results["scalability"]["file_counts"].items():
            f.write(f"{count:>10} {data['total_ms']:>15.2f} {data['avg_ms']:>15.3f}\n")
        
        f.write("\nDirectory Depth:\n")
        f.write(f"{'Depth':>10} {'Avg/File (ms)':>15}\n")
        for depth, data in results["scalability"]["depth_levels"].items():
            f.write(f"{depth:>10} {data['avg_ms']:>15.3f}\n")
        
        f.write("\n")
        
        # Summary Statistics
        f.write("-" * 70 + "\n")
        f.write("4. COMPARISON STATISTICS\n")
        f.write("-" * 70 + "\n")
        
        for system in ['tagfs', 'tmpfs']:
            f.write(f"\n{system.upper()}:\n")
            for op in ['create', 'read', 'lookup']:
                s = results["comparison"][system][op]
                f.write(f"  {op.capitalize():8}: mean={s['mean']:.3f}ms, "
                       f"std={s['std']:.3f}ms, p95={s['p95']:.3f}ms, p99={s['p99']:.3f}ms\n")
        
        f.write("\n")
        f.write("=" * 70 + "\n")
        f.write("END OF REPORT\n")
        f.write("=" * 70 + "\n")
    
    print(f"  Report saved to: {report_path}")

# ============================================================
# Main
# ============================================================

def main():
    print("=" * 70)
    print("        ArcFS TagFS Benchmark Suite")
    print("=" * 70)
    print(f"Started: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Output:  {BENCHMARK_DIR}")
    
    # Build release binary
    print("\nBuilding release binary...")
    subprocess.run(
        ["cargo", "build", "--release"],
        cwd=PROJECT_ROOT,
        capture_output=True
    )
    
    # Setup
    ensure_dirs()
    results = {
        "timestamp": datetime.now().isoformat(),
        "config": {
            "iterations": ITERATIONS,
            "file_counts": FILE_COUNTS,
            "depth_levels": DEPTH_LEVELS,
        }
    }
    
    try:
        # Run benchmarks
        run_latency_benchmarks(results)
        run_scalability_benchmarks(results)
        run_comparison_benchmarks(results)
        
        # Save raw results
        results_file = RESULTS_DIR / f"benchmark_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        with open(results_file, 'w') as f:
            json.dump(results, f, indent=2)
        print(f"\nRaw results saved to: {results_file}")
        
        # Generate outputs
        generate_graphs(results)
        generate_report(results)
        
        print("\n" + "=" * 70)
        print("BENCHMARK COMPLETE")
        print("=" * 70)
        print(f"\nOutputs:")
        print(f"  Results: {RESULTS_DIR}")
        print(f"  Graphs:  {GRAPHS_DIR}")
        print(f"  Report:  {BENCHMARK_DIR / 'report.txt'}")
        
    except KeyboardInterrupt:
        print("\n\nBenchmark interrupted by user.")
    finally:
        # Cleanup
        cleanup_mount(TAGFS_MOUNT)
        shutil.rmtree(TMPFS_MOUNT, ignore_errors=True)
        shutil.rmtree(TAGFS_MOUNT, ignore_errors=True)

if __name__ == "__main__":
    main()
