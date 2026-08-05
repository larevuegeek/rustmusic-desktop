#!/usr/bin/env bash
# Retire les libs libwayland-* bundlées d'une (ou plusieurs) AppImage.
#
# Pourquoi : linuxdeploy (utilisé par `tauri build`) embarque par erreur
# libwayland-client/cursor/egl/server de la machine de build. Sur les distros
# dont le Mesa hôte est différent (SteamOS/Steam Deck, Arch, Ubuntu 26.04+),
# le libEGL de l'hôte exige les symboles de SON libwayland ; le mix
# bundlé/hôte fait échouer l'init graphique :
#   « Could not create default EGL display: EGL_BAD_PARAMETER. Aborting... »
# Ces libs sont sur l'excludelist AppImage officielle précisément pour ça :
# l'hôte doit toujours fournir son propre libwayland (présent sur toutes les
# distros modernes, même en session X11).
#
# Usage : ./scripts/fix-appimage-wayland.sh <fichier.AppImage> [autre.AppImage ...]
# Effet : chaque fichier est remplacé en place, sans les libwayland-*.
#
# ═══ QUAND LE LANCER : APRÈS le build, AVANT la signature ═══
# Le script travaille sur le .AppImage fini (donc build terminé obligatoire),
# et comme il modifie le fichier, toute signature faite avant est invalidée.
#
# Ordre exact sur la machine de build Linux :
#   1. npm run tauri build                    → produit l'AppImage
#   2. ./scripts/fix-appimage-wayland.sh \
#        src-tauri/target/release/bundle/appimage/RustMusic_*.AppImage
#   3. npm run tauri signer sign -- ...       → signature EN DERNIER,
#                                               sur le fichier final
#   4. Copier AppImage + .sig vers le site + mettre à jour l'API releases
#
# ⚠ Si le build génère déjà un .sig automatiquement (clé configurée dans
#   l'env au moment du `tauri build`), ce .sig-là devient CADUC après le
#   passage du script : c'est celui régénéré à l'étape 3 qu'il faut publier.
#
# À répéter pour la variante glibc 2.35 sur son build dédié.
set -euo pipefail

command -v appimagetool >/dev/null 2>&1 || {
    echo "❌ appimagetool introuvable dans le PATH" >&2
    exit 1
}

[ $# -ge 1 ] || { echo "Usage: $0 <fichier.AppImage> [autre.AppImage ...]" >&2; exit 1; }

fix_one() {
    local appimage
    appimage="$(readlink -f "$1")"
    [ -f "$appimage" ] || { echo "❌ Fichier introuvable : $appimage" >&2; return 1; }

    local workdir
    workdir="$(mktemp -d)"

    echo "→ Extraction de $(basename "$appimage")..."
    (cd "$workdir" && "$appimage" --appimage-extract >/dev/null)

    local libs=("$workdir"/squashfs-root/usr/lib/libwayland-*)
    if [ ! -e "${libs[0]}" ]; then
        echo "✓ Aucune libwayland bundlée dans $(basename "$appimage") — rien à faire."
        rm -rf "$workdir"
        return 0
    fi

    local lib
    for lib in "${libs[@]}"; do
        rm -v "$lib"
    done

    echo "→ Re-packaging..."
    ARCH=x86_64 appimagetool "$workdir/squashfs-root" "$appimage.new" >/dev/null
    mv "$appimage.new" "$appimage"
    chmod +x "$appimage"
    rm -rf "$workdir"

    echo "✓ $(basename "$appimage") repackée sans libwayland."
}

for f in "$@"; do
    fix_one "$f"
done

echo ""
echo "⚠  N'oublie pas de régénérer les signatures .sig (tauri signer sign)"
echo "   et de mettre à jour l'API releases du site avec les nouvelles valeurs."
