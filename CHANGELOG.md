# Changelog

## [0.2.3] - 2026-08-27

La vue tableau devient une vraie table : colonnes redimensionnables, en-tête
qui reste visible, et un mode d'affichage disponible partout. La sélection
multiple sort enfin de l'onglet Morceaux.

### Colonnes — Largeur
- Redimensionnement à la souris par une poignée sur le bord droit de chaque
  en-tête. Elle vit hors du bouton de tri : dedans, un appui pour élargir
  déclencherait le tri de la colonne.
- Double-clic sur la poignée : la colonne s'ajuste à son contenu. La mesure
  passe par un canevas et non par le DOM — les lignes portent
  `content-visibility`, donc celles hors du cadre n'ont pas de disposition
  calculée et les mesurer rendrait une largeur juste pour l'écran courant.
- L'intitulé entre dans le calcul : sans lui, une colonne ajustée à un contenu
  court afficherait « Dernière éc… » en en-tête.
- Les largeurs sont enregistrées par colonne, et « Rétablir les largeurs »
  ramène tout à l'origine.
- Le titre ne descend plus sous 180 pixels. Seule colonne compressible, il
  absorbait toute la compression et disparaissait au-delà d'une poignée de
  colonnes ajoutées.

### Colonnes — Affichage
- L'en-tête reste visible au défilement. Une valeur sans intitulé ne veut rien
  dire, surtout quand ce sont des tags qu'on a soi-même choisis.
- Numéro, pochette et titre entrent dans le sélecteur : les trois premières
  colonnes se masquent et se réordonnent comme les autres.
- Les réglages écrits avant ce changement sont repris, sans quoi les trois
  auraient disparu d'un coup.

### Modes d'affichage
- Vue tableau pour les onglets Genres et Dossiers, qui n'avaient que la grille.
- La bascule grille/liste s'affiche aussi sur les pages de détail — le mode y
  existait sans qu'on puisse le demander.
- Les sous-sections des pages de détail suivent le mode : autres albums de
  l'artiste, albums du même genre, artistes similaires, discographie.

### Sélection multiple
- Fonctionne sur les albums, artistes et genres, en grille comme en liste.
  Cocher un album développe ses pistes : le compteur annonce « 47 titres » et
  non « 3 éléments », et toutes les actions continuent de ne connaître qu'une
  seule sorte d'objet.
- Le bouton « Sélectionner » passe dans la barre d'onglets. Il n'existait que
  sur quatre pages ; l'onglet Dossiers et les pages de détail n'avaient aucun
  moyen d'entrer en sélection.
- Maj + clic étend depuis le dernier élément cliqué, comme dans un explorateur.
- « Tout » fonctionne partout : il s'appuyait sur une liste que seul l'onglet
  Morceaux remplissait.
- Un clic droit sur une ligne déjà cochée porte sur toute la sélection.

### Playlists
- Le nombre d'écoutes s'affiche sur chaque ligne, et seulement s'il y en a.
- Les playlists intelligentes se reconnaissent à une pastille dans la barre
  latérale, et leur compteur se recalcule au lieu de rester figé à sa valeur
  d'enregistrement.
- L'en-tête d'une playlist intelligente écrit ses règles en clair.
- Les recettes sont disponibles à la modification, sous « Remplacer par une
  recette » — elles étaient masquées, alors que repartir d'une recette est
  précisément ce qui répare une playlist mal réglée.

### Corrections
- « Simple clic = lecture » désactivé : cliquer une piste affichait
  « Préparation du morceau… » puis rien. Un clic produisait deux intentions
  concurrentes — la file mutée lançait la lecture, l'action la coupait
  aussitôt. Le même défaut appelait `playFile` deux fois quand le réglage
  était actif.
- Le titre était la seule colonne qu'on ne pouvait pas redimensionner :
  `flex` écrasait la largeur qu'on lui donnait.
- Une playlist intelligente refusait de s'afficher — toutes ses lignes
  portaient la même clé.
- Modifier une playlist intelligente réécrivait sa couleur et son icône : ni
  l'une ni l'autre n'était rechargée.
- Une recette appliquée après une autre gardait le nom de la première.
- Mode clair : cases à cocher blanches sur blanc, menu de sélection resté
  entièrement sombre, pastilles de tri quasi invisibles.
- Les pastilles de tri des pages album et artiste disparaissent en vue
  tableau, où l'en-tête trie déjà.
- Défilement absent sur les trois pages de playlist.
- Le menu contextuel ne propose plus « Retirer » sur une playlist
  intelligente : l'action n'aurait rien fait tout en semblant marcher.

## [0.2.2] - 2026-08-24

Les playlists se composent à partir de règles, les colonnes se choisissent, et
les réglages s'exportent.

### Playlists intelligentes
- Des règles sur 24 champs de la bibliothèque et sur n'importe quel tag des
  fichiers, avec groupes imbriqués, tri et coupe.
- Le nombre de morceaux retenus se recalcule pendant qu'on écrit les règles :
  « supérieur à » et « au moins » s'expliquent mal, un compteur qui passe de
  3 000 à 12 ne laisse aucun doute.
- Huit recettes toutes faites, dont « les styles les plus écoutés », qui somme
  les écoutes par genre au lieu de figer une liste décidée un jour.
- Le contenu ne se stocke pas, il se calcule : la page, la lecture, la mise en
  file et l'export passent tous par le même chemin sans connaître la
  différence.

### Compteur d'écoutes
- Il n'existait pas. La fonction était écrite dans le dépôt mais n'était
  appelée nulle part, et `play_count` valait zéro sur toute la bibliothèque.
- Une écoute compte au-delà de la moitié du morceau, plafonnée à une minute.
  Compter dès le premier échantillon aurait fait d'un survol de bibliothèque
  une série de fausses écoutes.

### Colonnes au choix
- 21 champs de la bibliothèque et tous les tags réellement présents dans les
  fichiers, recensés avec leur effectif — proposer la liste théorique noierait
  les cinq tags utiles sous trente-cinq inutiles.
- Tri par en-tête sur n'importe quelle colonne, dans les deux sens : en base
  pour l'onglet Morceaux, en mémoire ailleurs. Trier en mémoire une page sur
  quatorze mille morceaux donnerait un résultat faux à l'air juste.
- La vue tableau s'étend aux albums, artistes, genres et playlists.

### Export et import
- Réglages, profils, playlists et titres aimés dans un fichier JSON.
- Les playlists n'emportent pas d'identifiants, qui n'auraient aucun sens
  ailleurs : chaque piste part avec son chemin et de quoi la reconnaître si ce
  chemin a changé.
- L'import annonce ce qu'il fera avant de l'écrire, et ne remplace une
  playlist existante que si on le demande.

### Notation
- Demi-étoiles, en décimal. Les notes existantes valent déjà 1,0 à 5,0 :
  aucune donnée n'est transformée.
- Les notes lues dans les fichiers gagnent la même finesse, là où l'arrondi à
  l'étoile entière jetait la moitié de l'information.

### Corrections
- La file ne s'enchaînait pas en bit-perfect : le `Drop` des sorties
  exclusives levait le drapeau d'arrêt, et `playback-ended` n'était jamais
  émis. WASAPI et ALSA étaient touchés.
- Les compteurs de playlist mentaient. Retirer un dossier efface en cascade
  fichiers, pistes et entrées de playlist ; le compteur restait sur son
  ancienne valeur. Il se lit désormais au lieu d'être cru.
- Menus déroulants blancs en thème sombre : `color-scheme` n'était pas
  déclaré, et un fond translucide ne peut pas servir de surface à une liste
  déroulante.
- Listes plus fluides : `content-visibility` sur les lignes, et un flou
  d'arrière-plan par piste supprimé — son fond était uni, il n'y avait rien à
  flouter.
- Le téléchargement des portraits d'artistes est débrayable.
- La fenêtre retient sa taille et sa position.

## [0.2.1] - 2026-08-20

L'atelier de tags gagne de quoi ranger une bibliothèque, pas seulement la
corriger : trouver ce qui cloche, renommer, réorganiser les dossiers, et
revenir en arrière.

### Tags — Inventaire « à corriger »
- L'atelier ouvert sans sélection montre désormais ce qu'il y a à reprendre,
  au lieu d'un écran vide : sans titre, sans artiste, sans album, sans année,
  sans numéro, sans pochette, format non réinscriptible.
- Détection des incohérences d'album : années divergentes, compilation sans
  artiste d'album, numéros de piste en double, trou dans la numérotation.
- Détection des doublons : même titre et même artiste, à deux secondes près.
  Ils se chaînent de proche en proche — 180 / 181,5 / 183 secondes sont bien
  le même enregistrement, alors qu'aucune paire extrême ne tient dans la
  tolérance.
- Un clic sur une catégorie charge les fichiers concernés dans le tableur.

### Tags — Renommage des fichiers
- Motifs avec champs, complément par des zéros (`{track:02}`), repli
  (`{albumartist|artist}`) et groupes optionnels (`[{disc}-]`) qui
  disparaissent entièrement quand le champ est vide.
- Aperçu avant/après ligne par ligne, recalculé à la frappe, avec sélection
  individuelle : on décoche ce qu'on ne veut pas renommer.
- Détection avant exécution des collisions, des cibles déjà occupées, des
  renommages en chaîne et des chemins trop longs.
- Les noms sont assainis pour Windows, macOS et Linux. Les séparateurs
  `/ \ : |` deviennent un tiret — « Main Title / Chase The Red BMW » donne
  « Main Title - Chase The Red BMW » et non une suite de soulignés — et les
  ornements `? * " < >` disparaissent.
- Le motif s'applique aux tags **corrigés** : on corrige dans le tableur, on
  renomme d'après ce qu'on vient d'écrire.

### Tags — Réorganisation des dossiers
- Le motif recompose l'arborescence complète, avec quatre modèles courants.
- Les fichiers compagnons suivent : `.lrc` et `.cue` avec leur piste, les
  pochettes et les `.m3u` avec leur dossier — et seulement si tout le dossier
  part au même endroit.
- Nettoyage des dossiers devenus vides, jamais au-delà des racines de la
  bibliothèque.
- Le changement de volume est traité : copie, vérification, puis suppression
  de la source — jamais l'inverse.

### Tags — Journal et annulation
- Chaque lot est journalisé : renommages, déplacements, et l'état des tags
  avant réécriture.
- Écran d'historique avec le motif employé, les compteurs, et l'annulation
  en deux temps. Les cinquante derniers lots sont conservés.
- L'annulation rend compte comme un lot : elle peut échouer et le dit. Un
  fichier retouché depuis l'opération n'est pas remis en place.

### Tags — Règles de nettoyage
- Cinq règles, aucune cochée d'office : soulignés en espaces, retrait du
  numéro en tête de titre, uniformisation de « feat. », espaces en trop,
  capitales à l'anglaise.
- Aperçu cellule par cellule ; le résultat va dans les modifications en
  attente, pas dans les fichiers.

### Corrections
- **Un morceau renommé devenait illisible.** La file d'attente conservait
  l'ancien chemin : sept tables stockent un chemin absolu, et celle-là
  manquait à l'inventaire. Corrigé, et couvert par l'annulation.
- L'atelier et le cache de la bibliothèque se remettent à jour après un
  déplacement — ils gardaient les chemins d'avant.
- **L'application ne démarrait plus après une mise à jour partielle.** Quand
  la base a été migrée par une version plus récente, le démarrage échouait
  sans un mot : sous Windows une application graphique n'a pas de console, et
  les erreurs fatales ne passaient pas par le journal. Elle affiche désormais
  un message qui dit quoi faire.

### Interne
- Tout le SQL de données est passé dans `repository/` : trente et une requêtes
  vivaient encore dans les commandes et les services.
- 197 tests Rust.

## [0.2.0] - 2026-08-14

Deux chantiers structurants : la **sortie bit-perfect est complète sur les trois
systèmes**, et l'application sait désormais **écrire les tags** — pas seulement
les lire.

### Audio — Sortie bit-perfect sur les trois systèmes
- **DoP natif sous Linux** via ALSA `hw:` en accès exclusif : le DSD part au DAC
  sans conversion PCM, marqueurs DoP continus (musique et silence), sans
  resampling ni volume logiciel. Équivalent Linux du WASAPI exclusive de Windows.
- **Réservation D-Bus `org.freedesktop.ReserveDevice1`** : PipeWire/WirePlumber
  libère la carte avant l'ouverture exclusive et la reprend à l'arrêt. Plus
  aucune manipulation manuelle (`pactl`).
- **DoP natif sur macOS** via CoreAudio *hog mode*.
- **Sortie exclusive PCM** sur macOS et Linux, en plus du DSD.
- **Lecture sans blanc** entre les pistes.
- Persistance du périphérique de sortie : le choix est écrit en config et
  restauré au démarrage, l'énumération ne l'écrase plus.
- Énumération nettoyée : les alias ALSA virtuels (`plughw:`, `surround`,
  `iec958`, `sysdefault`…) sont filtrés aussi sur le champ pilote — fin des
  doublons dans la liste des sorties.
- Correction : deux endpoints portant le même nom brut ne se confondent plus.

### Audio — Replay Gain
- Lecture des tags `REPLAYGAIN_*` et application du gain sans base de données :
  atomique global greffé sur le volume, chaîne DoP intacte.
- Préampli réglable, modes piste / album, dans les Réglages Audio.

### Tags — Écriture complète (MP3, FLAC, DSF, DFF)
- Nouveau module `core/audio_metadata/injector/`, miroir en écriture de
  l'extracteur. Texte et images, sur les quatre formats.
- **Rien n'est perdu** : les frames et blocs non édités sont recopiés tels
  quels — corriger un titre ne fait perdre ni pochette, ni note, ni
  identifiants MusicBrainz. Une image conservée est recopiée octet pour octet,
  la promouvoir en pochette ne la réencode jamais.
- **Écriture en place** quand le bloc de remplissage le permet : 2613 ms → 20 ms
  sur un FLAC de 34 Mo via SMB. Repli sur réécriture atomique complète sinon.
- Les images sont désignées par un hash de leur **contenu**, jamais par leur
  position : un décalage d'indice supprimerait la mauvaise image.
- Le backend reçoit l'**état visé**, pas des opérations : ajouter, remplacer,
  supprimer, réordonner et définir la pochette tiennent en une seule écriture.
- Éditeur individuel : popin deux colonnes, suivi des modifications champ par
  champ avec rétablissement, veto de fermeture sur travail non enregistré,
  gestion complète des médias intégrés.

### Tags — Atelier par lot
- Nouvelle page dédiée, deux vues au choix : tableur ou liste + panneau.
- Moteur de traitement par lot avec progression, annulation entre les lots et
  rapport d'échecs ; une tâche qui panique devient un échec, jamais un arrêt.
- Édition multiple : un champ inchangé est **omis** et non envoyé vide — la
  distinction entre « ne touche pas » et « efface » est ce qui évite de
  détruire cent tags que personne n'a demandé de supprimer.
- Remplissage vers le bas, numérotation automatique, tri par colonne,
  pochette commune.
- Lecture parallèle des fichiers (rayon) : 769 ms → 35 ms à froid pour seize
  fichiers, et cent six allers-retours IPC supprimés.

### Tags — Récupération depuis Deezer
- Recherche d'album, appariement des pistes par titre, durée et numéro, avec
  score et niveau de confiance ; un appariement écarté n'est jamais appliqué.
- Écran de revue **par morceau** : les valeurs du fichier en regard de celles
  de Deezer, dépliables champ par champ. Trois niveaux de choix — une valeur,
  un morceau, un champ sur toute la sélection.
- Pochette par ligne : la vignette du fichier dans la liste, celle de Deezer
  en regard quand elle est retenue, avec ses dimensions et son poids **après
  préparation**. Cochée d'office seulement là où il n'y en a aucune.
- Recherche de **piste** pour un fichier isolé, avec comparaison avant/après et
  sélection champ par champ.
- Rien n'est écrit directement : tout passe par les modifications en attente de
  l'atelier, relues avant d'écrire.
- Débit limité côté client ; l'objet `error` des réponses est vérifié — l'API
  répond 200 même en cas d'échec.

### Réglages — Refonte en sous-pages
- Page monolithique éclatée en `/settings/general`, `appearance`, `audio`,
  `network`, `storage`, `about`, avec navigation latérale par sections et
  en-tête fixe.
- Nouveaux composants réutilisables : `OptionRow`, `ToggleSwitch`,
  `ActionButton`.

### Linux — Fiabilité du rendu
- Un démarrage GPU qui plante avant d'afficher l'interface (EGL_BAD_PARAMETER,
  fenêtre blanche) est détecté par fichier sentinelle et bascule
  automatiquement l'app en rendu logiciel au lancement suivant.
- Écoute du signal WebKit `web-process-terminated` : un crash du processus de
  rendu en mode GPU est persisté et l'app redémarre d'elle-même en logiciel.
  Jamais de boucle : un crash en mode logiciel n'est que journalisé.
- Un Alt+F4 sur la fenêtre blanche ne désarme plus la sentinelle.
- Détection SteamOS : rendu logiciel d'office sur Steam Deck.
- Variable `RUSTMUSIC_RENDER=gpu|software|auto`, prioritaire sur le réglage.
- `scripts/fix-appimage-wayland.sh` : retire les `libwayland-*` embarquées par
  linuxdeploy, cause racine du crash EGL sur SteamOS, Arch et Ubuntu 26.04.

### Accessibilité — Mode contraste élevé
- `<html data-contrast="high">` redéfinit les variables de couleur de Tailwind :
  tous les gris secondaires deviennent lisibles d'un coup, sans reprendre les
  composants un par un.
- Les flous d'arrière-plan sont coupés — ils brouillent le texte posé dessus.
- Focus clavier franchement visible ; `data-focus-ring` permet à un composant de
  **déplacer** l'indicateur, jamais de le supprimer.
- Cases à cocher entièrement redessinées : `accent-color` ne repeint que l'état
  coché, une case vide restait un carré blanc dessiné par le système. La coche
  est une image SVG de fond et non un pseudo-élément — WebKit ne rend pas ceux
  des champs de formulaire.

### Bibliothèque
- **Bibliothèque par défaut** : marqueur par profil, unicité tenue par un index
  partiel en base plutôt que par le code. Reprise automatique à la création de
  la première bibliothèque et à la suppression de celle par défaut.
- Menu contextuel au clic droit dans « Récemment joués ».
- « Corriger les tags » dans les menus contextuels des albums et des listings.

### Fiabilité sur les partages réseau
- `is_network_path()` ignorait les lecteurs mappés (`S:` pour `\\NAS\music`).
  Ces fichiers gardaient une poignée ouverte pendant la lecture, et SMB refuse
  alors tout remplacement — d'où l'échec « accès refusé » à l'édition. Le type
  de lecteur est maintenant demandé à l'OS, et la lecture depuis un NAS
  précharge enfin en RAM comme prévu.
- Windows refuse de renommer par-dessus un fichier en lecture seule :
  l'attribut est levé puis reposé. Réessai court pour les gêneurs passagers.
- Ouverture du dossier d'un morceau : `revealItemInDir` remplace `openPath`,
  dont la capability était limitée à `$HOME` et `$APPDATA`.

### Dépendances
- symphonia 0.5.5 → 0.6.0 (nouvelle API probe/décodeur/buffers, tags typés).
  Conversion audio via `copy_to_vec_interleaved`, qui couvre aussi
  S24/U16/U24/U32.
- Feature `isomp4` : les AAC en conteneur M4A sont désormais lisibles.
- rubato 2 → 3, Tauri 2.11.5, alsa 0.11, zbus 5, regex 1.13.
- SvelteKit 2.69.2, Vite 8.1.4, svelte-check 4.7.2. TypeScript maintenu en 6.x.
- sqlx maintenu en 0.8.6 (0.9 bloqué par tauri-plugin-sql).

### Projet
- Modèles d'issues GitHub et politique de sécurité.
- Bouton mini-lecteur déplacé dans la barre de titre, à côté des contrôles de
  fenêtre (adapté aux trois styles et aux deux positions).

### Connu, non livré
- Navigation clavier dans le tableur de l'atelier.
- Renommage de fichiers et restructuration de dossiers depuis des motifs de
  tags — prévus, non commencés.

## [0.1.9] - 2026-07-08

### UI — Minuterie de veille (sleep timer)
- Bouton minuterie dans l'en-tête (à côté du sélecteur de thème), avec menu déroulant premium
- Durées rapides (15 / 30 / 45 / 60 min) + durée personnalisée, ou mode « fin du morceau en cours »
- Compte à rebours affiché en direct ; met la lecture en pause à l'échéance
- Option pour masquer le bouton dans les Réglages (section Général)

### UI — Mini-lecteur (compact, toujours au premier plan)
- Bascule vers une fenêtre compacte always-on-top depuis le player (icône picture-in-picture)
- Infos de lecture (pochette, titre, artiste, badge qualité DSD/Hi-Res, fréquence), transport et barre de progression cliquable
- Deux onglets déroulants : File d'attente (cliquable pour sauter à une piste) et Paroles synchronisées (ligne active surlignée, défilement auto)
- Animation de déroulement fluide, fenêtre dimensionnée au pixel près sur son contenu
- Restauration de la fenêtre principale en un clic

### Corrections & maintenance
- Compatibilité avec la montée de version des dépendances (CPAL 0.18, rubato 2.0, reqwest 0.13, image 0.25, Svelte/Vite/TypeScript)
- Correction de compilation Linux (chemin de module `system_detect`)

## [0.1.8] - 2026-07-01

### Audio — WASAPI exclusive (Windows, bit-perfect)
- Backend WASAPI exclusive event-driven : bypass complet du mixeur Windows, format natif envoyé au DAC (vraie sortie bit-perfect)
- Pré-négociation du format sur le DAC ciblé AVANT le décodeur → le décodeur produit directement au rate natif (pas de resampling parasite ni de pitch faux)
- Ciblage du device par nom complet « Nom (Fabricant) » — corrige le cas où plusieurs endpoints partagent le même nom brut (le mauvais DAC était sélectionné)
- Le render WASAPI lit aussi le mode FullBuffer (comme CPAL) — corrige le silence total sur les profils qui basculent en pré-décodage
- Pré-remplissage du buffer avant `start_stream` — supprime le clic de démarrage
- Toggle WASAPI exclusive dans les Réglages ET dans le popup des sorties audio du player
- Badge permanent dans la status bar : « WASAPI » (bit-perfect) ou « Standard » (mixeur Windows), cliquable vers les Réglages

### Audio — DSD natif (DoP / DSD over PCM)
- Nouveau : envoie les fichiers DSD (.dsf/.dff) **tels quels** au DAC compatible DoP, qui les décode nativement — plus de conversion DSD→PCM
- Le DAC affiche « DSD » et reconstruit le flux 1-bit d'origine (bit-perfect). Porteur : DSD64→176.4 kHz, DSD128→352.8 kHz, DSD256→705.6 kHz
- **Moteur DoP persistant (gapless)** : le stream WASAPI reste vivant entre les pistes DSD compatibles (silence DSD dans les intervalles) → le DAC ne se re-verrouille qu'une fois, les pistes suivantes démarrent instantanément sans clic
- Warm-up au 1er morceau (le temps que le DAC verrouille le DSD) avec compteur figé → plus de début de morceau perdu
- Fermeture automatique du moteur (libère le DAC) au passage vers un format non-DSD, un DSD incompatible, ou après 5 s d'inactivité
- Fallback automatique et transparent vers DSD→PCM si le DAC refuse le format DoP
- Volume logiciel grisé en DoP (bit-perfect oblige — le volume se règle sur le DAC/ampli)
- Toggle « DSD natif (DoP) » dans les Réglages, badge « DSD natif » dans la status bar
- Encodeur DoP maison : inversion de bits DSF (LSB-first), marqueurs 0x05/0xFA posés par le backend en continu (anti-clic aux jonctions)

### Audio — Périphériques de sortie enrichis
- Nouvelle énumération détaillée : fréquences supportées, formats d'échantillon, canaux, marqueur défaut système, détection Hi-Res (≥24-bit et ≥88.2 kHz)
- Probing WASAPI par périphérique (Windows) : interroge directement le pilote pour connaître les capacités RÉELLES du DAC en exclusive (CPAL exposait la même table pour tous les endpoints)
- Modal de détails par périphérique : identité, formats, fréquences groupées par catégorie (CD / Hi-Res / Studio / Ultra Hi-Res), format Windows par défaut, taille de buffer
- Sélection de la sortie active depuis le player (badge Hi-Res, fréquence max) et depuis les Réglages
- Carte « Périphériques de sortie » dans Réglages > Audio

### UI — Refonte des Réglages
- Passage en mode pleine page avec sidebar de navigation à gauche (Général / Apparence / Audio / Réseau & DLNA / Stockage / À propos)
- Groupes « Préférences » et « Application », barre d'accent sur la section active, en-tête sticky avec titre + description par section
- Section « À propos » désormais intégrée directement (plus de sous-page séparée)
- Section Audio réorganisée : périphériques, WASAPI + DSD natif regroupés dans une carte hiérarchisée, qualité de décodage

### UI — Player & pipeline
- Popover pipeline enrichi : bannière bit-perfect / DSD natif, chaîne source → sortie, backend effectif, transport DoP
- Info « source → sortie » affichée en permanence dans la status bar (couleur selon le mode : bit-perfect / DSD natif / standard / resamplé / DSD→PCM)
- Modales portées vers le body (fix positionnement)

## [0.1.7] - 2026-05-21

### Audio — Profil de qualité de décodage
- Système de profils audio Auto / Qualité maximale / Équilibré / Compatibilité / Mode dégradé
- Auto détecte les VMs et le nombre de cœurs CPU pour choisir le bon preset (VM → Mode dégradé, < 4 cœurs → Équilibré, sinon Qualité maximale)
- Mode dégradé : filtre DSD 256 taps + sortie 44,1 kHz + pré-décodage complet du fichier en RAM avant lecture (zéro underrun garanti)
- Filtre DSD configurable par profil : 2048 / 1024 / 512 / 256 taps
- Resampler chunk size et sub-chunks adaptés au profil
- Settings : 5 cartes de sélection + bloc d'information affichant le profil résolu et la machine détectée

### Audio — Décodage parallèle DSD multicanal
- Parallélisation du DSD2PCM via rayon : 1 thread par canal pour les fichiers 3+ canaux (SACD multicanal 5.0 / 5.1)
- Stéréo et mono restent en séquentiel (overhead rayon non justifié)
- Speedup quasi-linéaire sur les multicanaux : un SACD DSD64 5.0 en Qualité maximale est désormais lisible sans grésillement sur un CPU normal

### Audio — Downmix multicanal vers stéréo (ITU-R BS.775)
- Remplace l'ancien "copie ch0 + ch1 seulement" qui supprimait la voix centrale et les surrounds
- 3.0 → stéréo : L + 0,707·C × 0,707
- 4.0 (quad) → stéréo : L + 0,707·LS × 0,707
- 5.0 → stéréo : (L + 0,707·C + 0,707·LS) × 0,5 (−6 dB pour éviter le clipping)
- 5.1 → stéréo : ajoute 0,5·LFE dans le mix
- Tu entends maintenant correctement les SACD multicanaux en stéréo

### Audio — Pré-chargement et seek
- Pré-chargement du fichier audio en RAM quand le morceau est sur un partage réseau (GVFS/SMB/NFS/SSHFS) ou en profil Bas / Mode dégradé — élimine les coupures liées à l'I/O
- Seek instantané en Mode dégradé pour DSD : drain du ring buffer dans un `Vec<f32>`, CPAL lit en mode FullBuffer (curseur indexé) au lieu du FIFO
- Fix race condition Phase 6 / CPAL sur le seek en Mode dégradé Symphonia (atomique dédié `pending_seek_frames`)

### Audio — Robustesse
- CPAL `BufferSize::Fixed` : la valeur est désormais clampée dans la plage supportée par le périphérique (fix stream silencieux sur certaines cartes Intel HD)
- Log des erreurs CPAL rate-limité (1ère, puis 1 sur 100 jusqu'à 1000, puis 1 sur 1000) — sur VM faible on peut en recevoir des centaines par seconde
- Defense-in-depth scanner DSD : `catch_unwind` supplémentaire à l'entrée DSD pour transformer toute panic des parsers DSF/DFF en erreur propre — impossible de tuer le batch d'import
- Aggregation des fichiers ignorés à l'import + event Tauri enrichi (`skipped: usize`)

### Système de notifications OS
- Notification native (Windows action center / macOS / Linux libnotify) au changement de morceau avec titre + artiste
- Skip de la première émission au démarrage (restore de queue ≠ action utilisateur)
- Toggle dans Réglages (icône cloche)

### System Media Transport Controls (SMTC / MPRIS / Now Playing)
- Le morceau en cours apparaît dans le mini-player Windows (volume flyout), l'écran de verrouillage, et le widget Now Playing macOS / MPRIS Linux (KDE Plasma, GNOME)
- Les touches média du clavier (play/pause/next/prev) fonctionnent désormais
- Serveur HTTP local dédié pour servir les covers (cross-OS, contourne la limitation Windows unpackaged qui rejette les `file://`)
- Toggle dans Réglages pour désactiver l'intégration (utile sur Linux sans D-Bus, Windows N sans Media Pack)
- Implémenté via le crate `souvlaki`

### Render mode (Linux)
- Override manuel Automatique / Accélération GPU / Rendu logiciel
- Auto détecte si l'app tourne en VM et bascule en software rendering pour la stabilité
- Settings dédiés avec bannière "Redémarrage requis"
- Variables d'environnement WebKitGTK / GDK appliquées avant le démarrage de Tauri

### UI Player
- Popover hover stylé sur le badge "source → sortie" (PipelineInfoPopover) — affiche Source / Décodage / Resampling / Sortie / Profil avec badges Bit-perfect / DSP / DSD→PCM
- Indication multicanal correcte dans le popover : "3.0" / "4.0" / "5.0" / "5.1" / "6.1" / "7.1"
- Layout responsive sur 3 breakpoints : mobile (stacked), intermédiaire 500-768px (cover + actions en haut, transport en bas), desktop ≥768px (3 colonnes)
- Toutes les actions transport (shuffle, ±10s, play, next, repeat) restent visibles à toutes les tailles
- Fallback titre depuis le nom de fichier pour les DSF/DFF sans tag DITI/ID3
- Indicateur de pré-décodage dans la StatusBar (Mo décodés / Mo total) au lieu d'un loader dans le Player

### Internationalisation
- Mise à parité totale des 4 langues : FR, EN, DE, ES (toutes à 282/282 clés strictement identiques)
- Sections rattrapées : bibliothèque (genres, artist_label, total_duration), réglages (audio_quality_*, render_mode_*, scan_on_startup, single_click_play, album_covers), pipeline, statistiques, recherche, common

### Stabilité & Bugs
- Détection automatique du format réel des images via les magic bytes : fix des covers PNG/WebP renommées en `.jpg` (warning `Format error decoding Jpeg: Illegal start bytes` éliminé)
- Iconify offline : pré-bundling de 7 collections (lucide, heroicons, mynaui, ph, radix-icons, tabler, uit) — fix des icônes invisibles sur Debian 12 + WebKitGTK 2.40, fonctionne désormais sans connexion
- Auto-config Linux : détection VM + variables d'environnement WebKit / GDK pour éviter les freezes au démarrage sur certaines VMs (KVM, KDE Wayland, AMD Mesa)
- Fix svelte-check : 10 erreurs préexistantes corrigées (EditPlaylistPopin, ProfilSelectorPopin, page détail morceau avec ajout de `library_artist_id` au modèle TS + SQL JOIN library_artists)

## [0.1.6] - 2026-05-05

### Audio metadata extractor — refonte complète
- Nouveau module `audio_metadata` (extractor + file_format + tag_format)
- Parser ID3v2.3 / v2.4 entièrement maison : 35+ frames mappés (TIT2, TPE1, TALB, TCON, TRCK, TPOS, TBPM, TKEY, POPM, APIC, USLT, COMM, etc.)
- Support des encodages ISO-8859-1 / UTF-16 BE+LE avec BOM / UTF-16BE no BOM (v2.4) / UTF-8
- Helpers unsynchronisation, synchsafe int, split_at_null encoding-aware
- Folder cover fallback consolidé (cover.jpg / folder.jpg / front.{jpg,jpeg,png,webp,avif})
- Champ `total_tracks` ajouté, `compilation` typé bool, parsing TCMP correct

### Lecture DSD native (DSF + DFF)
- Décodage en pur Rust, sans Symphonia ni FFmpeg
- Parser DSF (Sony) et DFF (Philips DSDIFF) avec leur format de bytes spécifique (LSB-first vs MSB-first)
- Convertisseur DSD → PCM via l'algorithme Gesemann avec LUT 256 × 256 + filtre Blackman-Harris 2048 taps (foobar2000-grade)
- Initialisation de l'historique du filtre à `0x69` pour éliminer le pop au démarrage
- Décimation fixe 32× (DSD64 → 88,2 kHz, DSD128 → 176,4 kHz, etc.)
- Pipeline complète : Decoder → DSD2PCM → Resampler rubato → ring buffer → CPAL
- Seek block-aligned validé sur DSF et DFF

### Frontend DSD
- Badge "DSD64" doré dans le player, les listes, les vues compactes
- Affichage du signal en MHz (au lieu de kHz) pour les DSD
- Helper `audioFormatTools.ts` : `isDsdFormat()`, `dsdLabel()` ("DSD64/128/256/512/1024")
- Le badge porte tout, le format/bits/kHz redondants sont masqués

### Serveur DLNA / UPnP intégré
- Découverte automatique sur le réseau local via SSDP multicast 239.255.255.250:1900
- Streaming HTTP avec support des Range requests pour le seek dans les apps clientes
- ContentDirectory : navigation par Artistes / Albums / Dossiers
- Multi-bibliothèques natif (chaque lib expose ses propres contenus)
- Covers DLNA via `<upnp:albumArtURI>` dans le DIDL-Lite
- Section "Réseau" dans les réglages : toggle, nom du serveur, port, statut, copier l'URL
- Indicateur de statut dans la barre du player
- Auto-start au lancement si activé en réglages

### Paroles synchronisées (Apple Music style)
- Table SQLite `lyrics` avec source enum (Sidecar, LRCLIB, Manual, None)
- Client LRCLIB pour récupérer automatiquement les paroles synchronisées
- Lecteur de sidecar `.lrc` à côté du fichier audio (heuristique has_synced_timestamps)
- Parser LRC frontend avec binary search activeIndex O(log n) + métadonnée offset
- Auto-scroll basé uniquement sur la ligne active (instant si delta > 3 lignes, smooth sinon)
- Fond cover floutée + voile noir + dégradé en arrière-plan du panel
- Service avec dedup in-flight pour éviter les requêtes en double

### Instance unique
- Plugin `tauri-plugin-single-instance` : un seul process à la fois
- Au second lancement, la fenêtre existante est restaurée, focus, demande d'attention sur la barre des tâches

### Stabilité & Qualité
- Fix critique : `LibraryFile.modified_at` (INTEGER en BDD vs `Option<String>` en entity) faisait échouer le décodage en `SELECT *` → aucun morceau n'apparaissait dans la vue Dossiers DLNA
- Logger termlogger + cleanup massif de la verbosité (info → debug pour les events per-action SSDP, HTTP, SOAP, audio player, DSD, resampler)
- Helper formatBitrate qui affiche en Mb/s quand bitrate ≥ 1000 kbps
- Composant réutilisable `NowPlayingCard` avec variantes default / blur (mutualisé entre QueuePanel et LyricsPanel)

## [0.1.5] - 2026-05-02

### Paroles synchronisées (foundation)
- Table SQLite `lyrics` + migration + repository + entité Rust
- Commandes Tauri `get_lyrics(path)` et `refresh_lyrics(path)`

### Section Apparence (réglages)
- Nouveau panneau "Apparence" avec sélecteur de thème Auto / Clair / Sombre
- Mini previews du thème dans chaque carte (split image)
- Application live à chaque changement, listener `prefers-color-scheme` pour le mode Auto

### Contrôles fenêtre — 4 styles + 2 positions
- Auto (détection OS) / macOS / Windows / Linux
- Style macOS : 3 traffic lights colorés
- Style Windows : 3 boutons rectangulaires
- Style Linux : 3 boutons ronds GNOME Adwaita
- Position à droite ou à gauche de la barre de titre

### Instance unique
- Plugin `tauri-plugin-single-instance` v2 (foundation, finalisé en 0.1.6)

## [0.1.4] - 2026-04-21

### Pochettes d'albums
- Recuperation automatique via l'API Deezer (menu contextuel album)
- Recherche manuelle via un popin premium avec grille de resultats
- Choix d'un fichier local comme pochette (file picker)
- Sous-menu "Changer de pochette" dans le menu contextuel album avec 3 options
- Batch "Pochettes d'albums" dans les settings pour traiter tous les albums sans pochette
- Filtre "Sans pochette" sur la page albums (toggle dans la FilterBar)

### Notation des morceaux
- Extraction automatique du tag POPM/Rating au scan (support des formats 0-5, 0-100, 0-255)
- Composant StarRating cliquable (5 etoiles vertes, hover preview, glow)
- Integration dans la vue liste compacte, la vue grille et la page detail du morceau
- Tri par notation dans la page morceaux (NULLs places en fin de liste)
- Sauvegarde en base avec update optimiste (UI reactive sans refresh)

### Navigation alphabetique
- Composant AlphabetNav reutilisable (A-Z + #) style ascenseur
- Scroll-to au clic sur une lettre (style Spotify/Apple Music)
- Lettres grisees pour celles sans entree
- Integre sur les pages albums, artistes et morceaux
- Bouton toggle dans la barre de vue pour afficher/masquer

### Filtres et tri
- Refonte premium de la FilterBar : select dropdown pour le tri + bouton direction ASC/DESC integre
- Filtre "Sans pochette" etendu aux morceaux
- Direction DESC par defaut pour notation, duree, date (plus intuitif)
- Memorisation du tri dans localStorage par contexte (morceaux / albums / artistes / genres)
- Layout uniformise avec hauteurs coherentes et dropdown flottant au clic

### Menu contextuel
- Sous-menu "Ajouter a une playlist" sur les menus contextuels (ajout groupe depuis les albums et selections multiples)
- Padding ajuste sur les sous-menus pour une hierarchie visuelle plus claire

### Settings
- Option "Simple clic = lecture" (single_click_play) sur les listes de morceaux

### Stabilite
- Double protection contre les crashs de scan : catch_unwind par fichier + par batch rayon complet
- Si un batch crash en parallele, fallback sequentiel automatique fichier par fichier
- Log des fichiers du batch problematique pour identifier le coupable

### Corrections
- Le tri etait reinitialise a chaque changement de bibliotheque sur la page morceaux

## [0.1.3] - 2026-04-07

### Selection multiple
- Mode selection sur les listes de morceaux (checkbox, barre d'actions flottante)
- Actions groupees : lire, ajouter a la file, ajouter a une playlist
- Boutons "Selectionner" / "Tout selectionner" dans la barre de filtres
- Support sur les 3 vues : cards, compacte et discographie

### Albums
- Bouton play au survol des covers dans la grille d'albums
- Bouton "..." (menu contextuel) au survol des covers
- Ajout de "Ajouter a une playlist" dans le menu contextuel album (ajout groupé de tous les tracks)
- Bouton "..." avec menu contextuel sur la page detail album
- Tri des morceaux par N° / Titre / Duree sur la page album
- Tri des "Autres albums" par Annee / Titre sur la page album
- Zoom lightbox sur la cover album (clic pour agrandir)
- Border-radius des covers reduit (rounded-lg)

### Artistes
- Tri de la discographie par Album / Annee / Titre
- Tri des albums par Annee / Titre
- Nom d'album + annee affiches dans la discographie
- Zoom lightbox sur la photo artiste
- Comptage correct des tracks pour les artistes featured (via library_track_artists)

### Queue
- Drag & drop fonctionnel via pointer events (reorganisation des morceaux)
- Effets visuels premium : ligne de drop emerald avec glow, opacite, scale
- Bouton play/pause dans le header de la queue
- Correction du style scrollbar

### Player
- Correction du bouton pause/play qui remettait le morceau a zero (resume au lieu de replay)

### Grilles responsives
- Ajout de breakpoints pour grands ecrans : 6 colonnes a 1536px+, 7 a 1800px+, 8 a 2200px+

### Stabilite
- Protection catch_unwind contre les crashs Symphonia sur fichiers corrompus/exotiques
- Ajout des extensions wav, aiff, opus, aac au scan de bibliotheque
- Changement de library redirige vers la meme section avec le nouveau library_id

### Configuration
- Categorie macOS corrigee : Music au lieu de DeveloperTool
- Support minimum macOS 10.15 (Catalina) pour Intel
- Licence "Free for personal use" dans la page A propos

## [0.1.2] - 2026-04-05

### Profils
- Ajout d'un bouton "Modifier" visible sous chaque profil dans la popin de selection
- Confirmation de suppression de profil : il faut taper "supprimer" pour confirmer
- La creation d'un profil ne quitte plus la popin et ne le selectionne plus automatiquement

### Page artiste — Performance
- Chargement progressif avec skeletons : le hero s'affiche instantanement, les sections se remplissent au fur et a mesure
- Nouvelles commandes backend ciblees : `get_albums_by_artist`, `get_similar_artists` (remplacent le chargement de TOUS les albums/artistes)
- Optimisation des requetes SQL : suppression des sous-requetes `OR IN (SELECT ...)` couteuses
- Ajout de 6 index SQLite manquants (`library_tracks.artist_id`, `library_albums.artist_id`, `library_albums.genre`, etc.)

### Playlists
- Ajout du bouton "Tout lire" sur la page playlist

### Recherche
- Correction du lien artiste dans les resultats de recherche (le `library_id` etait null)

### Feedback
- Remplacement du formulaire de contact (non fonctionnel) par une page avec lien mailto `contact@rustmusic.dev`
- Suppression des mentions a RiffFlow
- Mise a jour des traductions (FR, EN, ES, DE)

## [0.1.1] - 2026-03-30

### Linux
- Correction de l'affichage des covers sur Linux (contournement du bug WebKitGTK 2.50 avec le protocole `asset://`)
- Correction de l'encodage `%2F` dans les URLs asset sur Linux
- Ajout du support de compilation `.deb` et `.rpm`
- Correction du warning GTK `gtk_widget_get_scale_factor`

### Covers & Thumbnails
- Nouveau composant `CoverImg` avec chargement async et cache LRU (max 300 entrées, ~40MB)
- Commande Rust `read_cover_as_base64` pour servir les covers via IPC (fallback base64)
- Fonction `assetSrc()` pour corriger l'encodage des chemins Linux
- Support des tailles de miniatures (`full`, `1x`, `2x`) avec generation en background
- Reorganisation des covers : `covers/albums/` et `covers/artists/` avec sous-dossiers `full/1x/2x`
- Migration automatique des anciennes covers vers la nouvelle structure
- Redimensionnement rapide via `fast_image_resize` (SIMD) au lieu du crate `image`
- Generation a la volee des miniatures manquantes (`resolve_thumbnail`) avec fallback sur `full`
- Pool de threads dedie (50% des cores) pour la generation de miniatures en arriere-plan
- Double mode d'affichage : `asset` protocol (direct, rapide) ou `base64` IPC (fallback)
- Filtrage des images artistes par defaut de Deezer (detection du pattern URL sans hash)

### Images artistes
- Live update des images artistes : apparition en temps reel pendant le fetch Deezer
- Store reactif `artistImageReadyStore` avec event `artist-image-ready`
- Recuperation des images artistes en mode force (re-telecharge meme si le chemin existe en base)
- Sauvegarde des images artistes dans `covers/artists/full/` avec miniatures en background

### Import & Progression
- Nouveau composant d'import premium : cercle de progression avec glow, shimmer, ETA
- Estimation du temps restant en live pendant l'import
- StatusBar redesignee : gradient, shimmer, pourcentage, bouton d'annulation au hover
- Refresh automatique des donnees apres import (`libraryContentStore.refresh()`)
- Correction du freeze de 30s a l'annulation du dialogue d'import
- Loader d'import ajoute aux pages Genres et Explorateur de fichiers
- Migration des covers avec progression dans la statusbar

### Parametres
- Ajout du bouton "Ouvrir le dossier de donnees" (ouvre l'explorateur sur le dossier AppData)
- Notifications desactivees par defaut (option retiree des parametres, sera reintroduite plus tard)
- Traductions ajoutees (FR, EN, ES, DE) pour les nouvelles entrees

### UI
- Correction de la troncature des titres longs (noms de fichiers) dans toutes les vues (album, playlists, queue, player, stats, recherche)
- Ajout du `title` (tooltip au hover) pour afficher le nom complet des titres tronques

### Optimisation
- Tailles de covers adaptees par contexte : `1x` pour les listes, `2x` pour les grilles, `full` pour les pages detail
- Transaction SQL pour les migrations de covers (rollback en cas d'erreur)
- Generation des miniatures en 2 passes : deplacement + DB (rapide) puis resize en parallele (rayon)

## [0.1.0] - Initial Release

- Lecteur audio FLAC/WAV/MP3/OGG/OPUS
- Gestion de bibliotheques avec scan automatique
- Interface Svelte 5 + Tailwind CSS 4
- Mode clair / sombre
- File d'attente avec persistance SQLite
- Recherche globale
- Playlists et favoris
- Profils utilisateur
- Auto-update via Tauri updater
