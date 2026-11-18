# Contribuer à RustMusic

Merci de t'intéresser au projet ! Voici les éléments à connaître avant de proposer une contribution.

## Avant de commencer

1. **Vérifie que l'issue n'existe pas déjà** dans les [issues GitHub](https://github.com/larevuegeek/rustmusic-desktop/issues).
2. **Pour les changements significatifs** (nouvelle feature, refactor majeur), **ouvre d'abord une issue de discussion** avant d'écrire du code. Ça évite les déceptions si la direction n'est pas alignée.
3. **Les petits fix** (bug, typo, traduction, doc) peuvent aller directement en PR.

## Style de code

### Rust

- `cargo fmt` avant chaque commit
- `cargo clippy` doit passer sans warning nouveau
- Pas d'`unwrap()` / `expect()` dans le code de production sauf justification claire (commentaire)
- Préférer `?` et les `Result` propagés
- Les commentaires décrivent le code tel qu'il est : pas de renvoi à l'ancienne interface Svelte (`Foo.svelte`, `foo.store`, `bar.ts`)
- Logs : `log::debug!` pour le verbeux, `log::info!` pour les événements clés (init, shutdown), `log::warn!` pour les erreurs récupérables, `log::error!` pour les erreurs graves
- `src/` ne contient que du code Rust, organisé comme un projet Symfony :
  - `entity/` : les tables ; `repository/` : les requêtes SQL ; `query/<domaine>/` : ce que renvoient les lectures (`…Row`) ;
  - `core/` et `service/` : la logique du moteur ; `commands/` : les opérations appelées par l'interface ;
  - `controller/<domaine>/` : les actions (réagir aux messages, charger les données), un dossier par domaine et rien à la racine. `layout/` réunit ce qui entoure les pages (navigation, popins, dialogues, notifications, bandeaux). Dans un domaine, les fichiers portent son nom et leur rôle : `albums/album_list.rs` (la liste), `album_detail.rs` (la fiche), `playlist_form.rs`, `podcast_menu.rs`… Le lecteur (`player/` : file, paroles, mini-lecteur, lecture) garde des noms simples ; l'aiguillage des messages (`Msg`) vers les domaines est dans `controller/handle/` ;
  - `models/<domaine>/` et `mapper/<domaine>/` suivent les mêmes domaines que `controller/` (`albums/`, `tracks/`, `tags/`, `layout/`…) ;
  - `models/<domaine>/` : les types de l'interface, **un fichier par type, rien d'autre qu'une définition** (messages `…Msg`, état d'une page, ce qu'affiche un gabarit `…View`) ;
  - `mapper/<domaine>/` : les conversions (lignes lues et état d'une page → ce qu'affichent les gabarits), fichiers nommés comme leur contrôleur (`mapper/albums/album_detail.rs` pour `controller/albums/album_detail.rs`). Un gabarit appelle une méthode de `MusicApp` (`self.album_view()`) ; les calculs (filtres, tris, regroupements) sont des fonctions sans état à côté d'elle (`filter_albums`, `album_groups`…), testées dans le `tests.rs` du domaine ;
  - `component/<domaine>/` : les composants des gabarits (`<PlayButton>`, `<SidebarRow>`…). Le `.rs` ne déclare que le contrat : la struct dont le nom devient la balise, ses attributs et leurs valeurs par défaut, et l'appel à son gabarit. L'apparence est dans le `.ui`. `shared/` regroupe les composants utilisés par tous les écrans, rangés par famille (`buttons/`, `menus/`, `chips/`, `inputs/`, `display/`) ; le gabarit suit le même chemin (`ui/templates/shared/menus/MenuItem.ui`). Les autres dossiers suivent les domaines avec préfixe (`sidebar/sidebar_row.rs`, `settings/settings_toggle_switch.rs`) ;
  - `app/` : `platform/` (poignée `AppHandle`, runtime tokio, dossiers, pont vers le moteur, icône de notification), `resources/` (polices, icônes, traductions, CSS embarqués), `launch/` (ouverture de la fenêtre, aperçu hors écran), `music_app/` (la fenêtre, `MusicApp`) ;
  - `helper/` : outils génériques.
- Un `mod.rs` ne contient que des déclarations (`mod`, `pub use`, documentation du module).

### Interface (karbon-ui)

- Le thème vit hors du code, dans `ui/` à la racine : `ui/templates/` (gabarits `.ui`), `ui/assets/` (CSS, icônes, polices, images), `ui/translations/`. On peut travailler sur l'interface sans ouvrir `src/`.
- Un gabarit lit des champs déjà préparés, envoie des messages et appelle `t()` / `t_with()` ; la mise en forme (nombres, durées, pluriels, valeurs par défaut) se fait dans `mapper/`, qui remplit une struct `…View` de `models/` (exemple : `QueuePanelView` et `mapper/player/queue.rs`).
- Aucun texte visible écrit en dur dans un gabarit : `title={t("clé")}`, `{t("clé")}`.
- Une valeur optionnelle s'affiche avec `{#if let Some(x) = …}`, pas avec `{#each x.iter() …}`.
- Une valeur se nomme avec `{@const x = …}` (visible jusqu'à la fin du bloc), jamais avec une boucle d'un seul tour `{#each [x] as y (0usize)}`.
- Un gabarit ne lit jamais `self.data` : il passe par un accesseur nommé de `MusicApp` (`self.volume()`, `self.show_favorites()`…, dans `mapper/app/app_state.rs`).
- Pas de commentaires `<!-- … -->` dans les gabarits.
- Le code non plus n'écrit pas de texte visible en dur : `mapper/` passe par les traductions (`t()`, `t_with()`, `helper::date` pour les dates).
- Les macros lisent les fichiers du thème depuis la racine du paquet : `ui_file!("/ui/templates/…")`, `css_file!(…, "/ui/assets/css/app.css")`.
- Tailwind 4 canonical classes (`bg-linear-to-br` et non `bg-gradient-to-br`, etc.)
- Gabarits en `PascalCase.ui`, rangés comme `src/` : un dossier par domaine (`ui/templates/albums/`, `tags/`, `layout/`…), nom préfixé par le domaine (`AlbumList.ui`, `AlbumDetail.ui`, `TagEditorField.ui`) ; le gabarit d'un composant porte le nom de sa balise (`shared/menus/MenuItem.ui` pour `<MenuItem>`)

### Internationalisation

- **Toute string visible utilisateur doit passer par `t("key")`**, `t_with("key", &[("n", …)])` ou `t_count(n, "key_one", "key_n")` (`karbon_ui::intl`, catalogue installé par `src/app/resources/translations.rs`)
- Ajoute la clé dans les **5 fichiers** de `ui/translations/` (`fr.json`, `en.json`, `de.json`, `es.json`, `it.json`) — un fichier non synchronisé est rejeté
- Garde la structure hiérarchique cohérente (`settings.audio_quality_*`, etc.)

### SQL

- Les migrations vont dans `SQL/` avec format `YYYYMMDDHHMM_description.sql`
- Toujours ajouter un index si tu fais un `WHERE` sur un champ non-PK
- Préférer les `LEFT JOIN` explicites aux sous-requêtes `OR IN (SELECT...)` coûteuses
- Documenter le pourquoi en commentaire dans le SQL si non-évident

## Conventions de commit

Pas de convention stricte type "Conventional Commits", mais en français de préférence (le projet est francophone à l'origine), avec un préfixe scope :

```
Bibliothèque : ajoute le filtre "Sans pochette"
Player : fix le pause qui remettait le morceau à zéro
DSD : refactor du convertisseur DSD2PCM
i18n : ajoute les traductions DE pour la section Audio
DLNA : corrige childCount=0 sur Onkyo
```

Les commits multi-lignes décrivant le **pourquoi** sont les bienvenus pour les changements non triviaux.

## Tests

Le projet n'a pas (encore) de suite de tests automatisée complète. Pour le moment :
- `cargo check` doit passer sans erreur (warnings tolérés), `cargo test` sans échec
- Test manuel des changements UI sur Windows + Linux (Zorin / Ubuntu) + macOS si dispo

Si tu touches au moteur audio (DSD, resampler, player), valide :
- Un FLAC 44.1k (cas standard)
- Un FLAC 96k ou 192k (resampling actif)
- Un DSF DSD64 stéréo (chemin DSD)
- Un fichier avec tags exotiques (compilation, multi-artist, etc.)

## Process de PR

1. Fork le repo
2. Branche depuis `main` avec un nom descriptif (`feat/playlist-export`, `fix/dsd-multichannel-crash`)
3. Commits atomiques et clairs
4. Push, ouvre la PR vers `main`
5. Décris **le pourquoi**, pas seulement le quoi (le code montre déjà le quoi)
6. Coche la case "Allow edits from maintainers" pour qu'on puisse rebaser / ajuster si besoin

## Ce que je n'accepterai probablement pas

- Changement de stack majeur (remplacer karbon-ui par une interface web, etc.) sans discussion préalable
- Ajout de dépendances JS / Rust lourdes pour des features mineures
- Code mort (fonctions non appelées, imports inutilisés) — `cargo machete` et `unused-imports` doivent rester propres
- Reformatage cosmétique massif d'un fichier où tu touches 1 ligne (les diff énormes empêchent le review)
- Features qui nécessitent un compte / une API tierce payante sans alternative locale

## Ce que j'accepte avec joie

- Traductions (DE et ES sont les moins couvertes historiquement)
- Fixes de bugs avec reproduction claire
- Améliorations de performance mesurées (avant/après)
- Documentation, exemples, captures d'écran
- Compatibilité avec des plateformes / formats audio non testés
- Refactors qui simplifient sans casser

## Licence des contributions

En soumettant une PR, tu acceptes que ton code soit publié sous **GPL-3.0** comme le reste du projet. Tu gardes ton copyright mais le projet est licencié GPL.

## Questions

- Tag-moi dans l'issue : [@larevuegeek](https://github.com/larevuegeek)
- Mail : contact@rustmusic.dev

Merci de contribuer à RustMusic ! 🎵
