# Release Process

This document describes how to release a new version of `cmtk` to PyPI and update the precompiled `cmtk-pre-commit` hook repository.

---

## Overview of the Automation

We use `tbump` locally to coordinate version bumping across files, git tagging, and repository syncing.
Once a version tag (e.g. `v0.1.1`) is pushed to GitHub:
1. **GitHub Actions** compiles the Rust executable and builds Python wheels for all target platforms.
2. **PyPI Trusted Publishing (OIDC)** publishes the built wheels to PyPI.
3. The local `tbump` process automatically updates and pushes tags to the **`cmtk-pre-commit`** mirror repository so users can consume precompiled wheels.

---

## Standard Release Workflow (No Branch Protection)

If you have permission to push directly to the `main` branch, you can perform a release in one step:

1. **Run `tbump`** with the new version (e.g., `0.1.1`):
   ```bash
   uv run tbump 0.1.1
   ```
2. **Confirm the prompt**. `tbump` will:
   - Check that your git repository is clean.
   - Bump the version in `Cargo.toml` and `pyproject.toml`.
   - Run a `before_commit` hook: `cargo check` (to synchronize `Cargo.lock`) and stage `Cargo.lock`.
   - Commit the changes locally as `"Bump to 0.1.1"`.
   - Tag the commit as `v0.1.1`.
   - Push the branch and tag to GitHub.
   - Run an `after_push` hook: clones `cmtk-pre-commit` locally, updates `.pre-commit-hooks.yaml` to point to `cmtk==0.1.1`, commits/tags/pushes to the mirror repository.

---

## Pull Request / Protected Branch Workflow (Recommended)

If the `main` branch is protected and you cannot push directly:

1. **Create a release branch**:
   ```bash
   git checkout -b release/v0.1.1
   ```
2. **Run `tbump` with `--no-push`**:
   ```bash
   uv run tbump 0.1.1 --no-push
   ```
   *This updates the version files, runs the cargo check hook, commits, and tags the commit locally.*
3. **Push the branch to GitHub**:
   ```bash
   git push origin release/v0.1.1
   ```
4. **Open a Pull Request**:
   - Open a PR from `release/v0.1.1` to `main` on GitHub.
   - Wait for tests to pass and merge the PR.
5. **Push the Release Tag**:
   Once the PR is merged, pull the latest changes back to your local `main` branch, and push the local tag you created in step 2:
   ```bash
   git checkout main
   git pull origin main
   git push origin v0.1.1
   ```
   *Pushing the tag does not trigger branch protection and will kick off the PyPI release workflow.*
6. **Update the Pre-Commit Mirror**:
   Run the mirror update script manually to sync the pre-commit repository:
   ```bash
   python3 tools/update_mirror.py 0.1.1
   ```

---

## Infrastructure Configuration

### GitHub Actions
The publication pipeline is defined in `.github/workflows/release.yml`. It builds native wheels for the following targets:
* **Linux**: `x86_64` and `aarch64` (uses `manylinux: auto` containers)
* **macOS**: `x86_64-apple-darwin` and `aarch64-apple-darwin` (dedicated fast wheels)
* **Windows**: `x64`

### PyPI Trusted Publishing
This project uses OIDC to authenticate with PyPI. The publisher settings are:
* **PyPI Project**: `cmtk`
* **GitHub Repository Owner**: `halide`
* **Repository Name**: `cmtk`
* **Workflow Name**: `release.yml`
