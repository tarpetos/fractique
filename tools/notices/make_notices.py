import json
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parents[2]
metadata = json.loads(
    subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
)
members = set(metadata["workspace_members"])
crates = {}
for package in metadata["packages"]:
    if package["id"] in members:
        continue
    license = package.get("license") or "see the crate"
    crates[(package["name"].lower(), package["version"])] = (package["name"], package["version"], license)
table = [f"| {name} | {version} | {license} |" for _, (name, version, license) in sorted(crates.items())]

notices = root / "THIRD_PARTY_NOTICES.md"
lines = notices.read_text(encoding="utf-8").split("\n")
start = next(index for index, line in enumerate(lines) if line.startswith("| --- |")) + 1
end = start
while end < len(lines) and lines[end].startswith("|"):
    end += 1
notices.write_text("\n".join(lines[:start] + table + lines[end:]), encoding="utf-8")
print(f"{len(table)} crates")
