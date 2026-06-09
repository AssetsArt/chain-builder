---
name: release
description: Cut a chain-builder crates.io release. Use when the user asks to release, publish, cut a version, or tag a release of this project. Enforces the order PR → green checks → merge to main → tag ON MAIN (never tag a feature branch), bumps every version reference, and watches the publish.
---

# Releasing chain-builder

`publish.yml` publishes the `chain-builder` crate to **crates.io** on a `v*` tag.
Unlike a bare publish, this workflow **gates itself**: it runs the test suite,
verifies the tag matches the `Cargo.toml` version, and does a `cargo publish
--dry-run` before the real `cargo publish`. Even so, YOU are the correctness
gate — never tag until the tree is green on `main` and the version is bumped.

A published crates.io version is **immutable and can never be reused** — every
release MUST bump the version, or the publish step fails with "crate version
already uploaded".

## The order (do NOT deviate)

```
1. Push branch        → 2. Open PR (→ main)   → 3. WAIT for checks GREEN
→ 4. Merge PR to main → 5. Bump version on main → 6. Tag vX.Y.Z ON MAIN → push tag
→ 7. Watch publish.yml to success
```

**Never tag a feature branch.** The tag must point at the merged commit on `main`
— `publish.yml` builds and publishes from the tagged commit, so the tag has to be
the real release commit.

## 1–2. Pre-flight gates + PR

Mirror what `publish.yml` enforces (hard gates), before opening the PR:
```bash
cargo test --features dev-dependencies --no-fail-fast   # publish.yml runs this
cargo publish --dry-run                                  # publish.yml runs this (packaging)
```
Recommended hygiene (not gated by `publish.yml`, but keep the tree clean):
```bash
cargo fmt --all --check
cargo build --no-default-features --features sqlx_sqlite,sqlite   # sqlite-only must compile
cargo clippy --features dev-dependencies                         # advisory: a few pre-existing lints exist
```
Then: `git push origin <branch>` and `gh pr create --base main --head <branch> …`.

> Note: this repo has **no separate PR-CI workflow** — `publish.yml` only runs on
> tags. So the local pre-flight above IS the pre-merge gate; run it yourself.

## 3. Wait for checks — GREEN before merge

```bash
gh pr checks <pr#> --watch      # if/when PR checks exist; otherwise rely on local pre-flight
```
Do not proceed until everything passes. If red, fix on the branch and repeat.

## 4. Merge to main

```bash
gh pr merge <pr#> --merge --delete-branch    # repo convention so far: merge commits
git checkout main && git pull origin main
```

## 5. Bump version ON MAIN

Current shipped version = `Cargo.toml` `version` (and the last `v*` tag).
Increment per semver. **Every** reference must move or docs/lockfile drift:

```bash
OLD=1.0.1; NEW=1.0.2          # set these

# 1) Crate version (the source of truth `publish.yml` checks against the tag)
sed -i '' "s/^version = \"$OLD\"/version = \"$NEW\"/" Cargo.toml

# 2) Sync Cargo.lock's own entry for this crate
cargo build >/dev/null         # rewrites chain-builder's version in Cargo.lock
#   (or: cargo update -p chain-builder --precise $NEW)

# 3) README install examples (4 pins) — set them all to $NEW.
#    NOTE: README pins may LAG behind Cargo.toml (e.g. README at 1.0.0 while the
#    crate is 1.0.1). Find their current value first, then replace that → $NEW:
grep -n 'chain-builder = ' README.md           # see what the pins currently say
README_OLD=1.0.0                                # ← set from the grep above
sed -i '' "s/chain-builder = \"$README_OLD\"/chain-builder = \"$NEW\"/g" README.md
sed -i '' "s/chain-builder = { version = \"$README_OLD\"/chain-builder = { version = \"$NEW\"/g" README.md

# Verify Cargo.toml/lock no longer mention $OLD, and README pins are all $NEW:
grep -rn "$OLD" Cargo.toml Cargo.lock          # expect: no chain-builder lines
grep -n 'chain-builder = ' README.md           # expect: all show $NEW
```

Then update `CHANGELOG.md`: rename the `## [Unreleased]` heading to
`## [$NEW] - YYYY-MM-DD` (today's date), keeping its Security/Changed/Breaking
subsections. Leave a fresh empty `## [Unreleased]` above it if you like.

Commit: `chore(release): $NEW`.

## 6. Tag on main + push

The tag (minus the leading `v`) MUST equal `Cargo.toml` `version` or `publish.yml`
fails the "Verify tag matches Cargo.toml version" step.

```bash
git push origin main             # the bump commit first
git tag -a v$NEW -m "chain-builder $NEW — <summary>"
git push origin v$NEW            # ← triggers publish.yml
```

## 7. Watch the publish

```bash
gh run watch "$(gh run list --workflow=publish.yml --limit 1 --json databaseId --jq '.[0].databaseId')" --exit-status
```
Confirm on crates.io: <https://crates.io/crates/chain-builder/versions>.

## publish.yml notes (footguns)

- **Auth**: `cargo publish` uses `CARGO_REGISTRY_TOKEN` ← the **`RUST_TOKEN`
  organization secret**. It must (a) exist, (b) grant *this* repo access
  (Org → Settings → Secrets and variables → Actions → `RUST_TOKEN` → Repository
  access), and (c) belong to a crates.io **owner** of `chain-builder`.
- **Immutable versions**: you cannot republish or yank-then-reuse a version. If a
  tag was pushed with a bad version, delete the tag (`git push origin :vX.Y.Z`),
  fix, and tag a *new* version.
- **`workflow_dispatch`**: runs the same job manually (tests + dry-run + publish).
  Use it only when `Cargo.toml` already holds an unpublished version, since it
  skips the tag-match check but still calls `cargo publish`.
- **Pre-releases**: a version with a suffix (e.g. `1.0.2-alpha.1`) publishes fine
  and is excluded from default `^` version requirements — handy for test releases.
- **Feature combos**: `publish.yml` tests with `dev-dependencies` (mysql+sqlite);
  the package shipped to crates.io builds with default features (mysql +
  sqlx_mysql). The `--dry-run` step packages exactly what users get.
