import json
import os
from typing import Dict, Any, List, Tuple

import matplotlib.pyplot as plt
import numpy as np

RESULTS_DIR = "benchmarks/results"
OUT_DIR = "benchmarks/charts"
os.makedirs(OUT_DIR, exist_ok=True)

PERF_CLASSES = ["responsive", "durable"]
PERF_JOBS = ["seq_write", "rand_write", "realistic_mix", "massive_stream", "paranoid_db"]
INTEGRITY_PROFILES = ["seq_verify", "rand4k_verify", "rand64k_verify", "fsync4k_verify"]
MOUNTS = ["ext4_mount", "bindfs_mount", "btrfs_mount", "arcfs_mount"]
LABELS = ["ext4", "bindfs", "btrfs", "arcfs"]


def read_json(path: str) -> Dict[str, Any]:
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle)


def safe_percentile_ns(section: Dict[str, Any], percentile: str) -> float:
    return section.get("clat_ns", {}).get("percentile", {}).get(percentile, 0.0)


def parse_perf_stats(class_name: str, job_name: str, mount_name: str) -> Dict[str, float]:
    filepath = os.path.join(RESULTS_DIR, class_name, mount_name, f"{job_name}_{class_name}_{mount_name}.json")
    if not os.path.exists(filepath):
        return {
            "read_bw_mb": 0.0,
            "write_bw_mb": 0.0,
            "read_iops": 0.0,
            "write_iops": 0.0,
            "write_lat_mean_ms": 0.0,
            "write_lat_p99_ms": 0.0,
            "write_lat_p999_ms": 0.0,
            "runtime_ms": 0.0,
            "usr_cpu": 0.0,
            "sys_cpu": 0.0,
        }

    fio_job = read_json(filepath)["jobs"][0]
    read = fio_job.get("read", {})
    write = fio_job.get("write", {})

    return {
        "read_bw_mb": read.get("bw_bytes", 0.0) / (1024 * 1024),
        "write_bw_mb": write.get("bw_bytes", 0.0) / (1024 * 1024),
        "read_iops": read.get("iops", 0.0),
        "write_iops": write.get("iops", 0.0),
        "write_lat_mean_ms": write.get("clat_ns", {}).get("mean", 0.0) / 1e6,
        "write_lat_p99_ms": safe_percentile_ns(write, "99.000000") / 1e6,
        "write_lat_p999_ms": safe_percentile_ns(write, "99.900000") / 1e6,
        "runtime_ms": fio_job.get("job_runtime", 0.0),
        "usr_cpu": fio_job.get("usr_cpu", 0.0),
        "sys_cpu": fio_job.get("sys_cpu", 0.0),
    }


def parse_integrity_stats(profile_name: str, mount_name: str) -> Dict[str, float]:
    filepath = os.path.join(RESULTS_DIR, "integrity", mount_name, f"{profile_name}_integrity_{mount_name}.json")
    if not os.path.exists(filepath):
        return {
            "error": 1.0,
            "write_bw_mb": 0.0,
            "write_iops": 0.0,
            "write_p99_ms": 0.0,
            "runtime_ms": 0.0,
        }

    fio_job = read_json(filepath)["jobs"][0]
    write = fio_job.get("write", {})

    return {
        "error": float(fio_job.get("error", 1.0)),
        "write_bw_mb": write.get("bw_bytes", 0.0) / (1024 * 1024),
        "write_iops": write.get("iops", 0.0),
        "write_p99_ms": safe_percentile_ns(write, "99.000000") / 1e6,
        "runtime_ms": fio_job.get("job_runtime", 0.0),
    }


def load_perf_data() -> Dict[str, Dict[str, Dict[str, Dict[str, float]]]]:
    all_data: Dict[str, Dict[str, Dict[str, Dict[str, float]]]] = {}
    for class_name in PERF_CLASSES:
        all_data[class_name] = {}
        for job in PERF_JOBS:
            all_data[class_name][job] = {}
            for mount in MOUNTS:
                all_data[class_name][job][mount] = parse_perf_stats(class_name, job, mount)
    return all_data


def load_integrity_data() -> Dict[str, Dict[str, Dict[str, float]]]:
    integrity: Dict[str, Dict[str, Dict[str, float]]] = {}
    for profile in INTEGRITY_PROFILES:
        integrity[profile] = {}
        for mount in MOUNTS:
            integrity[profile][mount] = parse_integrity_stats(profile, mount)
    return integrity


def grouped_bars(ax, title: str, ylabel: str, series: List[Tuple[str, List[float]]]):
    x = np.arange(len(LABELS))
    width = 0.82 / max(len(series), 1)
    start = -0.41 + width / 2
    for idx, (name, values) in enumerate(series):
        ax.bar(x + start + idx * width, values, width=width, label=name)
    ax.set_title(title)
    ax.set_ylabel(ylabel)
    ax.set_xticks(x)
    ax.set_xticklabels(LABELS)
    ax.grid(axis="y", linestyle="--", alpha=0.35)
    ax.legend(fontsize=8)


def chart_class_detail_heatmaps(class_name: str, class_data: Dict[str, Dict[str, Dict[str, float]]]):
    metrics = [
        ("write_bw_mb", "Write BW (MB/s)", False),
        ("write_iops", "Write IOPS", False),
        ("write_lat_p99_ms", "Write P99 Latency (ms)", True),
        ("runtime_ms", "Runtime (ms)", False),
    ]

    fig, axes = plt.subplots(2, 2, figsize=(15, 10))
    fig.suptitle(f"{class_name.capitalize()} Class - Per Profile Detail (No Averages)", fontsize=16)
    axes = axes.flatten()

    for idx, (metric_key, metric_title, use_log) in enumerate(metrics):
        matrix = []
        for job in PERF_JOBS:
            matrix.append([class_data[job][m][metric_key] for m in MOUNTS])

        plot_matrix = np.array(matrix, dtype=float)
        if use_log:
            plot_matrix = np.log10(np.clip(plot_matrix, 1e-6, None))

        im = axes[idx].imshow(plot_matrix, aspect="auto")
        axes[idx].set_title(metric_title)
        axes[idx].set_xticks(np.arange(len(LABELS)))
        axes[idx].set_xticklabels(LABELS, rotation=20)
        axes[idx].set_yticks(np.arange(len(PERF_JOBS)))
        axes[idx].set_yticklabels(PERF_JOBS)
        cbar = fig.colorbar(im, ax=axes[idx], fraction=0.046, pad=0.04)
        cbar.set_label("log10(value)" if use_log else "value")

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, f"{class_name}_detail_heatmaps.png"), dpi=160)
    plt.close(fig)


def chart_class_per_job_bars(class_name: str, class_data: Dict[str, Dict[str, Dict[str, float]]]):
    fig, axes = plt.subplots(2, 3, figsize=(18, 10))
    axes = axes.flatten()
    x = np.arange(len(LABELS))

    for idx, job in enumerate(PERF_JOBS):
        ax = axes[idx]
        bw = [class_data[job][m]["write_bw_mb"] for m in MOUNTS]
        iops = [class_data[job][m]["write_iops"] for m in MOUNTS]
        p99 = [class_data[job][m]["write_lat_p99_ms"] for m in MOUNTS]

        width = 0.27
        ax.bar(x - width, bw, width=width, label="BW MB/s")
        ax.bar(x, iops, width=width, label="IOPS")
        ax.bar(x + width, p99, width=width, label="P99 ms")
        ax.set_title(job)
        ax.set_xticks(x)
        ax.set_xticklabels(LABELS, rotation=20)
        ax.grid(axis="y", linestyle="--", alpha=0.35)
        ax.legend(fontsize=7)

    axes[-1].axis("off")
    fig.suptitle(f"{class_name.capitalize()} Class - Per Job Metric Bars", fontsize=16)
    fig.tight_layout(rect=[0, 0, 1, 0.95])
    fig.savefig(os.path.join(OUT_DIR, f"{class_name}_per_job_bars.png"), dpi=160)
    plt.close(fig)


def chart_latency_ratio_vs_ext4(class_name: str, class_data: Dict[str, Dict[str, Dict[str, float]]]):
    fig, ax = plt.subplots(figsize=(10, 5))
    x = np.arange(len(PERF_JOBS))

    for mount, label in zip(MOUNTS, LABELS):
        if mount == "ext4_mount":
            continue
        ratios = []
        for job in PERF_JOBS:
            base = class_data[job]["ext4_mount"]["write_lat_p99_ms"]
            val = class_data[job][mount]["write_lat_p99_ms"]
            ratio = (val / base) if base > 0 else 0.0
            ratios.append(ratio)
        ax.plot(x, ratios, marker="o", label=f"{label}/ext4")

    ax.axhline(1.0, linestyle="--", linewidth=1)
    ax.set_title(f"{class_name.capitalize()} - P99 Latency Ratio vs ext4 (per job)")
    ax.set_ylabel("ratio (lower is better)")
    ax.set_xticks(x)
    ax.set_xticklabels(PERF_JOBS, rotation=20)
    ax.grid(axis="y", linestyle="--", alpha=0.35)
    ax.legend()
    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, f"{class_name}_p99_ratio_vs_ext4.png"), dpi=160)
    plt.close(fig)


def chart_arcfs_vs_ext4(perf_data: Dict[str, Dict[str, Dict[str, Dict[str, float]]]]):
    fig, axes = plt.subplots(1, 2, figsize=(14, 5))
    x = np.arange(len(PERF_JOBS))
    width = 0.35

    for ax_idx, class_name in enumerate(PERF_CLASSES):
        class_data = perf_data[class_name]
        ext4_bw = [class_data[j]["ext4_mount"]["write_bw_mb"] for j in PERF_JOBS]
        arc_bw = [class_data[j]["arcfs_mount"]["write_bw_mb"] for j in PERF_JOBS]
        axes[ax_idx].bar(x - width / 2, ext4_bw, width, label="ext4")
        axes[ax_idx].bar(x + width / 2, arc_bw, width, label="arcfs")
        axes[ax_idx].set_title(f"{class_name.capitalize()}: ArcFS vs ext4 Write BW")
        axes[ax_idx].set_ylabel("MB/s")
        axes[ax_idx].set_xticks(x)
        axes[ax_idx].set_xticklabels(PERF_JOBS, rotation=20)
        axes[ax_idx].grid(axis="y", linestyle="--", alpha=0.35)
        axes[ax_idx].legend()

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "arcfs_vs_ext4_by_class.png"), dpi=160)
    plt.close(fig)


def chart_integrity_summary(integrity_data: Dict[str, Dict[str, Dict[str, float]]]):
    error_matrix = []
    bw_matrix = []
    runtime_matrix = []
    for profile in INTEGRITY_PROFILES:
        error_matrix.append([integrity_data[profile][m]["error"] for m in MOUNTS])
        bw_matrix.append([integrity_data[profile][m]["write_bw_mb"] for m in MOUNTS])
        runtime_matrix.append([integrity_data[profile][m]["runtime_ms"] for m in MOUNTS])

    fig, axes = plt.subplots(1, 3, figsize=(17, 5))

    im0 = axes[0].imshow(np.array(error_matrix), aspect="auto")
    axes[0].set_title("Integrity Errors (0=pass)")
    axes[0].set_xticks(np.arange(len(LABELS)))
    axes[0].set_xticklabels(LABELS, rotation=20)
    axes[0].set_yticks(np.arange(len(INTEGRITY_PROFILES)))
    axes[0].set_yticklabels(INTEGRITY_PROFILES)
    fig.colorbar(im0, ax=axes[0], fraction=0.046, pad=0.04)

    im1 = axes[1].imshow(np.array(bw_matrix), aspect="auto")
    axes[1].set_title("Integrity Write BW (MB/s)")
    axes[1].set_xticks(np.arange(len(LABELS)))
    axes[1].set_xticklabels(LABELS, rotation=20)
    axes[1].set_yticks(np.arange(len(INTEGRITY_PROFILES)))
    axes[1].set_yticklabels(INTEGRITY_PROFILES)
    fig.colorbar(im1, ax=axes[1], fraction=0.046, pad=0.04)

    im2 = axes[2].imshow(np.array(runtime_matrix), aspect="auto")
    axes[2].set_title("Integrity Runtime (ms)")
    axes[2].set_xticks(np.arange(len(LABELS)))
    axes[2].set_xticklabels(LABELS, rotation=20)
    axes[2].set_yticks(np.arange(len(INTEGRITY_PROFILES)))
    axes[2].set_yticklabels(INTEGRITY_PROFILES)
    fig.colorbar(im2, ax=axes[2], fraction=0.046, pad=0.04)

    fig.tight_layout()
    fig.savefig(os.path.join(OUT_DIR, "integrity_summary.png"), dpi=160)
    plt.close(fig)


def main():
    perf_data = load_perf_data()
    integrity_data = load_integrity_data()

    for class_name in PERF_CLASSES:
        chart_class_detail_heatmaps(class_name, perf_data[class_name])
        chart_class_per_job_bars(class_name, perf_data[class_name])
        chart_latency_ratio_vs_ext4(class_name, perf_data[class_name])

    chart_arcfs_vs_ext4(perf_data)
    chart_integrity_summary(integrity_data)

    print(f"Generated final charts in {OUT_DIR}")


if __name__ == "__main__":
    main()
