---
name: license-check
description: Check the license of a third-party repo, crate, package, sample, font or document before we depend on it, copy from it or ship it. Use whenever code or assets from outside this repo are cloned, added as a dependency, copied, or referenced in docs.
---

# License check

Goal: know, before we use something, whether we may (a) depend on it, (b) copy code or data from it, (c) ship it, and what notice we owe. Record the answer so nobody has to check again.

This is not legal advice. When the answer is unclear, stop and ask Rickard.

## 1. Find the license (in this order)
1. License files in the repo root: `LICENSE`, `LICENSE-MIT`, `LICENSE-APACHE`, `COPYING`, `NOTICE`.
2. Package metadata: `Cargo.toml` (`license =`), `package.json` (`"license"`), `setup.py` / `pyproject.toml`, `go.mod` has none, so use the repo files.
3. GitHub API: `gh api repos/<owner>/<repo> --jq '.license.spdx_id'`.
   - `NOASSERTION` means GitHub could not decide (often two license files). Read the files and the metadata yourself.
   - `null` / no license means **all rights reserved**.
4. Per-file headers (SPDX lines such as `SPDX-License-Identifier: MIT`), and README sections.
5. For samples, fonts, images and documents: the source page and its terms. A manual can be "free to read" but not free to reproduce.

## 2. Classify

| Found | Depend on it | Copy code/data | Ship in our repo | We owe |
|---|---|---|---|---|
| MIT, BSD-2/3, ISC, Apache-2.0, Zlib | Yes | Yes | Yes | Keep copyright + license text; Apache-2.0 also NOTICE and change notes |
| MPL-2.0 | Yes | Only whole files, kept MPL | Yes | Source of those files stays MPL |
| LGPL | Yes, dynamic link preferred | Ask | Ask | Ask Rickard |
| GPL / AGPL | **Ask first** | **Ask first** | **Ask first** | Can force our whole project under GPL |
| CC-BY | Yes (assets) | Yes | Yes | Attribution |
| CC-BY-NC / -ND | **Ask first** | No for ND | Ask | Non-commercial / no derivatives |
| No license | No dependency | **No** | **No** | Read and learn only; re-implement from public specs |
| Custom terms (for example Ableton's manual) | Read the terms | Usually no | Usually no | Summarise and link, never reproduce |

Note: `pt2-clone` (our `src/`) is BSD-3-Clause; keep its license and do not edit `src/`.

## 3. Facts vs code
Facts (CC numbers, USB IDs, byte layouts from a public spec) are not copyrightable on their own. Verifying our numbers against a no-license repo is fine. Copying its source, comments, tables in its exact form, or structure is not.

## 4. Record it
Add one row per item to `docs/references/third-party.md`:

| Name | Source URL | License (SPDX) | Checked | Use | Notice needed |
|---|---|---|---|---|---|

- "Use" is one of: dependency, copied (which files), reference only, not used.
- "Checked" is the date, `YYYY-MM-DD`.
- When something is copied, keep its license text in `third_party/<name>/LICENSE`.

## 5. Stop conditions
Stop and ask Rickard when: the license is GPL/AGPL/LGPL/NC/ND or custom; there is no license and we want more than reading; the metadata and the license file disagree; or a sample's origin is unknown.

## Known results
| Name | License | Use |
|---|---|---|
| `mbracher/push2_display` | MIT OR Apache-2.0 (Cargo.toml; GitHub says NOASSERTION) | Dependency OK |
| `ffont/push2-python` | MIT | Copy pad/CC constants with notice OK |
| `federico-pepe/push-tethered-app` | None (all rights reserved) | Reference only, do not copy |
| Ableton Push 2 interface manual | Custom: use allowed, reproduction needs consent | Summarise and link only |
