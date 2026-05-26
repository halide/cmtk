import os
import subprocess
import sys
import json
import glob


def main():
    # 1. Clean old coverage files
    for f in glob.glob("*.profraw") + ["cmtk.profdata"]:
        if os.path.exists(f):
            try:
                os.remove(f)
            except OSError:
                pass

    # 2. Run cargo test with coverage instrumented
    print("Running instrumented tests...")
    env = os.environ.copy()
    env["RUSTFLAGS"] = "-C instrument-coverage"
    env["CARGO_INCREMENTAL"] = "0"

    res = subprocess.run(["cargo", "test"], env=env)
    if res.returncode != 0:
        print("Tests failed!")
        sys.exit(res.returncode)

    # 3. Get test binaries list from cargo
    print("Finding test binaries...")
    res = subprocess.run(
        ["cargo", "test", "--no-run", "--message-format=json"],
        env=env,
        capture_output=True,
        text=True,
    )
    if res.returncode != 0:
        print("Failed to get test binaries list!")
        sys.exit(res.returncode)

    binaries = []
    for line in res.stdout.splitlines():
        if not line.strip():
            continue
        try:
            data = json.loads(line)
            if data.get("reason") == "compiler-artifact":
                profile = data.get("profile", {})
                if profile.get("test") is True:
                    filenames = data.get("filenames", [])
                    for name in filenames:
                        # Ignore dSYM directories on macOS
                        if (
                            not name.endswith(".dSYM")
                            and os.path.exists(name)
                            and os.path.isfile(name)
                        ):
                            binaries.append(name)
        except json.JSONDecodeError:
            continue

    if not binaries:
        print("No test binaries found!")
        sys.exit(1)

    # 4. Merge profraw files
    print("Merging profile data...")
    profraw_files = glob.glob("*.profraw")
    if not profraw_files:
        print("No .profraw files found! Make sure tests ran and wrote profiles.")
        sys.exit(1)

    merge_cmd = (
        ["xcrun", "llvm-profdata", "merge", "-sparse"]
        + profraw_files
        + ["-o", "cmtk.profdata"]
    )
    res = subprocess.run(merge_cmd)
    if res.returncode != 0:
        print("llvm-profdata merge failed!")
        sys.exit(res.returncode)

    # 5. Generate report
    print("Generating coverage report...")
    cov_cmd = [
        "xcrun",
        "llvm-cov",
        "report",
        "-instr-profile=cmtk.profdata",
        "-ignore-filename-regex=/.cargo/|/rustc/|tests/",
    ]
    for binary in binaries:
        cov_cmd.extend(["-object", binary])

    # We rely on -ignore-filename-regex to limit coverage reporting to src/ files.

    res = subprocess.run(cov_cmd)
    if res.returncode != 0:
        print("llvm-cov report failed!")
        sys.exit(res.returncode)


if __name__ == "__main__":
    main()
