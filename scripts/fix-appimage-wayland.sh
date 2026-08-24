#!/usr/bin/env bash
# Rend à l'hôte les bibliothèques que l'AppImage n'aurait pas dû embarquer.
#
# ═══ Deux familles, une même cause ═══
#
# linuxdeploy (utilisé par `tauri build`) embarque des bibliothèques de la
# machine de build qui doivent impérativement venir de l'hôte. Le mélange des
# deux fait tomber l'application au démarrage — fenêtre blanche, puis rien.
#
# 1. libwayland-*
#    Sur les distros dont le Mesa hôte diffère (SteamOS, Arch, Ubuntu 26.04+),
#    le libEGL de l'hôte exige les symboles de SON libwayland :
#      « Could not create default EGL display: EGL_BAD_PARAMETER. Aborting... »
#    Ces libs sont sur l'excludelist AppImage officielle précisément pour ça.
#
# 2. WebKitGTK  ← ajouté après le rapport de crash de la 0.2.0
#    Un vidage mémoire sur Ubuntu 26.04 (webkit2gtk 2.52.3, mesa 26.0.8) montre
#    le `WebKitWebProcess` **embarqué** — chemin `/tmp/.mount_*` — qui se
#    termine par SIGABRT au lancement. Un processus WebKit embarqué qui
#    rencontre les bibliothèques graphiques de l'hôte est la même erreur que
#    ci-dessus, à un étage au-dessus.
#
#    ⚠ Ce retrait a un prix, et il faut le savoir : l'AppImage exige alors que
#    l'hôte fournisse `libwebkit2gtk-4.1`. C'est déjà ce qu'exige le paquet
#    .deb, qui lui fonctionne — mais une distro sans ce paquet ne lancera plus
#    l'AppImage. Le compromis est assumé : mieux vaut une dépendance annoncée
#    qu'un démarrage qui échoue sans message.
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

    # Motifs à retirer. `nullglob` évite qu'un motif sans correspondance soit
    # traité comme un nom de fichier littéral.
    shopt -s nullglob
    local victims=(
        "$workdir"/squashfs-root/usr/lib/libwayland-*
        "$workdir"/squashfs-root/usr/lib/libwebkit2gtk-*
        "$workdir"/squashfs-root/usr/lib/libjavascriptcoregtk-*
        "$workdir"/squashfs-root/usr/lib/x86_64-linux-gnu/libwebkit2gtk-*
        "$workdir"/squashfs-root/usr/lib/x86_64-linux-gnu/libjavascriptcoregtk-*
    )
    # Le dossier des processus auxiliaires de WebKit : c'est lui qui contient
    # le `WebKitWebProcess` du vidage mémoire.
    local webkit_dirs=(
        "$workdir"/squashfs-root/usr/lib/x86_64-linux-gnu/webkit2gtk-*
        "$workdir"/squashfs-root/usr/lib/webkit2gtk-*
    )
    shopt -u nullglob

    if [ ${#victims[@]} -eq 0 ] && [ ${#webkit_dirs[@]} -eq 0 ]; then
        echo "✓ Rien d'indésirable dans $(basename "$appimage") — rien à faire."
        rm -rf "$workdir"
        return 0
    fi

    local lib
    for lib in "${victims[@]}"; do
        rm -v "$lib"
    done
    local dir
    for dir in "${webkit_dirs[@]}"; do
        rm -rv "$dir"
    done

    echo "→ Re-packaging..."
    ARCH=x86_64 appimagetool "$workdir/squashfs-root" "$appimage.new" >/dev/null
    mv "$appimage.new" "$appimage"
    chmod +x "$appimage"
    rm -rf "$workdir"

    echo "✓ $(basename "$appimage") repackée sans libwayland ni WebKitGTK."
}

for f in "$@"; do
    fix_one "$f"
done

echo ""
echo "⚠  N'oublie pas de régénérer les signatures .sig (tauri signer sign)"
echo "   et de mettre à jour l'API releases du site avec les nouvelles valeurs."
