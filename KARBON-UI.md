# RustMusic sur karbon-ui (clone de travail, rien n'est commité)

## Branche `karbon-ui` (0.5.0), dossier `rustmusic-0.5`

Le 2026-11-14, le travail du clone (base 0.4.3, `0889e27`) a été reporté sur la branche
`karbon-ui` partie de la 0.4.4 (`aba0351`) : fusion à trois voies des 70 fichiers Rust
adaptés, sans conflit ; `src/app/`, `src/ui/` et ces notes copiés ; `handler.rs` retiré.
Le moteur de la 0.4.4 est donc là (export auto des playlists, portrait d'artiste,
relecture des tags, migrations 202611130001-2). `set_native_window_buttons` (#32, fenêtre
Tauri) est retiré : coins arrondis et boutons natifs de macOS à obtenir de karbon-ui.
Les textes de la 0.4.4 sont déjà dans les fichiers de langue partagés.

Interface de la 0.4.4 encore à traduire dans `src/ui/` :
- export automatique : interrupteur « Export auto » du menu d'une playlist, section des
  Réglages → Stockage (dossier, format, chemins relatifs, une ligne par playlist, Exporter
  maintenant) ; service `autoExport.service.ts` (relance après un changement, un scan, au
  démarrage) ;
- fenêtre du portrait d'un artiste (`ArtistPortraitPopin.svelte`) et « Changer de
  portrait… » dans le menu d'un artiste ;
- « Tout sélectionner / Tout désélectionner » (`SelectionToggle.svelte`, `provideAll` de
  l'onglet Morceaux) ;
- page d'un compositeur : « Interprété par », icône sur « Sa page d'artiste », pastille +n ;
- atelier de tags : champs qui reviennent à la ligne, panneau élargissable ;
- pages de détail sur toute la largeur (#25) ; menus toujours dans la fenêtre (#28, à
  vérifier côté karbon-ui).

Le frontend Svelte (`src/`, `static/`, `package.json`, Vite, TypeScript) est retiré le
2026-11-14 ; les traductions sont dans `src/ui/locales/`. Les composants Svelte qui
servent encore de référence (interface de la 0.4.4 ci-dessous, télécommande d'ampli) sont
dans `../rustmusic` ou dans l'historique (`git show aba0351:src/…`).

Tauri, ses plugins, wry, WebView2 et WebKitGTK sont retirés de `Cargo.toml`. L'interface
est native (karbon-ui, dépendance par chemin `../karbon-ui`) ; le moteur (audio, base,
réseau, podcasts, radios, DLNA) est inchangé.

Le même jour, le projet Cargo est remonté de `src-tauri/` à la racine (`src/`, `SQL/`,
`tests/`, `keys/`, `icons/`, `target/`) ; `Info.plist` est dans `packaging/macos/`.
Retirés : `tauri.conf.json`, `tauri.macos.conf.json`, `capabilities/`, icônes Android et
iOS, `scripts/build-macos.sh` et `scripts/fix-appimage-wayland.sh` (`npx tauri build`).
La recette Flatpak construit par `cargo build --release` (sans Node ni appindicator) ;
karbon-ui y reste à fournir (dépendance par chemin).

### Ce que Tauri fournissait, à porter dans karbon-ui ou le clone

- **Mise à jour automatique** (faite le 2026-11-14) : crate `karbon-updater` du dépôt
  karbon-ui (`crates/updater`, protocole, signatures et arguments d'installateur du
  module de Tauri), branchée dans `src/ui/pages/updater.rs` : vérification 5 s après le
  lancement (nouvelles tentatives à 1 et 10 min), carte et avancement, « Vérifier » des
  réglages et d'À propos, sortie 1,5 s après l'installation. Adresse
  `https://rustmusic.dev/api/releases/{{target}}-{{arch}}/{{current_version}}` (le
  serveur range les paquets sous `windows-x86_64`…), clé publique `keys/rustmusic.key.pub`
  (la clé privée `keys/rustmusic.key` n'est pas versionnée). Essayé contre le vrai
  serveur (0.4.3 → 0.4.4 : annonce, téléchargement, signature vérifiée) ; l'installation
  elle-même attend un vrai installateur 0.5. En développement, `RUSTMUSIC_UPDATE_FROM=0.4.3`
  simule une ancienne version (l'installation est refusée depuis `target`).
  **Serveur** : `/api/releases/windows/0.4.3`, l'adresse qu'appellent les 0.4.x (Tauri
  ne met que le système dans `{{target}}`), répond 204 ; seule `windows-x86_64/…` annonce
  la 0.4.4. À vérifier côté serveur pour que les 0.4.x reçoivent la 0.5.
- **Paquets de mise à jour** : Tauri produisait et signait les paquets
  (`createUpdaterArtifacts`, `tauri signer sign`). Il faut désormais les produire :
  installateur NSIS Windows qui accepte `/P /UPDATE /R /ARGS …` (ceux que lancent les 0.4.x
  et `karbon-updater`), `.app.tar.gz` macOS, `.deb` (et AppImage) Linux, signés par
  `tauri signer sign -f keys/rustmusic.key` ou minisign.
- **Installateurs** (`tauri build`) : NSIS ou MSI sous Windows, `.app` et `.dmg` sous
  macOS (signature Developer ID et notarisation par `keys/apple.local`, voir
  `git show aba0351:scripts/build-macos.sh`), `.deb`, `.rpm` et AppImage sous Linux.
  Métadonnées : nom `RustMusic`, identifiant `com.larevuegeek.rustmusic`, catégorie
  Music, macOS 10.15 au minimum, descriptions courte et longue dans
  `git show aba0351:src-tauri/tauri.conf.json`.
- **Icônes** (fait sous Windows le 2026-11-14) : icône et informations de version de
  l'exécutable (`build.rs`, crate `embed-resource`, `icons/icon.ico`), icône de la
  fenêtre (`WindowOptions::icon`, `icons/128x128@2x.png`), icône de la zone de
  notification (`src/ui/tray.rs`, `icons/tray-32x32.png` : « Afficher RustMusic »,
  « Quitter », clic qui ramène la fenêtre ; la croix et celle du mini-lecteur masquent
  si « Réduire dans la barre système »). Menu au thème du système sous Windows. macOS
  (barre des menus, icône `template`, Dock hors paquet avec `icons/icon-macos.png`) et
  Linux (`StatusNotifierItem`, `_NET_WM_ICON` sous X11) faits dans karbon-ui mais
  compilés seulement : à essayer sur un Mac et sous KDE ou GNOME + AppIndicator.
- Le reste est listé plus bas (« Pas encore traduit », « Manques de karbon-ui ») :
  dialogues de fichiers restants, notifications, démarrage
  automatique, instance unique, raccourcis globaux, contrôles média du système, coins
  arrondis et boutons natifs de macOS (fenêtre `titleBarStyle: Overlay`, feux à 20 × 33).

## Ce qui a été fait

- `src/app/` : remplace `tauri::AppHandle` / `State` / `async_runtime` / `path`. La poignée
  garde l'état géré et transmet les `emit` du moteur à l'interface par un canal.
- `src/main.rs` crée le runtime tokio ; `lib.rs::run` reprend l'initialisation du moteur
  puis ouvre la fenêtre (`src/ui/`).
- `src/ui/` : interface traduite des composants Svelte (templates `.ui` + Tailwind compilé).
  - Accueil complet : en-tête, hero « reprendre », « Ensuite », mix du jour tirés de la
    bibliothèque, playlists, genres, ajouts récents, récemment joués.
  - Barre latérale, barre de titre, recherche avec suggestions, lecteur et ligne d'état,
    menus (profil, sortie audio, bibliothèques, sources, minuteur, réseau, options), file
    d'attente.
  - Pages : morceaux (liste virtualisée), albums, artistes, genres, années, dossiers, album,
    artiste, genre, décennie, compositeur, dossier, playlists, playlist, titres likés,
    récemment joués, mix, podcasts, podcast (épisodes), radios, recherche, statistiques,
    réglages (interrupteurs Général, Apparence, Audio, Réseau).
  - Lecture : lecture/pause, précédent/suivant, avance/recul, position, volume, sourdine,
    aléatoire, répétition, file, enchaînement en fin de morceau, minuteur de veille, radios.
- 69 des 287 commandes du moteur sont appelées par l'interface.

## Tester sans toucher aux vraies données

`RUSTMUSIC_DATA_DIR` redirige tous les dossiers de données (ajout de ce clone) ;
`HOME` / `USERPROFILE` redirigent le fichier de réglages audio.

```powershell
cargo build --release          # ou cargo run [-- <options>]
$env:RUSTMUSIC_DATA_DIR = "<parent de la copie de com.larevuegeek.rustmusic>"
.\target\release\rustmusic.exe
# HOME / USERPROFILE : par Start-Process … -Environment @{…}, pas dans le terminal de
#   compilation (Cargo et rustup s'en servent).
# Aperçu hors écran : --export x.png [--size 1280x900] [--scale 1.5] [--renderer=software]
#   [--page tracks|albums|artists|genres|years|folders|playlists|liked|recent|mixes|stats|
#           settings|appearance|settings-audio|settings-network|settings-online|
#           settings-remote|settings-storage|settings-shortcuts|settings-guide|
#           settings-about|podcasts|subscriptions|downloads|podcast:<id>|radios|
#           search:<texte>|album:<id>|artist:<id>|genre:<nom>|decade:<1990>|
#           composer:<nom>|playlist:<id>|track:<id>|tags:<album>|import|feedback]
#   [--view list] [--offset <px>] [--folder <chemin>] [--popup <id>]
#   [--track-menu <ligne>[+]] [--album-menu <id album>] [--open lyrics]
#   [--open profile|output|library|source|queue|sleep|network|mix|open|display|new-playlist|whats-new[:n]|smart[:id]|new-mix|profiles|profile-new|profile:<id>|reset|remove-server:<id>|add-server|server-folders:<id>|pod-preview:<id>|device:<n>|tags-edit:<chemin>|cover-search:<album>|new-library|rename-library|select:<rangs>|support|update:<état>|columns|lib-folders|artist-report|clean|history|rename|batch-tags|tag-source[:2]|tracks-next:<n>|sb-pl:<id>|sb-pl-at:<id>|sb-pin:album|artist:<id>|sb-zone:album|artist|lib-options|organize]
#   [--bench <images> [--click x,y]]   (banc d'essai de karbon-ui : repeinture, défilement,
#           reconstruction, survol, animation à 60 Hz simulés après un clic en x,y, molette)
#   [--search <texte>]
# Fenêtre : --page <page> l'ouvre sur cette page ; --profile écrit une ligne
#   « Slow frame : {…} » par image de plus de 10 ms et, à la fermeture, la cadence
#   (« intervals », « late_frames ») : lancer avec la sortie standard redirigée.
```

Base copiée : six migrations ont été enregistrées en CRLF dans la vraie base, les autres
en LF ; ce clone a `core.autocrlf=false` et ces six fichiers en CRLF pour correspondre.
Les icônes sont compilées par `icons_dir!` de karbon-ui (`src/app/resources/icons.rs`) depuis
`ui/assets/icons/<jeu>/<nom>.svg` ; `ui/assets/icons-thick/` porte les variantes au trait
`.sb-icone`. Ajouter une icône = déposer son SVG (plus de script Python ni de `node_modules`).
Les drapeaux (`circle-flags`) ont perdu leur `<mask>` circulaire, que karbon-ui ne prend pas.

## Organisation (depuis le 2026-11-15)

Le code et le thème sont séparés, à la manière de Symfony (détail dans `CONTRIBUTING.md`) :
`ui/` à la racine pour le thème (`templates/`, `assets/`, `translations/`), et dans `src/`
`controller/`, `models/` (un fichier par type), `mapper/`, `component/`, `query/` et `app/`.
Les notes ci-dessous, écrites avant, citent l'ancienne arborescence ; correspondance :

| Avant | Maintenant |
| --- | --- |
| `src/ui/ui/<page>/*.ui` | `ui/templates/<domaine>/*.ui` |
| `src/ui/assets/`, `src/ui/locales/` | `ui/assets/`, `ui/translations/` |
| `src/ui/pages/<page>/` (types) | `src/models/<page>/<type>.rs` |
| `src/ui/pages/<page>/` (actions, chargement) | `src/controller/<page>` |
| `src/ui/pages/<page>/` (ce qu'affiche le gabarit) | `src/mapper/<page>` |
| `src/ui/app.rs`, `bridge.rs`, `tray.rs`, `assets.rs`… | `src/app/` (`music_app/`, `platform/bridge.rs`, `platform/tray.rs`, `resources/assets.rs`…) |
| `src/ui/kit.rs`, `components.rs`, `settings_kit.rs`, `smart_kit.rs` | `src/component/<domaine>/` (`shared/` pour les composants communs à tous les écrans) |
| `src/ui/viewmodel.rs` | `src/models/home/`, `src/mapper/home/` |
| `src/mapper/*View` (lignes lues en base) | `src/query/<domaine>/*_row.rs` (`…Row`) |
| `pages/srvform`, `tagedit`, `ctxmenu`, `collmenu`, `plform`, `libform`, `libfolders`, `coversearch`, `artistcheck`, `columns_popin`, `typed`, `whatsnew` | `server_form`, `tag_editor`, `context_menu`, `collection_menu`, `playlist_form`, `library_form`, `library_folders`, `cover_search`, `artist_check`, `columns`, `typed_confirm`, `whats_new` |

## Traduction à l'identique (en cours)

Une page par dossier : ses types dans `src/models/<page>/`, ses actions dans
`src/controller/<page>`, ce qu'affiche le gabarit dans `src/mapper/<page>`, et ses gabarits
dans `ui/templates/<domaine>/`. Les pages traduites remplacent la page générique (`PageKind`)
dans `load_page` et `layout/Layout.ui`.

Faites : Morceaux, Albums, page d'album, Artistes (et compositeurs), page d'artiste,
Genres, page de genre, Années, page de décennie ou d'année, page de compositeur, Dossiers,
Mix, Playlists, page de playlist, Titres aimés, Récents, Recherche, Radios, Podcasts
(4 pages), Statistiques, Réglages (les 10 pages : `pages/settings/`, briques communes dans
`model.rs`, composants dans `settings_kit.rs`), toasts (`pages/toast.rs`), fenêtre de
confirmation (`pages/popin.rs`, `Popin.svelte` et `Dialog.svelte`), Avis, Import (accueil
d'import), fiche d'un morceau (`pages/track/`), atelier de tags (`pages/tags/`, route
`tags:<album>` ; tableau, panneau, annulation, écriture ; sans ses fenêtres ni
l'inventaire), volet de la file d'attente (`pages/queue.rs`, sans glisser-déposer), panneau des paroles
(`pages/lyrics.rs`), panneau des lots (`pages/batch.rs`), menus contextuels d'un morceau,
d'une collection, d'une radio, d'un podcast et d'une playlist (clic droit ou « … » ;
`pages/ctxmenu.rs`, `collmenu.rs`, `radios/menu.rs`, `podcasts/menu.rs`,
`playlists/menu.rs`), formulaire d'une playlist (`pages/plform.rs`, création depuis la
barre latérale, la page Playlists ou le menu d'un morceau, modification depuis le menu ou
la page d'une playlist), nouveautés de la version (`pages/whatsnew.rs`, mégaphone et
ouverture au lancement ; `ui-local.json` tient lieu de `localStorage`, `local.rs`).
Playlist auto et mix (`pages/smart/`, recettes, règles en arbre sur trois niveaux, compte
vivant ; composants `smart_kit.rs`), ouverte depuis la barre latérale, la page Playlists,
la page et le menu d'une playlist auto, et la page Mix (créer, modifier).
Écran des profils « Qui écoute ? » (`pages/profiles.rs` : choisir, créer, modifier,
supprimer par mot à taper ; menu du profil et avatar du pied de la barre latérale).
Confirmation au mot tapé (`pages/typed.rs`, `TypedConfirmDialog` : réinitialiser
l'application puis la relancer, retirer un serveur).
Ajout d'un serveur et ses dossiers (`pages/srvform/` : OpenSubsonic, Jellyfin, DLNA avec
recherche sur le réseau et sélecteur de dossiers en arborescence).
Aperçu d'une émission (`pages/podcasts/preview.rs`, menu « Aperçu »), fiche d'une sortie
audio (`pages/settings/device_details.rs`), éditeur de tags d'un fichier avec recherche
Deezer / MusicBrainz et comparaison (`pages/tagedit/`, fiche d'un morceau et menu d'un
morceau), recherche de pochette chez Deezer (`pages/coversearch.rs`, menu d'un album).
Créer, renommer, supprimer une bibliothèque (`pages/libform.rs`). Mode sélection des
listes de morceaux et sa barre (`pages/selection.rs` : lire, file, playlist, atelier sur la
sélection), sur toutes les pages à tableau et à pistes d'album.
Cartes de soutien et de mise à jour (`pages/banners.rs` ; le greffon `updater` n'étant pas
porté, la seconde reste au repos).
Choix des colonnes du tableau (`pages/columns_popin.rs`), dossiers d'une bibliothèque
(`pages/libfolders.rs`), vérification des artistes en direct et son bilan
(`pages/artistcheck.rs`, événements `artist-check-*`).
Atelier de tags : nettoyer (`pages/tags/clean.rs`), historique des lots
(`history.rs`), renommer ou ranger les fichiers (`rename.rs`) ; tags de plusieurs fichiers
(`pages/tagedit/batch.rs`, « Tags » de la barre de sélection) ; compléter en ligne
(`pages/tags/source.rs`, `source_view.rs` : Deezer ou MusicBrainz, appariement, champs,
pochette en attente écrite avec les autres modifications).
Fenêtre générique : tailles `md` à `xxl`, modes sans en-tête et `flush`, corps défilant.

Retours du 2026-11-07, corrigés :
- **Morceaux** : chargement au fil du défilement (`onscroll` ajouté à karbon-ui), pages de
  50, fenêtre de 150 lignes qui glisse sans à-coup (ancrage du défilement de karbon-ui),
  lettre A–Z ouverte sur sa ligne (`scroll-initial-target`). Le plantage au bouton
  « suivant » venait de la limite de 10 000 éléments de karbon-ui (66 par ligne, dont 36
  pour les étoiles) : étoiles allégées à 46 éléments par ligne.
- **Artistes, albums** : tranches chargées au défilement au lieu de « Afficher plus ».
- **Padding des listes** : karbon-ui rognait le contenu d'une zone défilante à sa boîte de
  contenu ; corrigé (boîte de padding, comme CSS).
- **Clignotement au défilement** : le survol suivait chaque cran de molette ; limité à un
  rafraîchissement par 100 ms dans karbon-ui, comme Chromium.
- **Égaliseur** : barres de NowPlayingCard en `animate-[eq-bar-N_…]` avec les keyframes de
  hauteur d'app.css (karbon-ui anime `width`/`height`) ; lignes en `.mini-eq`.
- **Fenêtres** : entrée et sortie de Popin.svelte (fondu 150 ms du fond, échelle 0,96 sur
  180 ms du dialogue) par `animation` + `data-exit` ; `bare` implique `flush` comme
  popin.store.ts (nouveautés, aperçu d'un podcast).
- **Retour des réglages** : revient à la page d'avant les réglages (`settingsReturnUrl`),
  quelle que soit la section ouverte.
- **Nouveautés** : structure de la source (piste de toutes les diapos translatée,
  pastilles qui défilent en `w-max`, satellites décalés, `.pause`), illustration bord à
  bord (la marge de 24 px était l'écart principal) ; couleurs par `color_mix` exact ;
  « Essayer » de la diapo « Organiser » ouvre le mode Organiser. Reste : grain, `inert`.
  Glissade (2026-11-08) : les satellites et pastilles de la diapo amenée par la piste
  n'étaient ni animés ni ombrés (découpe de layout vide hors de la piste), corrigé dans
  karbon-ui ; mesure : `--open whats-new --bench 45 --click 967,700` en 1440 × 900
  (« Suivant »), phase `animation`.

Retours du 2026-11-07 (suite) :
- **Saccades au défilement pendant la lecture** : l'horloge du lecteur (250 ms) reconstruit
  toute la vue ; une reconstruction de la page Morceaux coûtait 57 ms (vue 19, styles 7,
  layout 29). karbon-ui reprend maintenant les sous-arbres mémorisés (`memo={clé}`, comme
  `Html.Lazy` d'Elm) : vue, mesures et placement des lignes inchangées sont repris ; une
  ligne de `TrackRows.ui` est mémorisée sur ses données, la sélection et sa pochette.
  Reconstruction : 23 ms (vue 3,9, styles 9, layout 8,5). Reste à sauter aussi les styles
  des sous-arbres repris.
- Barre latérale : sections dans l'ordre choisi, mode « Organiser » (glisser-déposer et
  flèches), épinglés (albums, artistes, carte d'accueil, en lecture) avec `LibraryItemMenu`
  et `PinZoneMenu`, podcasts favoris, menu d'une playlist au « … » et au clic droit
  (« Garder hors ligne » compris), `LibraryZoneMenu`, pastille du « … » des playlists
  (`pages/sidebar/`).
- Mini-lecteur et micro-lecteur (`pages/mini/`, `ui/mini/` : `MiniPlayer`, `MicroPlayer`,
  `MiniVolume`, store `miniPlayer.store.ts`) : même fenêtre qui change de mode par les
  commandes de fenêtre de karbon-ui (`WindowChange`, `Application::window_changed`), bouton
  de la barre de titre ; File (en cours, suite, vider, retirer), Paroles synchronisées,
  volume en bulle, épingle, micro carré redimensionnable par sa poignée
  (`data-resize-region`), taille et position du micro retenues ; géométrie normale
  mémorisée (`window_geometry::report`). Aperçu : `--open mini-player[:queue|:lyrics|:volume]`,
  `--open micro-player` (avec `--size 380x246`, `380x546`, `220x220`). Vérifié par captures
  seulement, pas encore dans une vraie fenêtre.
- États ARIA (`aria-pressed`, `aria-checked` avec `role`, `aria-selected`,
  `aria-current={x.then_some("page")}`), `data-no-drag` dans une `data-drag-region`, et
  `onwheel={|e: WheelEvent| Option<Msg>}` (`Some` consomme, `None` laisse défiler) ajoutés
  à karbon-ui et posés dans 49 gabarits du clone (choix segmentés, bascules, onglets,
  barre latérale, mini et micro-lecteur : volume à la molette).
- **Fluidité** (karbon-ui) : défilement doux à la molette (100 px par cran, 150 ms) ;
  résolution partielle des styles au survol ; sous-arbres mémorisés qui sautent aussi la
  cascade ; layout incrémental de tout conteneur inchangé ; le survol ne disparaît plus à
  chaque reconstruction de la vue (horloge de lecture).
- **Survol à travers un menu** (karbon-ui, 2026-11-08) : `:hover` et `group-hover`
  touchaient tout élément dont la boîte contenait le pointeur, si bien qu'un menu
  contextuel ouvert laissait les lignes de morceaux dessous s'éclairer et montrer leurs
  boutons ; ils suivent désormais le hit-test d'un navigateur (élément peint au-dessus et
  ses ancêtres, menu ouvert = fond qui retient le pointeur), et un calque non interactif
  ne laisse plus passer clics ni curseur. Un clic dans le `padding` d'un bouton
  (élément de menu `px-3.5 py-2`) l'atteint.
- **Défilement saccadé** (karbon-ui, 2026-11-08) : mesuré dans la vraie fenêtre
  (`--profile`, crans de molette postés), chaque pochette arrivée pendant le défilement
  reconstruisait toute la vue (accueil : 55 reconstructions de 6 à 13 ms pour 60 crans).
  Les pochettes sont désormais des `ImageSlot` de karbon-ui (`images.rs`, `assets.rs`) :
  l'image arrivée est repeinte sans reconstruction ; seuls les écrans qui testent la
  présence d'une pochette (`has_cover`, `covers.get`) reconstruisent encore. Côté
  karbon-ui, une image d'animation attend aussi sa place dans la file de présentation
  avant de lire l'horloge (à-coup au départ de chaque glissement). Après : aucune
  reconstruction pendant le défilement. `--page <page>` marche aussi en fenêtre (ouverte
  une fois la bibliothèque connue).
- **Défilement de Morceaux, bibliothèque chargée** (karbon-ui et clone, 2026-11-08) :
  « le jour et la nuit » face à Tauri. Côté karbon-ui : les 200 lignes étaient peintes à
  chaque image (lignes en `overflow: visible`), chaque bouton masqué ouvrait un calque
  Direct2D, chaque élément cliquable coupait la mémoire des mesures, et les lignes
  restantes recalculaient styles et mesures au chargement d'une page ; corrigé (voir
  `docs/performance.md` de karbon-ui) : banc 203 → 8,6 ms par image, rendu en fenêtre
  ~10 → ~3,5 ms. Côté clone : `Row::n` est le rang dans la liste entière
  (`track_table_at`, début de la fenêtre chargée) et `Row::first` marque la première
  ligne (trait de séparation) ; avant, toutes les lignes changeaient de clé de mémoire et
  étaient reconstruites quand la fenêtre perdait ses 100 premières lignes. Reste ~45 ms
  par reconstruction au chargement d'une page de la liste.
- **Pochettes redemandées après une vue seulement** (clone, 2026-11-09) :
  `take_task_commands`, appelé par karbon-ui après chaque message Windows, reconstruisait
  tout le `ViewModel` pour redemander les pochettes de la page (des trous de 40 à 88 ms
  entre deux images pendant un défilement) ; il ne le fait plus qu'après une vue
  (`covers_due`), et le chargeur de pochettes garde sa file entre deux demandes.
- **Dialogues de fichiers** (karbon-ui et clone, 2026-11-08) : karbon-ui montre le
  sélecteur natif (`WindowCommand::FileDialog`, choix rendu comme une tâche ; Windows,
  macOS et Linux). Côté clone (`pages/dialogs.rs`) : « Fichier » et « Dossier » de la
  barre latérale **lisent** comme `openAudioFile` / `openAudioDirectory` (ils
  importaient : `Msg::OpenFile`, `Msg::OpenFolder`) ; import de la page d'import
  (bibliothèque créée après le choix), « Ajouter des fichiers » et « Ajouter un dossier »
  des bibliothèques, « Ajouter » de la fenêtre des dossiers, « Ajouter de la musique » de
  l'accueil (`addMusic`). Restent à raccorder : avatar de profil, pochette d'une
  collection, export .m3u, OPML, sauvegarde et import, images de l'atelier de tags.
- **Défilement de Morceaux, suite** (karbon-ui et clone, 2026-11-08) : « tu ne vides pas
  la mémoire ou les dessins ». Côté karbon-ui : `content-visibility: auto` et
  `contain-intrinsic-size` (posés par `app.css` de la source sur les lignes et vignettes),
  accessibilité sans événement de position par élément défilé (38 % du thread de
  l'interface quand un client UI Automation écoute), pistes d'animation du contenu sauté
  ignorées, ombres floutées partagées, vérification de l'arbre une fois par
  reconstruction : préparation 0,5 / 0,8 ms (p50 / p95) en défilant, processeur 58 →
  44 %. Côté clone : classes `row-track`, `row-track-detail`, `row-track-compact`,
  `row-album-track`, `card-album` et `card-artist` de la source (`assets/css/library.css`,
  gabarits des lignes et des cartes) ; la clé `memo` des lignes de table ne contient plus
  l'identité de la pochette en cache (chaque pochette arrivée pendant un défilement
  refaisait sa ligne au chargement suivant). Reste ~35 + ~15 ms aux deux reconstructions
  d'un chargement de page. Mémoire : ~280 Mo viennent du cache SQLite du backend commun
  avec Tauri (`cache_size = -64000`, 64 Mo par connexion du pool).
- **Micro-lecteur non redimensionnable** (karbon-ui, 2026-11-08) : la poignée
  `data-resize-region` est désormais suivie par karbon-ui sous Windows comme sous macOS,
  comme `resizeMicroPlayer` de la source : souris capturée, côté = départ + max(dx, dy)
  en coordonnées de l'écran, borné 220-660 (`MinSize`, `MaxSize`, `AspectRatio`), une
  taille par image ; au relâchement `mini_window` retient la taille et ramène la fenêtre
  dans l'écran (`endMicroResize`). Les bords de la fenêtre fixe ne redimensionnent pas ;
  plus de `WS_THICKFRAME` temporaire ni de boucle modale de Windows.

Restent :
- la télécommande d'ampli (`remote/AmpRemote.svelte`, environ 900 lignes) ;
- bloqués par karbon-ui (voir les manques) :
  - aperçu d'import, « pochette depuis un fichier », ajout ou remplacement d'une image
    dans l'éditeur de tags, ajout d'un dossier (dialogue de fichiers).

Gabarits : une inclusion est recopiée à chaque usage et l'ensemble est borné à 2 Mio ; les
briques répétées des réglages sont des composants (`settings_kit.rs`), compilés une fois.

Mémoire : sans visibilité, une grille demande toutes ses pochettes ; les grilles sont
limitées à 96 cartes (fenêtres de 48) et les petites vignettes (lignes, mosaïques) sont
décodées à 160 px (`images::small_key`).

## Pas encore traduit

Dialogues de fichiers (Fichier, Dossier, import, OPML, export des réglages), popins
(playlists, playlists auto, profils, nouveautés), éditeur de tags, paroles,
glisser-déposer de la file, saisie d'un raccourci, raccourcis clavier globaux, contrôles
média du système, notifications, démarrage automatique, mise à
jour, instance unique.

## Manques de karbon-ui relevés

- ~~Une infobulle vide est une erreur fatale~~ : corrigé le 2026-11-07, un `title` vide n'affiche rien.
- Pas de moyen pour un autre thread de réveiller l'application : une tâche « pompe »
  occupe en permanence un des quatre ouvriers.
- L'aperçu hors écran n'exécute les tâches qu'après `load`, avant la première vue.
- Les lignes de `k-virtual-list` n'ont pas accès aux ressources (pas de pochettes).
- Absents : presse-papiers côté application, notifications, handle natif de la fenêtre.
  (Dialogues de fichiers, événements de position et de taille, « afficher / premier
  plan », icônes de fenêtre et de zone de notification faits.)

## Mesures (Windows, base réelle, 25 s après le lancement)

| Cas | Privé |
|---|---|
| Natif, base vide, GPU | 113 Mo |
| Natif, base vide, logiciel | 73 Mo |
| Natif, base réelle, GPU | 502 Mo |
| Tauri installé, après des heures, 7 processus | 431 Mo |
| Tauri installé, processus principal seul | 30 Mo |

Le surplus avec la base réelle vient des données (caches SQLite de 64 Mo par connexion,
pochettes décodées) ; une comparaison juste demande les deux applications au même moment
après un lancement à froid.
