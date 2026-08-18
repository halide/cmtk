#!/usr/bin/env python3
import sys
import os
import tempfile
import subprocess
import re


def main():
    if len(sys.argv) < 2:
        print("Usage: python tools/update_mirror.py <version>")
        sys.exit(1)

    version = sys.argv[1]

    # Locally, developers authenticate over SSH using their own key. In CI,
    # there's no SSH key for the bot, so a fine-grained PAT (scoped to just
    # this mirror repo) is used over HTTPS instead.
    token = os.environ.get("CMTK_MIRROR_TOKEN")
    if token:
        repo_url = (
            f"https://x-access-token:{token}@github.com/halide/cmtk-pre-commit.git"
        )
    else:
        repo_url = "git@github.com:halide/cmtk-pre-commit.git"

    print(f"Updating pre-commit mirror to version {version}...")

    # Create a temporary directory
    with tempfile.TemporaryDirectory() as tmpdir:
        # Clone the mirror repo
        print("Cloning mirror repository into temp dir...")
        res = subprocess.run(
            ["git", "clone", repo_url, "repo"],
            cwd=tmpdir,
            capture_output=True,
            text=True,
        )
        if res.returncode != 0:
            print("Error cloning repository:")
            print(res.stderr)
            sys.exit(1)

        repo_dir = os.path.join(tmpdir, "repo")

        if token:
            # The CI bot has no git identity configured; local developers
            # already have one and should keep authoring mirror commits
            # under their own name.
            subprocess.run(
                ["git", "config", "user.name", "cmtk-release-bot"],
                cwd=repo_dir,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.email", "actions@users.noreply.github.com"],
                cwd=repo_dir,
                check=True,
            )

        # Write .pre-commit-hooks.yaml
        hooks_content = """- id: cmtk
  name: cmtk
  description: Format CMake files in place with cmtk.
  entry: cmtk format --discover=git -i
  language: python
  files: (^|/)CMakeLists\\.txt$|\\.cmake(\\.in)?$
  require_serial: true

- id: cmtk-check
  name: cmtk (check)
  description: Check that CMake files are formatted with cmtk, without modifying them.
  entry: cmtk format --discover=git --check
  language: python
  files: (^|/)CMakeLists\\.txt$|\\.cmake(\\.in)?$
  require_serial: true
"""
        hooks_path = os.path.join(repo_dir, ".pre-commit-hooks.yaml")
        with open(hooks_path, "w", encoding="utf-8") as f:
            f.write(hooks_content)

        # Write dummy pyproject.toml with cmtk dependency
        pyproject_content = f"""[project]
name = "cmtk-pre-commit"
version = "0.0.0"
dependencies = [
    "cmtk=={version}",
]
"""
        pyproject_path = os.path.join(repo_dir, "pyproject.toml")
        with open(pyproject_path, "w", encoding="utf-8") as f:
            f.write(pyproject_content)

        # Write README.md if it doesn't exist
        readme_path = os.path.join(repo_dir, "README.md")
        if not os.path.exists(readme_path):
            readme_content = f"""# cmtk-pre-commit

pre-commit hooks for cmtk (CMake formatter and static analyzer).

This repository is a pre-compiled mirror distribution. It allows using `cmtk` in `pre-commit` without compiling it from source.

## Usage

Add this to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/halide/cmtk-pre-commit
    rev: v{version}
    hooks:
      - id: cmtk
```
"""
            with open(readme_path, "w", encoding="utf-8") as f:
                f.write(readme_content)
        else:
            # Update the README to reference the new version
            with open(readme_path, "r", encoding="utf-8") as f:
                readme_content = f.read()
            # Replace the old version in instructions
            readme_content = re.sub(
                r"rev: v\d+\.\d+\.\d+", f"rev: v{version}", readme_content
            )
            with open(readme_path, "w", encoding="utf-8") as f:
                f.write(readme_content)

        # Commit, tag, and push
        subprocess.run(
            ["git", "add", ".pre-commit-hooks.yaml", "pyproject.toml", "README.md"],
            cwd=repo_dir,
        )

        # Check if there are changes to commit
        status_res = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=repo_dir,
            capture_output=True,
            text=True,
        )
        if not status_res.stdout.strip():
            print("No changes to mirror repository.")
            return

        subprocess.run(["git", "commit", "-m", f"Release v{version}"], cwd=repo_dir)
        subprocess.run(["git", "tag", "-f", f"v{version}"], cwd=repo_dir)

        print("Pushing commits and tags to mirror repository...")
        # Get active branch name
        branch_res = subprocess.run(
            ["git", "branch", "--show-current"],
            cwd=repo_dir,
            capture_output=True,
            text=True,
        )
        branch = branch_res.stdout.strip() or "main"

        push_res = subprocess.run(
            ["git", "push", "origin", branch],
            cwd=repo_dir,
            capture_output=True,
            text=True,
        )
        # Push tag (forcefully)
        subprocess.run(
            ["git", "push", "-f", "origin", f"v{version}"],
            cwd=repo_dir,
            capture_output=True,
            text=True,
        )
        if push_res.returncode != 0:
            print("Error pushing to mirror repository:")
            print(push_res.stderr)
            sys.exit(1)

        print("Successfully updated and pushed mirror repository!")


if __name__ == "__main__":
    main()
