# Contribuer à RustMusic

Merci de t'intéresser au projet ! Voici les éléments à connaître avant de proposer une contribution.

## Avant de commencer

1. **Vérifie que l'issue n'existe pas déjà** dans les [issues GitHub](https://github.com/larevuegeek/rustmusic/issues).
2. **Pour les changements significatifs** (nouvelle feature, refactor majeur), **ouvre d'abord une issue de discussion** avant d'écrire du code. Ça évite les déceptions si la direction n'est pas alignée.
3. **Les petits fix** (bug, typo, traduction, doc) peuvent aller directement en PR.

## Style de code

### Rust

- `cargo fmt` avant chaque commit
- `cargo clippy` doit passer sans warning nouveau
- Pas d'`unwrap()` / `expect()` dans le code de production sauf justification claire (commentaire)
- Préférer `?` et les `Result` propagés
- Logs : `log::debug!` pour le verbeux, `log::info!` pour les événements clés (init, shutdown), `log::warn!` pour les erreurs récupérables, `log::error!` pour les erreurs graves
- Modules organisés : `core/` pour la logique, `commands/` pour les Tauri commands, `repository/` pour SQLite, `mapper/` pour les conversions

### TypeScript / Svelte

- Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`) — pas de `let` réactif old-school
- Pas de `any` sauf nécessité absolue, et alors avec un commentaire qui l'explique
- Tailwind 4 canonical classes (`bg-linear-to-br` et non `bg-gradient-to-br`, etc.)
- Composants en PascalCase, fichiers en `PascalCase.svelte`
- Services en `camelCase.service.ts`, stores en `camelCase.store.ts`

### Internationalisation

- **Toute string visible utilisateur doit passer par `$t('key')`**
- Ajoute la clé dans les **5 locales** (`fr.json`, `en.json`, `de.json`, `es.json`, `it.json`) — un fichier non synchronisé est rejeté
- Garde la structure hiérarchique cohérente (`settings.audio_quality_*`, etc.)

### SQL

- Les migrations vont dans `src-tauri/SQL/` avec format `YYYYMMDDHHMM_description.sql`
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
- `cargo check` doit passer sans erreur (warnings tolérés)
- `npx svelte-check --threshold error` doit afficher **0 erreur**
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

- Changement de stack majeur (passer de Tauri à Electron, de SvelteKit à React, etc.) sans discussion préalable
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
