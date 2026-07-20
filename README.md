<!--
  SPDX-FileCopyrightText: 2026 Kubuno contributors
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

# Kubuno Notes

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-edition_2021-orange.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)
![Module](https://img.shields.io/badge/Kubuno-module-4D38DB.svg)

**Kubuno Notes — module de prise de notes**

A module for [Kubuno](https://github.com/kubuno/core), the self-hosted, libre (AGPLv3) cloud platform.

## Features

- **Notes** — WYSIWYG or Markdown editing, checklists, colors, pinning, archive and trash. Note contents are stored as `.kbnot` files, so your notes live alongside your other documents.
- **Notebooks & labels** — organize notes in nested notebooks and tag them with colored labels.
- **Full-text search** — accent-insensitive, weighted PostgreSQL full-text search across titles and contents.
- **Bidirectional links & graph** — link notes to each other and explore the resulting knowledge graph.
- **Reminders** — attach reminders to notes; a background worker delivers them when due.
- **Sharing** — share notes with other users, or publish them through public links.
- **Delta sync** — cursor-based `/notes/delta`, `/notebooks/delta` and `/labels/delta` endpoints expose an ordered, paginated change feed (including tombstones for deletions), designed for local-first clients: they can mint ids offline and replay their changes against the server.
- **Cross-module clipboard** — pasting data copied from another Kubuno module (an event, a contact, a place…) inserts a readable, linked Markdown block instead of a plain-text dump.
- **Linkable views** — every sidebar view (pinned, archived, a notebook, a label…) has a real, shareable URL, so deep links and the browser Back button just work.
- **Admin settings** — instance-wide defaults (editor mode, auto-save interval, spell check, bidirectional links, reminder lead time) are declared in `module.toml` and managed from the core's admin console.

## Architecture

A standalone Rust process that registers with the [core](https://github.com/kubuno/core) at startup; the core proxies its routes (`/api/v1/notes/*`) and serves its runtime-loaded React frontend bundle.

- **Backend** — `src/`: Axum + SQLx (PostgreSQL, schema `notes`); migrations in `migrations/`.
- **Frontend** — `frontend/`: a React bundle built to `entry.js`, consuming `@kubuno/sdk`, `@kubuno/ui` and `@kubuno/drive` from npm (provided by the host at runtime via the import map).

## Install

This module ships in the **all-in-one [Kubuno](https://github.com/kubuno/core) Docker image** (`ghcr.io/kubuno/kubuno`) — the easiest way to self-host a full Kubuno instance (core + every module). See **[kubuno/docker](https://github.com/kubuno/docker)** for `docker compose` instructions.

Prebuilt packages are attached to every [GitHub release](https://github.com/kubuno/notes/releases):

- **Debian/Ubuntu** — `kubuno-notes_*.deb`
- **Fedora / RHEL / openSUSE** — `kubuno-notes-*.rpm`
- **Windows** — `kubuno-notes-setup-*.exe` (NSIS installer, deposits the module into an existing core installation and restarts the service)
- **macOS** — `kubuno-notes-*.pkg`
- **Marketplace bundles** — flat `kubuno-notes-<version>-<os>-<arch>.{tar.gz,zip}` archives (with `.sha256` checksums) that the core downloads, verifies and extracts at runtime when the module is installed from the marketplace

To build any of these from source, see below.

## Build

**Requirements:** Rust ≥ 1.82, Node.js ≥ 24, PostgreSQL 16.

```bash
cargo build --release                     # → target/release/kubuno-notes
cd frontend && npm ci && npm run build     # → dist/{entry.js, entry.css}
bash build_deb.sh                          # → dist/kubuno-notes_*.deb
```

Other platform packages are produced by their dedicated scripts (all auto-detect the module id and version, and all follow the same on-disk layout so the core discovers the module identically everywhere):

```bash
bash build_rpm.sh                          # → dist/kubuno-notes-*.rpm       (Fedora/RHEL/openSUSE)
bash build_windows.sh                      # → dist/kubuno-notes-setup-*.exe (NSIS; native or cargo-xwin cross-build)
bash build_macos.sh                        # → dist/kubuno-notes-*.pkg       (run on macOS)
bash build_bundle.sh                       # → dist/kubuno-notes-<ver>-<os>-<arch>.{tar.gz,zip} (marketplace bundle + SHA-256)
```

The `dist.yml` GitHub Actions workflow builds the RPM, Windows and macOS installers plus the per-platform marketplace bundles on every `v*` tag and attaches them to the release (the Debian package is produced by `build.yml`).

> Shared dependencies come from Kubuno — no `kubuno/core` checkout required:
> - **Rust** — shared crates via tagged git dependencies on `kubuno/core`.
> - **Frontend** — `@kubuno/sdk`, `@kubuno/ui`, `@kubuno/drive` from the `@kubuno` npm scope.

## License

[AGPL-3.0-or-later](LICENSE) © Kubuno contributors.
