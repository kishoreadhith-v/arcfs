# FGCS Submission Checklist — ArcFS

## Details to Fill In

All placeholders are marked `[FILL IN` in `paper_fgcs.tex`. Use your editor's search to find each one.

---

### Authors

- [ ] Corresponding author's **public ORCID** — `https://orcid.org/XXXX-XXXX-XXXX-XXXX`
  - Log in to orcid.org → Account settings → set visibility to **Everyone**
  - FGCS will desk-reject the paper if this is not publicly visible
- [ ] **Faculty email** for Dr. Arul Anand N (`[FILL IN: faculty email @psgtech.ac.in]`)

---

### Table 1 — System Configuration (`paper_fgcs.tex` lines ~1029–1039)

- [ ] CPU — model, core count, clock speed
- [ ] RAM — total capacity and speed (e.g., `16 GB DDR4-3200`)
- [ ] Storage — device type and model (e.g., `Samsung 970 EVO Plus NVMe SSD, 1 TB`)
- [ ] OS — Linux distribution + kernel version (e.g., `Ubuntu 22.04.3, kernel 5.15.0-91`)
- [ ] ext4 — mount options used during benchmarks (e.g., `-o defaults`)
- [ ] Btrfs — mount options used (confirm no `compress=zstd` was used)
- [ ] bindfs — version number (`bindfs --version`)
- [ ] fio — version number (`fio --version`)

---

### Acknowledgements

- [ ] Add any lab-mates, reviewers, or infrastructure providers beyond the department

---

### CRediT Authorship Statement

- [ ] Verify the role assignments for each author are accurate before submission
  - Current assignments are reasonable guesses — authors must confirm

---

### Funding

- [ ] If any scholarship, grant, or institutional funding exists, replace the "no funding" statement with the standard Elsevier format:
  > "This work was supported by [Agency] [grant number xxxx]."
- [ ] If truly no funding: statement is already correct, no action needed

---

### Data Availability

- [ ] Push the ArcFS repository to **GitHub** (if not already public)
- [ ] Deposit a release on **Zenodo** (zenodo.org → link GitHub repo → create release → get DOI)
- [ ] Fill in the URL in `paper_fgcs.tex`: `[FILL IN: GitHub or Zenodo repository URL with DOI]`
  - Format: `https://doi.org/10.5281/zenodo.XXXXXXX`

---

### Author Vitae (6 authors, max 100 words each)

- [ ] **Anandkumar N S** — complete bio + provide `Author1_photo.jpg` (passport-type, min 300 dpi)
- [ ] **Dhakkshin S R** — complete bio + provide `Author2_photo.jpg`
- [ ] **Kishoreadhith V** — complete bio + provide `Author3_photo.jpg`
- [ ] **M Raj Ragavender** — complete bio + provide `Author4_photo.jpg`
- [ ] **Rithvik K** — complete bio + provide `Author5_photo.jpg`
- [ ] **Arul Anand N** — complete bio (add research interests) + provide `Author6_photo.jpg`

---

## Files to Submit

| File | Status | Action needed |
|---|---|---|
| `paper_fgcs.tex` | Done | Fill in placeholders above |
| `references.bib` | Done | No changes needed |
| `highlights.tex` | Done | Verify each bullet ≤ 85 chars |
| `figures/arcfs_vs_ext4_by_class.png` | Done | Submit as separate file |
| `figures/responsive_per_job_bars.png` | Done | Submit as separate file |
| `figures/durable_per_job_bars.png` | Done | Submit as separate file |
| `figures/integrity_summary.png` | Done | Submit as separate file |
| `figures/dedup_curve.png` | Done | Submit as separate file |
| `figures/compression_efficiency.png` | Done | Submit as separate file |
| `figures/snapshot_growth.png` | Done | Submit as separate file |
| `figures/summary_heatmap.png` | Done | Submit as separate file |
| `Author1_photo.jpg` … `Author6_photo.jpg` | **Missing** | Provide passport-type photos |
| Competing interests Word doc | **Missing** | Complete via Elsevier declarations tool at submission time |

---

## Submission Notes

- Page count: **15 pages** (limit is 18) — within limit
- Keywords: **8** (limit is 6–10) — within limit
- Abstract: **~230 words** (limit is 250) — within limit
- `highlights.tex` must be uploaded as a **separate file** with "highlights" in the filename
- Author photos must be uploaded as **separate figure files**
- The Elsevier Editorial Manager system will prompt for the competing interests declaration during submission — do not add it manually
