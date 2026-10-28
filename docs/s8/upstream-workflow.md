# Upstream workflow

Two upstream projects matter: the bridge
([jtoronto/S4MK3-HID-MIDI-BRIDGE](https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE))
and its protocol layer ([RufusIbiza/Encdr](https://github.com/RufusIbiza/Encdr)).
The aim is to keep S8 work small and separable so useful pieces can be
offered back.

## Remotes

| Remote     | URL                                                         | Use                      |
| ---------- | ----------------------------------------------------------- | ------------------------ |
| `origin`   | `gootecks/S4MK3-HID-MIDI-BRIDGE`                            | Fork; PRs land here.     |
| `upstream` | `https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE.git`     | Read-only source of truth. |

Set up once per clone:

```sh
git remote add upstream https://github.com/jtoronto/S4MK3-HID-MIDI-BRIDGE.git
git fetch upstream --tags
```

In `../Encdr`, `origin` is `gootecks/Encdr`; add
`https://github.com/RufusIbiza/Encdr.git` as `upstream` the same way before
preparing an Encdr PR.

## Branches

- Never commit directly to `main`. Every change starts from a GitHub issue and
  a branch from `main`: `feat/N-description`, `fix/N-…`, `chore/N-…`,
  `docs/N-…`. PRs into the fork use `Closes #N`.
- Use conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, `test:`,
  `refactor:`). The hk `commit-msg` hook enforces the format.
- Sync upstream through a branch, not by rewriting `main`:

  ```sh
  git fetch upstream --tags
  git checkout -b chore/N-sync-upstream main
  git merge upstream/main
  ```

## Keeping commits upstreamable

Classify each change before writing it. Issue labels record the class.

| Class             | Label             | Examples                                                    |
| ----------------- | ----------------- | ----------------------------------------------------------- |
| Fork-only         | `fork-only`       | `docs/s8/`, `CHANGELOG.md`, `mise.toml`, `hk.pkl`, `communique.toml`, `fnox.toml`, fork sections of README/AGENTS |
| Bridge candidate  | `upstream:bridge` | Device-profile seam, bug fixes, S4-neutral refactors         |
| Encdr candidate   | `upstream:encdr`  | S8 descriptor corrections, LED/screen protocol fixes         |
| S8 feature        | `device:s8`       | S8 catalog, mapping, app integration                         |

Rules:

1. Never mix classes in one commit. A feature commit must not touch fork-only
   files; record its changelog entry in a separate commit.
2. Upstream candidates keep upstream conventions: existing module layout,
   `AGENTS.md` gates, comment style, user-guide/README audience rules, and no
   reformatting of untouched code or docs.
3. Keep S4 MK3 behaviour byte-identical unless the change is an explicit S4 fix.
   Prove it with the existing test suite and, for mapping changes, an unchanged
   generated S4 mapping.
4. Prefer additive seams (a profile, a second catalog) over renames. Leave
   S4-named identifiers (`S4BridgeApp`, crate name, bundle name) alone unless
   upstream agrees to rename them.
5. Large refactors of files over 500 lines (`src/full/mod.rs`,
   `src/full/mapping.rs`, `macos/S4BridgeApp.swift`, …) are out of scope unless
   a feature needs them; discuss with upstream first.

## Offering a change upstream

```sh
git fetch upstream
git checkout -b upstream/<topic> upstream/main
git cherry-pick <feature commits>   # never fork-only commits
cargo fmt --all -- --check && cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked && swift test --package-path macos
git push origin upstream/<topic>
gh pr create --repo jtoronto/S4MK3-HID-MIDI-BRIDGE --base main --head gootecks:upstream/<topic>
```

The `upstream/<topic>` branch must contain only upstream-candidate commits.
For a design-level change (e.g. the device-profile seam, #5), open an issue
or discussion upstream before the PR.

## Encdr changes

1. Make the change on a branch in `../Encdr` from `upstream/main` with tests
   for the descriptor or protocol path.
2. Open the PR against RufusIbiza/Encdr.
3. Point the bridge at the merged upstream revision. Pinning the gootecks fork
   is a temporary measure only, recorded in the issue and `CHANGELOG.md`.
