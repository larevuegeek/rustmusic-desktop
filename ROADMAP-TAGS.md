# Roadmap — Gestion des tags

> Document de travail préparé le 2026-08-12, pour une session dédiée.
> Complète [ROADMAP.md](ROADMAP.md), qui reste la feuille de route générale.

L'objectif : passer d'un **éditeur de tags fichier par fichier** à un **atelier
de bibliothèque** — récupérer les métadonnées manquantes, corriger par lot,
et réorganiser fichiers et dossiers à partir des tags, sans jamais casser la
base ni perdre un fichier.

---

## Où on en est

Livré (commit courant) :

| Brique | État |
|---|---|
| Écriture MP3 / DSF / DFF / FLAC, texte + images | ✅ `core/audio_metadata/injector/` |
| `FieldEdit { Keep, Clear, Set }` | ✅ conçu pour l'édition multiple, jamais utilisé comme tel |
| `ImagePlan` — liste finale plutôt qu'opérations | ✅ |
| Écriture atomique, au fil de l'eau, lecture seule gérée | ✅ `atomic_write.rs` |
| Éditeur d'un fichier | ✅ `EditTagsPopin.svelte` |
| Deezer — **pochettes uniquement** | ✅ `search/artist`, `search/album` |
| Resynchronisation après édition | ✅ via `save_track_to_library` |

Ce que ça veut dire concrètement : **le moteur d'écriture est fait et testé**
(91 tests). Tout ce qui suit est de l'orchestration au-dessus de lui, pas de
la manipulation d'octets — sauf T8.

---

## Les trois contraintes qui structurent tout

Elles sont vérifiées dans le code actuel, pas supposées. Chaque phase y revient.

### 1. Six tables stockent un chemin absolu

```
library_files.path      library_cache.path (UNIQUE)
recent_files.path (UNIQUE)   track_liked.path (UNIQUE profil_id, path)
library_dirs.path       library.cover
```

Déplacer ou renommer un fichier sans les mettre à jour **orpheline
silencieusement l'historique d'écoute, les likes et le cache**. Les playlists,
elles, référencent `library_track_id` : elles survivent.

C'est la contrainte n°1 de T5 et T6. Un renommage doit être une transaction
qui couvre le système de fichiers **et** la base, ou ne pas avoir lieu.

### 2. L'écriture est synchrone dans une commande `async`

`write_track_tags` est `pub async fn` mais appelle `injector::apply` qui fait
des entrées/sorties bloquantes — mesuré à **2,6 s pour un FLAC de 34 Mo sur
SMB**. Sur un lot de 500 fichiers, ça bloque un worker tokio pendant vingt
minutes. À corriger **avant** T0, pas pendant.

### 3. Un lot raté est une catastrophe, pas un bug

Corriger 5 000 fichiers avec un motif erroné n'est pas rattrapable à la main.
D'où l'aperçu à blanc obligatoire (T4) et le journal d'annulation (T7), qui ne
sont pas du confort mais la condition pour oser lancer un lot.

---

## T0 — Socle : moteur de traitement par lot

**Objectif** — une brique unique qui exécute N opérations, rend compte, et
s'interrompt proprement. Tout le reste s'y branche.

**À faire**
- Sortir les entrées/sorties du runtime async (`tokio::task::spawn_blocking`
  ou un pool dédié). Contrainte n°2.
- Type `BatchJob` : identifiant, total, avancement, annulation, résultat
  **par élément** (`Ok` / `Err(raison)`).
- Événements Tauri : `batch-progress`, `batch-done`.
- Un lot ne s'arrête **jamais** au premier échec. Un fichier verrouillé au
  milieu de 500 ne doit pas annuler les 499 autres — il est signalé, on
  continue.
- Écran de compte rendu : ce qui a réussi, ce qui a échoué et pourquoi,
  avec la possibilité de relancer les seuls échecs.

**Pièges**
- L'annulation doit être vérifiée **entre** les fichiers, jamais au milieu
  d'une écriture atomique : couper un `replace_file_with` laisserait un
  temporaire orphelin.
- La concurrence est tentante mais un partage réseau sature vite. Deux ou
  trois fichiers en parallèle, pas seize.

**Effort** — 1,5 j

---

## T1 — Édition multiple

**Objectif** — sélectionner N morceaux, corriger l'album des N sans écraser
leurs N titres différents.

**Ce qui existe** — `FieldEdit { Keep, Clear, Set }` a été conçu exactement
pour ça. `Keep` signifie « ne touche pas », et c'est déjà la valeur par défaut
de tous les champs. Le backend est prêt, **rien à y changer**.

**À faire**
- L'éditeur accepte `paths: string[]` au lieu de `path: string`.
- Champs dont les valeurs diffèrent : afficher « valeurs multiples » en
  filigrane, laisser le champ vide, et ne l'envoyer **que s'il a été touché**.
  Le distinguer d'un champ vidé exprès (qui veut dire « efface ») demande de
  suivre l'intention, pas la valeur.
- Numérotation automatique : « numéroter de 1 à N dans l'ordre affiché »,
  et « renseigner le total » — les deux corvées les plus fréquentes.
- Les médias en édition multiple : appliquer **une** pochette à tout un album
  est le cas utile ; le reste (réordonner sur N fichiers) n'a pas de sens et
  doit être masqué plutôt que grisé.

**Pièges**
- Le champ « valeurs multiples » est le piège classique de tous les éditeurs
  de tags : un utilisateur clique dedans, ne tape rien, et le champ envoie
  une chaîne vide qui efface les N valeurs. L'intention doit être explicite.

**Effort** — 1,5 j

---

## T2 — Recherche et filtres de correction

**Objectif** — trouver ce qu'il y a à corriger. Sans ça, le traitement par
lot n'a pas de point d'entrée.

**À faire**
- Filtres dans la bibliothèque : sans pochette (existe déjà), **sans tag
  essentiel** (titre / artiste / album vide), **sans année**, **sans numéro
  de piste**, format non réinscriptible.
- Détection d'incohérences par album : années divergentes, artiste d'album
  absent alors que les pistes ont des artistes différents (compilation non
  déclarée), numéros en double ou trous dans la séquence.
- Doublons : même titre + même artiste + durée proche, sur des chemins
  différents.
- Vue « à corriger » qui agrège tout ça, avec sélection multiple → T1.

**Pièges**
- La détection de doublons sur la durée demande une tolérance (±2 s) :
  deux rips du même CD ne donnent pas la même durée à l'échantillon près.
- Ces requêtes portent sur toute la bibliothèque : prévoir les index, et
  mesurer sur une bibliothèque de 50 000 titres avant de livrer.

**Effort** — 2 j

---

## T3 — Deezer : récupération des métadonnées

**Objectif** — remplir les tags manquants depuis l'API publique, avec revue
avant application. Jamais d'écriture automatique.

**Ce qui existe** — `search/artist` et `search/album`, utilisés pour les
**pochettes uniquement**. Le client HTTP et la gestion d'erreur sont là.

**À faire**
- Nouveaux points d'accès : `/album/{id}` (renvoie la liste des pistes avec
  `track_position`, `disk_number`, `title`, `duration`), `/track/{id}`
  (`isrc`, `bpm`, `release_date`).
- **Appariement** local ↔ distant : c'est le cœur du sujet. Proposer un score
  par piste, combinant numéro de piste, similarité du titre (distance de
  Levenshtein normalisée) et écart de durée. Trois seuils : sûr, douteux,
  rejeté.
- Écran de revue : deux colonnes, valeur locale contre valeur proposée, case
  à cocher par champ **et** par piste. « Tout appliquer » ne concerne que les
  appariements sûrs.
- Application via T1 : la revue produit un `TagEdit` par fichier, le reste est
  déjà écrit.

**Ce que Deezer ne donnera pas** — à dire dans l'interface, pas à découvrir :
- pas de **compositeur**, pas de **parolier**
- **genre au niveau de l'album** seulement, et très grossier
- pas de numéro de disque fiable sur les coffrets
- couverture inégale hors variété internationale (classique, jazz de niche,
  rap français ancien)

Pour ces champs, c'est MusicBrainz qu'il faut (T8) — le dire tout de suite
évite de promettre ce que la source ne contient pas.

**Pièges**
- **Quota** : la documentation annonce 50 requêtes / 5 s par IP. À revalider
  au moment de coder. Prévoir une file avec limitation de débit dès le
  départ : un lot de 200 albums part en rafale sinon.
- **Cache** : un même album interrogé deux fois dans la même session ne doit
  pas repartir sur le réseau.
- Les accents et la ponctuation font échouer les recherches. Normaliser
  (minuscules, sans accents, sans ponctuation) **des deux côtés** avant
  comparaison — mais écrire la valeur distante telle quelle.
- Ne jamais écraser une valeur locale **non vide** sans coche explicite.

**Effort** — 3 j

---

## T4 — Moteur de motifs

**Objectif** — la brique commune à T5 et T6. Un motif, des tags, un chemin.

**Langage proposé**

```
{artist}/{album} ({year})/{track:02} - {title}.{ext}
```

- Champs : `artist`, `albumartist`, `album`, `title`, `year`, `genre`,
  `track`, `disc`, `composer`, `ext`
- Remplissage : `{track:02}` → `07`
- Optionnel : `[{disc}-]` — le groupe entier disparaît si le champ est vide,
  ce qui évite les `1-` orphelins sur les albums mono-disque
- Repli : `{albumartist|artist}` — prend le premier champ renseigné

**À faire**
- Analyse et validation du motif, avec message d'erreur utile.
- Assainissement par plateforme : `\ / : * ? " < > |` interdits sous Windows,
  `/` sous Unix ; noms réservés (`CON`, `PRN`, `AUX`, `NUL`, `COM1`…) ;
  point ou espace final interdits sous Windows.
- **Longueur de chemin** : 260 caractères sous Windows si les chemins longs
  ne sont pas activés. Une arborescence `Artiste/Album (Année)/07 - Titre`
  avec des noms de groupes de metal atteint la limite pour de vrai.
- Motifs prédéfinis (les trois ou quatre conventions courantes) + motifs
  enregistrés par l'utilisateur.
- **Aperçu à blanc** : la liste complète avant/après, avec les collisions et
  les dépassements signalés en rouge. Le bouton d'application reste
  inaccessible tant qu'un conflit subsiste.

**Pièges**
- Deux pistes qui produisent le même chemin cible (même titre sur un album
  avec des numéros absents) : détecter **avant**, pas au moment d'écraser.
- Un tag vide ne doit pas produire un dossier nommé `` ou `Unknown` en dur —
  laisser l'utilisateur choisir le repli.

**Effort** — 2 j

---

## T5 — Renommage des fichiers par lot

**Objectif** — appliquer un motif aux noms de fichiers, sans toucher à
l'arborescence.

**À faire**
- Renommage dans le dossier courant, via le moteur T4.
- Mise à jour de la base **dans la même transaction** : `library_files.path`,
  `library_cache.path`, `recent_files.path`, `track_liked.path`.
  Contrainte n°1.
- Ordre : renommer le fichier, puis mettre à jour la base, puis valider. Si
  la base échoue, renommer en sens inverse.

**Pièges**
- `library_cache.path` et `recent_files.path` sont **UNIQUE**. Un renommage
  qui produit un chemin déjà présent en base échoue sur la contrainte — le
  détecter à l'aperçu, pas à l'exécution.
- Le morceau **en cours de lecture** : la file d'attente garde l'ancien
  chemin. Soit on refuse de renommer un fichier en lecture, soit on met la
  file à jour. Refuser est plus honnête pour une v1.
- Sous Windows, renommer un fichier ouvert échoue — et sur un partage réseau,
  une simple lecture suffit (vérifié cette session). Le préchargement en RAM
  des chemins réseau règle le cas courant, pas tous.

**Effort** — 2 j

---

## T6 — Restructuration des dossiers

**Objectif** — le motif s'applique au chemin complet : les fichiers changent
de dossier, l'arborescence se recompose.

**À faire**
- Déplacement via le moteur T4, création des dossiers manquants.
- **Fichiers satellites** : `cover.jpg`, `folder.jpg`, `.lrc`, `.cue`,
  `.log`, `.m3u`, `.nfo` doivent suivre l'album. Un `.lrc` laissé derrière,
  ce sont les paroles perdues.
- Nettoyage des dossiers devenus vides, **jamais récursif au-delà de la
  racine de la bibliothèque**.
- Mise à jour de `library_dirs.path` en plus des quatre tables de T5.

**Pièges**
- **Changement de volume** : `rename` n'est plus atomique entre deux disques.
  Il faut copier, vérifier, puis supprimer — et ne supprimer qu'après
  vérification réussie.
- Un déplacement interrompu au milieu de 3 000 fichiers laisse la
  bibliothèque à moitié dans l'ancienne arborescence. D'où T7, qui devrait
  être livré **avec** T6 et non après.
- L'espace disque : une restructuration inter-volumes demande de la place
  pour la copie. Vérifier avant de commencer.

**Effort** — 3 j

---

## T7 — Annulation

**Objectif** — pouvoir revenir en arrière sur un lot.

**À faire**
- Table `batch_journal` : identifiant de lot, horodatage, type d'opération,
  et pour chaque élément l'état avant et après.
- Pour un renommage ou un déplacement : ancien et nouveau chemin suffisent.
- Pour une écriture de tags : conserver le **blob de tags d'origine** avant
  réécriture. Coûteux en place — le limiter aux N derniers lots, purge auto.
- Écran « historique des lots » avec bouton d'annulation.

**Pièges**
- L'annulation peut elle-même échouer (le fichier a bougé depuis). Elle doit
  rendre compte comme un lot normal, pas prétendre avoir tout remis en place.
- Ne pas proposer d'annuler un lot dont les fichiers ont été modifiés depuis :
  comparer les dates de modification.

**Effort** — 2 j

---

## T8 — Au-delà

Pas dans la même session, mais à garder en vue pour ne pas se fermer de portes.

| # | Sujet | Pourquoi |
|---|---|---|
| T8.1 | **MusicBrainz** | Compositeur, parolier, genre fin, coffrets, classique. Là où Deezer s'arrête. Modèle de données bien plus riche mais API plus exigeante (agent utilisateur obligatoire, 1 req/s). |
| T8.2 | **Empreinte acoustique (AcoustID / Chromaprint)** | Identifier un fichier sans aucun tag. C'est la seule réponse aux `track01.mp3`. Demande une bibliothèque native — première vraie dépendance externe du projet. |
| T8.3 | **Règles de nettoyage** | Casse des titres, espaces multiples, `feat.` normalisé, `(Remastered 2011)` déplacé en sous-titre. Beaucoup de valeur pour peu de code, une fois T0 et T1 en place. |
| T8.4 | **Import / export** | Sauvegarder les tags d'une bibliothèque en CSV ou JSON, les réappliquer. Filet de sécurité, et passerelle vers les autres logiciels. |
| T8.5 | **Écriture des paroles** | On sait les lire (LRCLIB, sidecar), pas les écrire dans le fichier (`USLT` / `LYRICS`). L'encodeur est déjà là. |

---

## Ordre proposé

```
T0 socle ──┬── T1 édition multiple ──┬── T3 Deezer
           │                          │
           └── T2 recherche ──────────┘
                    │
                    └── T4 motifs ── T5 renommage ── T6 dossiers
                                          └── T7 annulation ──┘
```

**Livrable minimal utile** : T0 + T1 + T2. Ça donne déjà « sélectionner tous
les morceaux sans année d'un album et la renseigner en une fois », ce qui
couvre l'essentiel du travail de correction.

**Ne pas livrer T6 sans T7.** Un déplacement de bibliothèque sans marche
arrière est une prise de risque disproportionnée.

Total T0 → T7 : **environ 15 jours**.

---

## Décisions à prendre avant de coder

À trancher en début de session, elles conditionnent la structure :

1. **Où vit l'atelier ?** Une popin ne suffira plus. Page dédiée
   (`/library/[id]/tags`) ou vue plein écran ?
2. **Le motif s'applique-t-il aux tags ou aux tags corrigés ?** Renommer
   d'après des tags qu'on vient de corriger dans le même lot suppose un
   enchaînement — ou deux passes.
3. **Refuser ou gérer le morceau en cours de lecture ?** Refuser est simple
   et honnête ; gérer demande de toucher à la file d'attente.
4. **Quelle profondeur d'annulation ?** Garder les tags d'origine coûte de la
   place ; les chemins seuls coûtent presque rien mais ne couvrent que T5/T6.
5. **Deezer seul, ou architecture à plusieurs sources dès le départ ?** Poser
   un trait `MetadataProvider` maintenant coûte peu et évite de tout reprendre
   à l'arrivée de MusicBrainz.
