"""
ArcFS Disk Usage Benchmark - Chart Generator

Reads JSON results from benchmarks/results/disk_usage/ and generates
comparison charts across ext4, btrfs, bindfs, and arcfs.

Usage:
    python3 benchmarks/generate_disk_usage_charts.py
"""

import json
import os
import sys
from typing import Any, Dict, List, Optional

import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

RESULTS_DIR = "benchmarks/results/disk_usage"
OUT_DIR = "benchmarks/charts/disk_usage"
os.makedirs(OUT_DIR, exist_ok=True)

FILESYSTEMS = ["ext4", "bindfs", "btrfs", "arcfs"]
FS_COLORS = {
    "ext4": "#4C72B0",
    "bindfs": "#8DA0CB",
    "btrfs": "#DD8452",
    "arcfs": "#55A868",
}
FS_MARKERS = {"ext4": "s", "bindfs": "D", "btrfs": "^", "arcfs": "o"}

TESTS = ["dedup", "compress", "snapshot", "smallfiles", "mixed"]


def load_result(test_name: str, fs: str) -> Optional[Dict[str, Any]]:
    path = os.path.join(RESULTS_DIR, f"{test_name}_{fs}.json")
    if not os.path.exists(path):
        print(f"  [WARN] Missing: {path}")
        return None
    with open(path, "r") as f:
        return json.load(f)


def bytes_to_mb(b: float) -> float:
    return b / (1024 * 1024)


def setup_style():
    plt.rcParams.update({
        "figure.facecolor": "white",
        "axes.facecolor": "#FAFAFA",
        "axes.grid": True,
        "grid.alpha": 0.3,
        "grid.linestyle": "--",
        "font.size": 11,
    })


# ==============================================================================
# Chart 1: Deduplication Curve
# ==============================================================================
def chart_dedup():
    print("[CHART] Generating dedup curve...")
    fig, ax = plt.subplots(figsize=(10, 6))

    has_data = False
    for fs in FILESYSTEMS:
        data = load_result("dedup", fs)
        if not data:
            continue
        has_data = True
        steps = data["steps"]
        copies = [s["copies"] for s in steps]
        physical_mb = [bytes_to_mb(s["physical_bytes"]) for s in steps]
        ax.plot(copies, physical_mb,
                marker=FS_MARKERS[fs], color=FS_COLORS[fs],
                linewidth=2, markersize=8, label=fs)

    if not has_data:
        print("  [SKIP] No dedup data found.")
        plt.close(fig)
        return

    # Add theoretical linear line (no dedup)
    sample = load_result("dedup", FILESYSTEMS[0])
    if sample:
        file_mb = sample["file_size_mb"]
        max_copies = max(s["copies"] for s in sample["steps"])
        theory_x = list(range(1, max_copies + 1))
        theory_y = [(1 + c) * file_mb for c in theory_x]  # source + copies
        ax.plot(theory_x, theory_y, linestyle=":", color="gray",
                linewidth=1.5, label="Linear (no dedup)", alpha=0.7)

    ax.set_xlabel("Number of Copies", fontsize=12)
    ax.set_ylabel("Physical Disk Usage (MB)", fontsize=12)
    ax.set_title("Deduplication Efficiency: Physical Disk vs. Copy Count", fontsize=14)
    ax.legend(fontsize=10)
    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))

    fig.tight_layout()
    out = os.path.join(OUT_DIR, "dedup_curve.png")
    fig.savefig(out, dpi=160)
    plt.close(fig)
    print(f"  Saved: {out}")


# ==============================================================================
# Chart 2: Compression Efficiency
# ==============================================================================
def chart_compress():
    print("[CHART] Generating compression efficiency bars...")
    fig, ax = plt.subplots(figsize=(12, 6))

    compress_levels = [0, 25, 50, 75, 100]
    x = np.arange(len(compress_levels))
    n_fs = 0
    bar_data = {}

    for fs in FILESYSTEMS:
        data = load_result("compress", fs)
        if not data:
            continue
        steps = {s["compress_pct"]: s for s in data["steps"]}
        ratios = [steps.get(p, {}).get("ratio", 1.0) for p in compress_levels]
        bar_data[fs] = ratios
        n_fs += 1

    if n_fs == 0:
        print("  [SKIP] No compression data found.")
        plt.close(fig)
        return

    width = 0.8 / n_fs
    for idx, (fs, ratios) in enumerate(bar_data.items()):
        offset = (idx - n_fs / 2 + 0.5) * width
        ax.bar(x + offset, ratios, width=width, color=FS_COLORS[fs], label=fs)

    ax.axhline(1.0, linestyle="--", color="gray", alpha=0.5, label="1:1 (no saving)")
    ax.set_xlabel("Data Compressibility (%)", fontsize=12)
    ax.set_ylabel("Storage Ratio (physical / logical)", fontsize=12)
    ax.set_title("Compression Efficiency by Compressibility Level", fontsize=14)
    ax.set_xticks(x)
    ax.set_xticklabels([f"{p}%" for p in compress_levels])
    ax.legend(fontsize=10)
    ax.set_ylim(0, max(max(r) for r in bar_data.values()) * 1.15)

    fig.tight_layout()
    out = os.path.join(OUT_DIR, "compression_efficiency.png")
    fig.savefig(out, dpi=160)
    plt.close(fig)
    print(f"  Saved: {out}")


# ==============================================================================
# Chart 3: Snapshot Growth
# ==============================================================================
def chart_snapshot():
    print("[CHART] Generating snapshot growth curve...")
    fig, ax = plt.subplots(figsize=(10, 6))

    has_data = False
    for fs in FILESYSTEMS:
        data = load_result("snapshot", fs)
        if not data:
            continue
        has_data = True
        steps = data["steps"]
        snaps = [s["snapshot"] for s in steps]
        physical_mb = [bytes_to_mb(s["physical_bytes"]) for s in steps]
        ax.plot(snaps, physical_mb,
                marker=FS_MARKERS[fs], color=FS_COLORS[fs],
                linewidth=2, markersize=8, label=fs)

    if not has_data:
        print("  [SKIP] No snapshot data found.")
        plt.close(fig)
        return

    ax.set_xlabel("Snapshot Count", fontsize=12)
    ax.set_ylabel("Cumulative Physical Disk Usage (MB)", fontsize=12)
    ax.set_title("Snapshot Storage Cost Over Time (10% churn per snapshot)", fontsize=14)
    ax.legend(fontsize=10)
    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))

    fig.tight_layout()
    out = os.path.join(OUT_DIR, "snapshot_growth.png")
    fig.savefig(out, dpi=160)
    plt.close(fig)
    print(f"  Saved: {out}")


# ==============================================================================
# Chart 4: Small Files Overhead (Stacked Bar)
# ==============================================================================
def chart_smallfiles():
    print("[CHART] Generating small files overhead bars...")
    fig, ax = plt.subplots(figsize=(8, 6))

    fs_labels = []
    data_mb_vals = []
    overhead_mb_vals = []

    for fs in FILESYSTEMS:
        data = load_result("smallfiles", fs)
        if not data:
            continue
        fs_labels.append(fs)
        data_mb_vals.append(bytes_to_mb(data["data_bytes"]))
        overhead_mb_vals.append(bytes_to_mb(data["overhead_bytes"]))

    if not fs_labels:
        print("  [SKIP] No smallfiles data found.")
        plt.close(fig)
        return

    x = np.arange(len(fs_labels))
    colors_data = [FS_COLORS[fs] for fs in fs_labels]
    colors_overhead = ["#E0E0E0"] * len(fs_labels)

    ax.bar(x, data_mb_vals, color=colors_data, label="Data")
    ax.bar(x, overhead_mb_vals, bottom=data_mb_vals, color=colors_overhead,
           edgecolor="#999999", linewidth=0.5, label="Metadata Overhead")

    # Annotate totals
    for i, fs in enumerate(fs_labels):
        total = data_mb_vals[i] + overhead_mb_vals[i]
        ax.text(i, total + 0.5, f"{total:.1f} MB", ha="center", fontsize=9)

    ax.set_xlabel("Filesystem", fontsize=12)
    ax.set_ylabel("Disk Usage (MB)", fontsize=12)
    data_obj = load_result("smallfiles", fs_labels[0])
    count = data_obj["file_count"] if data_obj else "?"
    ax.set_title(f"Small Files Test: {count} files - Data vs Metadata Overhead", fontsize=14)
    ax.set_xticks(x)
    ax.set_xticklabels(fs_labels)
    ax.legend(fontsize=10)

    fig.tight_layout()
    out = os.path.join(OUT_DIR, "smallfiles_overhead.png")
    fig.savefig(out, dpi=160)
    plt.close(fig)
    print(f"  Saved: {out}")


# ==============================================================================
# Chart 5: Mixed Realistic Workload (Horizontal Bar)
# ==============================================================================
def chart_mixed():
    print("[CHART] Generating mixed workload comparison...")
    fig, ax = plt.subplots(figsize=(10, 5))

    fs_labels = []
    physical_mb_vals = []
    logical_mb_vals = []

    for fs in FILESYSTEMS:
        data = load_result("mixed", fs)
        if not data:
            continue
        fs_labels.append(fs)
        physical_mb_vals.append(bytes_to_mb(data["physical_bytes"]))
        logical_mb_vals.append(bytes_to_mb(data["logical_bytes"]))

    if not fs_labels:
        print("  [SKIP] No mixed workload data found.")
        plt.close(fig)
        return

    y = np.arange(len(fs_labels))
    bar_colors = [FS_COLORS[fs] for fs in fs_labels]

    bars = ax.barh(y, physical_mb_vals, color=bar_colors, height=0.5)

    # Add logical size reference line
    if logical_mb_vals:
        avg_logical = sum(logical_mb_vals) / len(logical_mb_vals)
        ax.axvline(avg_logical, linestyle="--", color="gray", alpha=0.6,
                   label=f"Logical size (~{avg_logical:.0f} MB)")

    # Annotate bars with savings %
    for i, (phys, logic) in enumerate(zip(physical_mb_vals, logical_mb_vals)):
        if logic > 0:
            saving = (1 - phys / logic) * 100
            label = f"{phys:.1f} MB ({saving:+.1f}%)" if saving != 0 else f"{phys:.1f} MB"
        else:
            label = f"{phys:.1f} MB"
        ax.text(phys + 2, i, label, va="center", fontsize=10)

    ax.set_yticks(y)
    ax.set_yticklabels(fs_labels)
    ax.set_xlabel("Physical Disk Usage (MB)", fontsize=12)
    ax.set_title("Mixed Workload: Source Code + Logs + Binaries", fontsize=14)
    ax.legend(fontsize=10, loc="lower right")

    fig.tight_layout()
    out = os.path.join(OUT_DIR, "mixed_workload.png")
    fig.savefig(out, dpi=160)
    plt.close(fig)
    print(f"  Saved: {out}")


# ==============================================================================
# Chart 6: Summary Heatmap
# ==============================================================================
def chart_summary_heatmap():
    print("[CHART] Generating summary heatmap...")

    # Collect storage ratio (physical / logical) for each test × filesystem
    test_labels = []
    matrix = []

    for test in TESTS:
        row = []
        all_missing = True
        for fs in FILESYSTEMS:
            data = load_result(test, fs)
            if not data:
                row.append(float("nan"))
                continue
            all_missing = False

            if test == "dedup":
                # Use the last step (max copies)
                last = data["steps"][-1]
                p, l = last["physical_bytes"], last["logical_bytes"]
                row.append(p / l if l > 0 else 1.0)
            elif test == "compress":
                # Average ratio across all compression levels
                ratios = [s["ratio"] for s in data["steps"]]
                row.append(sum(ratios) / len(ratios) if ratios else 1.0)
            elif test == "snapshot":
                # Use the last step
                last = data["steps"][-1]
                p, l = last["physical_bytes"], last["logical_bytes"]
                row.append(p / l if l > 0 else 1.0)
            elif test == "smallfiles":
                p, l = data["physical_bytes"], data["logical_bytes"]
                row.append(p / l if l > 0 else 1.0)
            elif test == "mixed":
                p, l = data["physical_bytes"], data["logical_bytes"]
                row.append(p / l if l > 0 else 1.0)

        if not all_missing:
            test_labels.append(test)
            matrix.append(row)

    if not matrix:
        print("  [SKIP] No data for summary heatmap.")
        return

    arr = np.array(matrix, dtype=float)

    fig, ax = plt.subplots(figsize=(10, 6))
    im = ax.imshow(arr, aspect="auto", cmap="RdYlGn_r", vmin=0, vmax=max(1.5, np.nanmax(arr)))

    # Annotate cells
    for i in range(len(test_labels)):
        for j in range(len(FILESYSTEMS)):
            val = arr[i, j]
            if not np.isnan(val):
                text_color = "white" if val > 1.0 else "black"
                ax.text(j, i, f"{val:.2f}", ha="center", va="center",
                        fontsize=12, fontweight="bold", color=text_color)

    ax.set_xticks(np.arange(len(FILESYSTEMS)))
    ax.set_xticklabels(FILESYSTEMS, fontsize=12)
    ax.set_yticks(np.arange(len(test_labels)))
    ax.set_yticklabels(test_labels, fontsize=12)

    cbar = fig.colorbar(im, ax=ax, fraction=0.046, pad=0.04)
    cbar.set_label("Storage Ratio (physical / logical)\nLower = Better", fontsize=10)

    ax.set_title("Disk Usage Efficiency Summary\n(physical / logical ratio — lower is better)", fontsize=14)

    fig.tight_layout()
    out = os.path.join(OUT_DIR, "summary_heatmap.png")
    fig.savefig(out, dpi=160)
    plt.close(fig)
    print(f"  Saved: {out}")


# ==============================================================================
# Main
# ==============================================================================
def main():
    setup_style()

    if not os.path.isdir(RESULTS_DIR):
        print(f"[ERROR] Results directory not found: {RESULTS_DIR}")
        print("        Run benchmarks/run_disk_usage_benchmarks.sh first.")
        sys.exit(1)

    result_files = [f for f in os.listdir(RESULTS_DIR) if f.endswith(".json")]
    print(f"[INFO] Found {len(result_files)} result files in {RESULTS_DIR}")

    chart_dedup()
    chart_compress()
    chart_snapshot()
    chart_smallfiles()
    chart_mixed()
    chart_summary_heatmap()

    print(f"\n[DONE] All charts saved to {OUT_DIR}/")


if __name__ == "__main__":
    main()
