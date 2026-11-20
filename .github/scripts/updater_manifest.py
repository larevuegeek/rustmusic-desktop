"""Manifeste des mises à jour (format du module de mise à jour de la 0.4 et de karbon-updater).

Usage : python updater_manifest.py <dossier des paquets> <étiquette> <dépôt owner/nom>

Lit les paquets et leurs signatures `.sig` du dossier, écrit `latest.json` (version finale)
ou `latest-beta.json` (étiquette avec un suffixe, `v0.5.0-beta.1`) dans le même dossier.
Chaque plateforme est publiée sous `<os>-<arch>-<format>` et sous `<os>-<arch>` : c'est
l'ordre dans lequel la 0.4 et la 0.5 cherchent leur paquet.
"""
import datetime
import json
import pathlib
import re
import sys

# (motif du nom de fichier, clé précise, clé générale ou None)
PLATFORMS = [
    (r"_x64-setup\.exe$", "windows-x86_64-nsis", "windows-x86_64"),
    (r"_aarch64\.app\.tar\.gz$", "darwin-aarch64-app", "darwin-aarch64"),
    (r"_x64\.app\.tar\.gz$", "darwin-x86_64-app", "darwin-x86_64"),
    (r"\.AppImage$", "linux-x86_64-appimage", "linux-x86_64"),
    (r"\.deb$", "linux-x86_64-deb", None),
]


def release_notes(version: str) -> str:
    """Section du CHANGELOG pour cette version (sans suffixe de pré-version), sinon un renvoi."""
    base = version.split("-")[0]
    changelog = pathlib.Path("CHANGELOG.md")
    if changelog.exists():
        text = changelog.read_text(encoding="utf-8")
        match = re.search(
            rf"^## \[{re.escape(base)}\][^\n]*\n(.*?)(?=^## \[|\Z)", text, re.S | re.M
        )
        if match:
            return match.group(1).strip()
    return f"RustMusic {version}"


def main() -> None:
    folder, tag, repo = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    version = tag.removeprefix("v")
    platforms = {}
    for path in sorted(folder.iterdir()):
        for pattern, key, general in PLATFORMS:
            if not re.search(pattern, path.name):
                continue
            signature = path.with_name(path.name + ".sig")
            if not signature.exists():
                sys.exit(f"signature manquante : {signature.name}")
            asset = {
                "url": f"https://github.com/{repo}/releases/download/{tag}/{path.name}",
                "signature": signature.read_text(encoding="utf-8").strip(),
            }
            platforms[key] = asset
            if general:
                platforms.setdefault(general, asset)
    missing = [key for _, key, _ in PLATFORMS if key not in platforms]
    if missing:
        sys.exit(f"paquets manquants : {', '.join(missing)}")
    manifest = {
        "version": version,
        "notes": release_notes(version),
        "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }
    name = "latest-beta.json" if "-" in version else "latest.json"
    (folder / name).write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{name} : {len(platforms)} entrées")


if __name__ == "__main__":
    main()
