import glob
import json
import os
import re
from typing import Dict, Any, List, Tuple

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.gridspec import GridSpec


FILESYSTEMS = ["ext4", "bindfs", "btrfs", "arcfs"]
COLORS = {
    "ext4": "#1f77b4",
    "bindfs": "#9467bd",
    "btrfs": "#2ca02c",
    "arcfs": "#d62728",
}

PERCENTILE_KEYS = [
    "1.000000", "5.000000", "10.000000", "20.000000", "30.000000", "40.000000", "50.000000",
    "60.000000", "70.000000", "80.000000", "90.000000", "95.000000", "99.000000", "99.500000",
    "99.900000", "99.950000", "99.990000"
]
PERCENTILE_LABELS = ["P1", "P5", "P10", "P20", "P30", "P40", "P50", "P60", "P70", "P80", "P90", "P95", "P99", "P99.5", "P99.9", "P99.95", "P99.99"]

OUTPUT_DIR = os.path.dirname(os.path.abspath(__file__))


def discover_json_files(input_dir: str) -> List[str]:
    return sorted(glob.glob(os.path.join(input_dir, "**", "*.json"), recursive=True))


def parse_filename(file_path: str):
    name = os.path.basename(file_path)

    # Pattern A: {filesystem}_{profile}_mount.json
    m_a = re.match(r"^(ext4|bindfs|btrfs|arcfs)_(.+)_mount\.json$", name)
    if m_a:
        filesystem = m_a.group(1)
        profile = m_a.group(2)
        class_name = "legacy"
        return class_name, profile, filesystem

    # Pattern B: {profile}_{class}_{filesystem}_mount.json
    m_b = re.match(r"^(.+)_(responsive|durable)_(ext4|bindfs|btrfs|arcfs)_mount\.json$", name)
    if m_b:
        profile = m_b.group(1)
        class_name = m_b.group(2)
        filesystem = m_b.group(3)
        return class_name, profile, filesystem

    # Pattern C: {profile}_integrity_{filesystem}_mount.json
    m_c = re.match(r"^(.+)_integrity_(ext4|bindfs|btrfs|arcfs)_mount\.json$", name)
    if m_c:
        profile = m_c.group(1)
        filesystem = m_c.group(2)
        class_name = "integrity"
        return class_name, profile, filesystem

    return None, None, None


def safe_get_job0(payload: Dict[str, Any]) -> Dict[str, Any]:
    jobs = payload.get("jobs", [])
    if not jobs or not isinstance(jobs[0], dict):
        return {}
    return jobs[0]


def ns_to_ms(value: float) -> float:
    return (value or 0.0) / 1_000_000.0


def extract_percentile_series(ns_block: Dict[str, Any]) -> Tuple[List[float], bool]:
    pct = ns_block.get("percentile", {}) if isinstance(ns_block, dict) else {}
    if pct:
        values = [ns_to_ms(pct.get(key, 0.0)) for key in PERCENTILE_KEYS]
        return values, False

    # Fallback when no percentile map exists (common for slat/lat in many fio outputs)
    # Build a monotonic synthetic curve from min -> mean -> max to preserve readability.
    min_v = ns_to_ms(ns_block.get("min", 0.0) if isinstance(ns_block, dict) else 0.0)
    mean_v = ns_to_ms(ns_block.get("mean", 0.0) if isinstance(ns_block, dict) else 0.0)
    max_v = ns_to_ms(ns_block.get("max", 0.0) if isinstance(ns_block, dict) else 0.0)

    if max_v <= 0 and mean_v <= 0 and min_v <= 0:
        return [0.0] * len(PERCENTILE_KEYS), True

    left = np.linspace(max(min_v, 1e-9), max(mean_v, min_v, 1e-9), 9)
    right = np.linspace(max(mean_v, min_v, 1e-9), max(max_v, mean_v, min_v, 1e-9), len(PERCENTILE_KEYS) - 9)
    synthetic = np.concatenate([left, right]).tolist()
    return synthetic, True


def load_dataset(input_dir: str):
    # data[class][profile][filesystem] = metrics
    data: Dict[str, Dict[str, Dict[str, Dict[str, Any]]]] = {}

    for path in discover_json_files(input_dir):
        class_name, profile, filesystem = parse_filename(path)
        if not class_name or filesystem not in FILESYSTEMS:
            continue

        try:
            payload = json.load(open(path, "r", encoding="utf-8"))
        except Exception as exc:
            print(f"[WARN] Skipping unreadable file: {path} ({exc})")
            continue

        job0 = safe_get_job0(payload)
        write = job0.get("write", {})

        slat_series, slat_fallback = extract_percentile_series(write.get("slat_ns", {}))
        clat_series, clat_fallback = extract_percentile_series(write.get("clat_ns", {}))
        lat_series, lat_fallback = extract_percentile_series(write.get("lat_ns", {}))

        iodepth_level = job0.get("iodepth_level", {})
        iodepth_complete = job0.get("iodepth_complete", {})

        metrics = {
            "bw_mb_s": (write.get("bw", 0.0) or 0.0) / 1024.0,
            "iops": write.get("iops", 0.0) or 0.0,
            "sys_cpu": job0.get("sys_cpu", 0.0) or 0.0,
            "usr_cpu": job0.get("usr_cpu", 0.0) or 0.0,
            "ctx": job0.get("ctx", 0.0) or 0.0,
            "slat_ms": slat_series,
            "clat_ms": clat_series,
            "lat_ms": lat_series,
            "slat_fallback": slat_fallback,
            "clat_fallback": clat_fallback,
            "lat_fallback": lat_fallback,
            "iodepth_level": iodepth_level,
            "iodepth_complete": iodepth_complete,
        }

        data.setdefault(class_name, {}).setdefault(profile, {})[filesystem] = metrics

    return data


def values_for(profile_data: Dict[str, Dict[str, Any]], key: str) -> List[float]:
    return [profile_data.get(fs, {}).get(key, 0.0) for fs in FILESYSTEMS]


def plot_bar(ax, values: List[float], title: str, ylabel: str):
    bars = ax.bar(FILESYSTEMS, values, color=[COLORS[fs] for fs in FILESYSTEMS])
    ax.set_title(title)
    ax.set_ylabel(ylabel)
    ax.grid(axis="y", linestyle="--", alpha=0.35)
    for bar in bars:
        y = bar.get_height()
        ax.annotate(f"{y:.2f}", (bar.get_x() + bar.get_width()/2, y), textcoords="offset points", xytext=(0, 3), ha="center", fontsize=8)


def iodepth_matrix(profile_data: Dict[str, Dict[str, Any]], key_name: str, depth_keys: List[str]) -> np.ndarray:
    rows = []
    for fs in FILESYSTEMS:
        bucket = profile_data.get(fs, {}).get(key_name, {})
        rows.append([float(bucket.get(dk, 0.0) or 0.0) for dk in depth_keys])
    return np.array(rows, dtype=float)


def plot_iodepth_heatmap(ax, matrix: np.ndarray, x_labels: List[str], title: str):
    im = ax.imshow(matrix, aspect="auto")
    ax.set_title(title)
    ax.set_xticks(np.arange(len(x_labels)))
    ax.set_xticklabels(x_labels)
    ax.set_yticks(np.arange(len(FILESYSTEMS)))
    ax.set_yticklabels(FILESYSTEMS)
    plt.colorbar(im, ax=ax, fraction=0.046, pad=0.04)


def plot_latency_lines(ax, profile_data: Dict[str, Dict[str, Any]], field: str, title: str):
    fallback_used = False
    for fs in FILESYSTEMS:
        series = profile_data.get(fs, {}).get(field, [0.0] * len(PERCENTILE_KEYS))
        fb = profile_data.get(fs, {}).get(field.replace("_ms", "_fallback"), False)
        fallback_used = fallback_used or bool(fb)
        ax.plot(PERCENTILE_LABELS, series, marker="o", linewidth=1.8, label=fs, color=COLORS[fs])

    ax.set_title(title + (" (fallback for missing percentiles)" if fallback_used else ""))
    ax.set_ylabel("ms")
    ax.set_yscale("log")
    ax.grid(True, linestyle="--", alpha=0.35)
    ax.tick_params(axis="x", rotation=30)


def build_dashboard(class_name: str, profile: str, profile_data: Dict[str, Dict[str, Any]]):
    fig = plt.figure(figsize=(20, 14), constrained_layout=True)
    gs = GridSpec(3, 3, figure=fig, hspace=0.35, wspace=0.25)
    fig.suptitle(f"{class_name.upper()} :: {profile} Dashboard", fontsize=18, y=0.98)

    # Row 1: core perf + cpu
    ax1 = fig.add_subplot(gs[0, 0])
    plot_bar(ax1, values_for(profile_data, "bw_mb_s"), "Write Bandwidth", "MB/s")

    ax2 = fig.add_subplot(gs[0, 1])
    plot_bar(ax2, values_for(profile_data, "iops"), "Write IOPS", "ops/s")

    ax3 = fig.add_subplot(gs[0, 2])
    usr = values_for(profile_data, "usr_cpu")
    sys = values_for(profile_data, "sys_cpu")
    x = np.arange(len(FILESYSTEMS))
    ax3.bar(FILESYSTEMS, usr, color="#8da0cb", label="usr_cpu")
    ax3.bar(FILESYSTEMS, sys, bottom=usr, color="#fc8d62", label="sys_cpu")
    ax3.set_title("CPU Usage")
    ax3.set_ylabel("%")
    ax3.grid(axis="y", linestyle="--", alpha=0.35)
    ax3.legend(fontsize=8)

    # Row 2: iodepth maps + context switches
    ax4 = fig.add_subplot(gs[1, 0])
    depth_level_keys = ["1", "2", "4", "8", "16", "32", ">=64"]
    mat_level = iodepth_matrix(profile_data, "iodepth_level", depth_level_keys)
    plot_iodepth_heatmap(ax4, mat_level, depth_level_keys, "iodepth_level (%)")

    ax5 = fig.add_subplot(gs[1, 1])
    depth_complete_keys = ["0", "4", "8", "16", "32", "64", ">=64"]
    mat_complete = iodepth_matrix(profile_data, "iodepth_complete", depth_complete_keys)
    plot_iodepth_heatmap(ax5, mat_complete, depth_complete_keys, "iodepth_complete (%)")

    ax6 = fig.add_subplot(gs[1, 2])
    plot_bar(ax6, values_for(profile_data, "ctx"), "Context Switches", "count")

    # Row 3: latency percentile curves
    ax7 = fig.add_subplot(gs[2, 0])
    plot_latency_lines(ax7, profile_data, "slat_ms", "slat percentile curve")

    ax8 = fig.add_subplot(gs[2, 1])
    plot_latency_lines(ax8, profile_data, "clat_ms", "clat percentile curve")

    ax9 = fig.add_subplot(gs[2, 2])
    plot_latency_lines(ax9, profile_data, "lat_ms", "lat percentile curve")
    ax9.legend(fontsize=8)

    out_file = os.path.join(OUTPUT_DIR, f"dashboard_{class_name}_{profile}.png")
    fig.set_constrained_layout_pads(w_pad=0.02, h_pad=0.02, wspace=0.03, hspace=0.04)
    fig.savefig(out_file, dpi=170)
    plt.close(fig)
    return out_file


def main():
    data = load_dataset(OUTPUT_DIR)
    produced = []

    for class_name, profiles in sorted(data.items()):
        for profile, profile_data in sorted(profiles.items()):
            if not profile_data:
                continue
            out_file = build_dashboard(class_name, profile, profile_data)
            produced.append(out_file)

    if not produced:
        print("No dashboards generated. Check input filenames and JSON location.")
        return

    print("Generated profile dashboards:")
    for path in produced:
        print(f"- {path}")


if __name__ == "__main__":
    main()
