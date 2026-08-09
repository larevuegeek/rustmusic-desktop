# Politique de sécurité

## Signaler une faille

**N'ouvre pas d'issue publique pour une faille de sécurité.** Une issue est
visible de tous, y compris de quelqu'un qui voudrait en profiter avant qu'un
correctif existe.

Deux canaux privés :

1. **GitHub Security Advisories** (préféré) — onglet *Security* du dépôt,
   « Report a vulnerability ». La discussion reste privée jusqu'à publication.
2. **Courriel** — contact@rustmusic.dev, avec `[SECURITY]` en objet.

## Ce qui aide à traiter vite

- La version de RustMusic et le système d'exploitation
- Ce qu'un attaquant obtiendrait concrètement (lecture de fichiers, exécution
  de code, accès réseau…)
- Les étapes pour reproduire, ou un fichier d'exemple si la faille vient d'un
  contenu malformé
- Si tu en as un : un correctif proposé, même partiel

## Délais

Le projet est développé par une seule personne sur son temps libre — les
délais ci-dessous sont un engagement de bonne foi, pas un contrat de support.

| Étape | Délai visé |
|---|---|
| Accusé de réception | 72 heures |
| Première évaluation | 7 jours |
| Correctif pour une faille critique | dès que possible, publication anticipée si nécessaire |

Tu seras crédité dans l'avis de sécurité et le changelog, sauf si tu préfères
rester anonyme.

## Versions suivies

Seule la **dernière version publiée** reçoit des correctifs. Le projet est en
version 0.x : il n'y a pas de branche de maintenance.

## Surface d'attaque connue

Quelques éléments utiles pour orienter une recherche :

- **Parsers de fichiers audio maison** (ID3v2, DSF, DFF) : ils lisent des
  données non fiables. Un fichier malformé ne doit jamais provoquer autre
  chose qu'une erreur propre — un dépassement de tampon ou une panique
  exploitable est une faille.
- **Serveur DLNA/UPnP** : il écoute sur le réseau local quand il est activé.
  Le parcours de la bibliothèque et le streaming exposent des chemins de
  fichiers ; toute possibilité de sortir du dossier de la bibliothèque est
  une faille.
- **Serveur HTTP local des pochettes** (contrôles média système) : lié à
  `127.0.0.1` sur un port éphémère, il sert des images depuis le dossier de
  couvertures.
- **Récupération de pochettes et de paroles** (Deezer, LRCLIB) : réponses
  distantes non fiables.
- **Auto-updater** : les mises à jour sont signées ; un contournement de la
  vérification de signature est critique.

## Hors périmètre

- Le fait que RustMusic lise les fichiers audio que l'utilisateur lui indique
- L'avertissement SmartScreen sous Windows (absence de signature de code,
  connue et documentée)
- Les vulnérabilités des dépendances sans chemin d'exploitation démontré dans
  RustMusic — signale-les quand même, mais en issue publique classique
