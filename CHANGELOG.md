# Changelog

## [0.4.0] - à venir

Votre musique hébergée chez vous, les podcasts et les radios arrivent dans
RustMusic. Serveurs : Navidrome (et tout serveur Subsonic), Jellyfin et les
serveurs DLNA des NAS deviennent des sources, chacune avec ses
bibliothèques, ses playlists, ses titres aimés et son historique ; la
lecture démarre tout de suite et les albums choisis restent disponibles
hors ligne. Podcasts : recherche dans l'annuaire Apple ou par adresse de
flux RSS, classement des émissions les plus écoutées par pays, écoute
pendant le téléchargement, reprise là où on s'était arrêté. Radios : des
milliers de stations en direct, une pause qui ne fait rien perdre (différé
jusqu'à 30 minutes) et le titre en cours. La barre latérale s'organise à la
main, et un mégaphone présente les nouveautés de chaque version.

### Serveurs de musique (bêta)
- **Ajouter un serveur** (Réglages → Stockage → Serveurs) : Navidrome et
  tout serveur compatible Subsonic (Gonic, Airsonic…), Jellyfin (mot de
  passe ou clé d'API), ou serveur DLNA du réseau (MiniDLNA, NAS…), trouvé
  tout seul ou par son adresse. Les dossiers choisis deviennent des
  bibliothèques ; d'autres s'ajoutent plus tard par « Dossiers… ». Le mot de
  passe n'est jamais enregistré : seulement une empreinte, un jeton de
  session ou la clé d'API.
- **Synchronisation** du catalogue (titres, albums, artistes, pochettes),
  comme un scan : seul ce qui a changé est réécrit.
- **Lecture immédiate** : le morceau joue pendant qu'il arrive, les sauts
  restent rapides, le suivant se prépare d'avance. Qualité par serveur :
  fichier d'origine (bit-perfect possible, DSD compris) ou MP3 320, 192 ou
  128 kbit/s transcodé par le serveur. Un cache de 2 Go garde les derniers
  morceaux écoutés.
- **Hors ligne** : « Garder hors ligne » sur un morceau, un album, un
  artiste, un genre ou une playlist du serveur ; ils restent jouables
  serveur éteint.
- **Retour au serveur** : J'aime, notes, écoutes et « en cours de lecture »
  lui sont renvoyés (Subsonic ; favoris, écoutes et en cours pour
  Jellyfin) ; serveur injoignable, ils partent plus tard.
- **Playlists du serveur** (Subsonic et Jellyfin) : importées, modifiables
  dans les deux sens, créées sur le serveur depuis RustMusic. Elles ne
  s'épinglent dans la barre latérale que si on le demande, serveur par
  serveur (Navidrome importe aussi les `.m3u` des albums).
- **Paroles du serveur** quand il en a, avant la recherche en ligne.
- **Serveur injoignable ou désactivé** : ses morceaux sont sautés à
  l'enchaînement ; un serveur DLNA qui a changé d'adresse est retrouvé tout
  seul sur le réseau.
- **Retirer un serveur** (en tapant SUPPRIMER) efface son catalogue, ses
  traces et son cache, arrête sa lecture en cours ; rien n'est touché sur le
  serveur.

### Sources
- **Bouton « Source »** en haut de la barre latérale : « Cet ordinateur »,
  chaque serveur, ou « Toutes les sources ». Chaque source a ses
  bibliothèques, ses playlists, ses titres aimés, son historique, sa
  recherche et son accueil ; changer de source ne coupe pas la lecture. Le
  bouton n'apparaît qu'avec un serveur : sans serveur, rien ne change.
- **Une playlist appartient à une source** et n'accepte que ses morceaux.
  Dans une source serveur, on choisit de la créer sur le serveur ou dans
  RustMusic seulement ; la page Playlists les range en deux groupes.
- **« Toutes les sources »** : un album ou un morceau présent en local et
  sur un serveur n'est qu'un résultat de recherche, avec un disque et un
  nuage pour lire ou ouvrir l'une ou l'autre version (la version préférée
  se règle).

### Recherche
- **Page de résultats refaite** : meilleur résultat, morceaux (qualité et
  durée), albums et artistes, filtres par type, « Tout lire ».
- La recherche ne fouille plus que les bibliothèques du profil actif.

### Podcasts
- **Nouvelle section « Écoute en ligne »** dans les réglages : les podcasts
  s'y activent (désactivés par défaut). On y choisit aussi le mode d'écoute
  et le pays de l'annuaire, et on y voit la place prise par les
  téléchargements et ce qui passe par le réseau.
- **Trouver une émission** : recherche dans Apple Podcasts, ou adresse d'un
  flux RSS collée telle quelle. « Les plus écoutés » suit le classement
  Apple du pays choisi (drapeaux dans le menu ; par défaut celui de
  l'ordinateur), par catégorie, triable par classement, nom ou dernier
  épisode.
- **Ouvrir une émission** : un clic sur une émission de l'annuaire ouvre sa
  page, avec ses épisodes et la lecture (l'abonnement se fait au passage).
  Le clic droit ou le menu « … » d'une carte propose un **aperçu** sans
  s'abonner (fiche et derniers épisodes, en deux colonnes), l'abonnement ou
  le désabonnement, et les favoris.
- **Accueil des podcasts** : « À écouter » (un épisode par émission, celui
  commencé ou le plus récent), « Mes podcasts » triable par dernier épisode,
  nom, date d'abonnement ou nouveaux épisodes.
- **Fiche d'une émission** : épisodes rangés par mois, plus récents ou plus
  anciens d'abord, filtre « Non écoutés », carte « Reprendre l'écoute »,
  tout marquer comme écouté, lien vers le site.
- **Écoute pendant le téléchargement** : la lecture démarre au bout de
  quelques secondes, et un saut loin devant ne fait pas attendre la fin du
  téléchargement. Au choix, l'écoute après téléchargement complet. Vers un
  lecteur réseau UPnP, l'épisode est d'abord téléchargé en entier.
- **Reprise et suite** : un épisode reprend là où on l'avait laissé, passe
  « écouté » à la fin, et les épisodes non écoutés qui suivent prennent la
  suite. « Lire ensuite » et « Ajouter à la file » placent un épisode dans
  la file d'attente. Les épisodes n'entrent pas dans « Récemment joués » et
  n'ont pas de bouton favori.
- **Téléchargements** : une page fait le bilan des épisodes gardés sur
  l'ordinateur (nombre, place prise, épisodes déjà écoutés et place
  récupérable, barre de l'espace par émission) ; on y supprime un épisode,
  une émission, les épisodes écoutés ou tout. Les fichiers portent un nom
  lisible (`Émission/2026-10-21 Titre.mp3`).
- **Favoris** : une étoile épingle une émission dans la barre latérale,
  sous « Podcasts », et en tête de « Mes podcasts ».
- Les nouveaux épisodes sont cherchés en arrière-plan ; « Tout vérifier »
  relit tous les flux à la demande.
- **File d'attente des épisodes**, à part de la musique : l'épisode suivant
  de la même émission prend la suite.
- **Téléchargement automatique**, émission par émission, des nouveaux
  épisodes ; les épisodes écoutés s'effacent après 24 h (réglable).
- **OPML** : importer ou exporter ses abonnements depuis une autre appli.

### Radios
- **Activables dans Réglages → Écoute en ligne** (désactivées par défaut),
  avec une entrée « Radios » dans la barre latérale, illustrée du logo de
  la dernière radio écoutée.
- **Trouver une station** dans l'annuaire libre Radio Browser : recherche
  par nom, « Les plus écoutées » du pays choisi, par genre (Info, Pop, Rock,
  Jazz, Classique…). On peut aussi coller l'adresse d'un flux (.mp3, .m3u,
  .pls) : la station est reconnue par l'annuaire quand il la connaît (nom,
  logo, genres). Une station listée plusieurs fois n'apparaît qu'une fois,
  dans sa meilleure version ; un flux AAC+, que le décodeur ne lit qu'en
  qualité réduite, est signalé.
- **Mes radios et Dernières écoutées** : l'étoile garde une station en
  favorite ; les 30 dernières radios lancées sont retenues. Le menu « … »
  (ou le clic droit) propose aussi le site de la radio et l'adresse du
  flux.
- **Écoute en direct** : le lecteur passe en mode radio, avec le logo, la
  station, le titre en cours quand la station l'annonce, et une bande « EN
  DIRECT » à la place de la barre de progression ; Précédent et Suivant
  passent d'une radio favorite à l'autre. Le mini-lecteur suit.
- **Pause en différé** : pendant une pause, la radio continue d'arriver
  (jusqu'à 30 minutes gardées) et la lecture reprend là où on l'avait
  laissée. « Revenir au direct » rattrape le retard. La pause compte d'où
  qu'elle vienne : bouton, clavier, touches média, minuterie de sommeil.
- **Coupure réseau** : la radio se reconnecte d'elle-même ; perdue pour de
  bon, un message le dit.
- **Une seule source à la fois** : lancer une radio arrête la musique, dont
  la file reste intacte ; relancer un morceau coupe la radio. L'ordinateur
  ne se met pas en veille pendant qu'une radio joue.
- **Qualité du flux** : quand une station en propose plusieurs, le menu
  « … » permet de choisir (AAC, MP3, débit). Les flux HLS (Radio France…)
  sont lus.
- **Titres passés** : l'historique des titres entendus sur une station, à
  vider quand on veut.
- **Sur un lecteur réseau** UPnP ou Sonos, la radio part comme un morceau.

### Nouveautés
- **Mégaphone dans la barre du haut**, à droite de la minuterie : les
  nouveautés de la version en quelques diapos, avec un bouton « Essayer »
  qui mène à chacune. Elles s'ouvrent d'elles-mêmes au lancement après une
  mise à jour, jusqu'à ce qu'on coche « Ne plus afficher au démarrage » ;
  une pastille signale celles qu'on n'a pas encore vues. La carte de
  soutien attend 30 s après leur fermeture.

### Barre du haut
- **Mégaphone et bouton de thème masquables**, comme le minuteur de veille
  (Réglages → Général → Intégration système).

### Barre latérale
- **Organiser la barre latérale** (menu « … » des Playlists) : les
  sections Playlists, Écoute en ligne et Épinglés se déplacent, comme les
  playlists épinglées, « Titres likés » et « Récemment joués », par leur
  poignée ou par les flèches. L'ordre est retenu.
- **Carte « Épinglez vos favoris »** : sa croix la ferme, et le choix est
  retenu. Elle masquait jusqu'ici les sections d'albums et d'artistes
  épinglés.

### Lecture
- **Historique par profil** : « Récemment joués » et les positions de
  reprise sont propres à chaque profil (ils étaient partagés).
- **Fin de la file** : la lecture s'arrêtait sur un morceau resté « en
  lecture ». Elle s'arrête maintenant et la barre de progression revient au
  début.
- Un seul morceau dans la file en répétition « Tout » est bien rejoué.

## [0.3.1] - à venir

La télécommande s'ouvre aux amplis Denon, Marantz et Yamaha, et retrouve un
ampli qui a changé d'adresse. Les réglages de sortie s'appliquent tout de
suite, sans relancer l'appli, et une sortie qui lâche en pleine lecture est
rouverte d'elle-même. L'ampli enchaîne enfin les morceaux servis par le
serveur DLNA.

### Réseau
- **Serveur DLNA : enchaînement des morceaux** d'un album ou d'un dossier.
  Interrogé sur un morceau, le serveur répondait par un dossier vide sans
  fichier : l'ampli s'arrêtait après le premier morceau et ne trouvait pas
  la pause. Il décrit maintenant le morceau (fichier, format, pause et saut
  permis), et un album ou un dossier annonce son vrai nombre de morceaux.

### Télécommande d'ampli
- **Denon, Marantz et Yamaha (MusicCast)** sont trouvés sur le réseau comme
  les Onkyo, Pioneer et Integra, avec les mêmes réglages : marche, volume au
  demi-décibel, entrées avec leurs noms personnalisés, modes, graves et
  aigus, zone 2. Sur l'entrée réseau, la liste et la lecture en cours
  s'affichent ; chez Yamaha, avec la pochette. Sous le nom de l'ampli, son
  modèle (« SR6015 »).
- **Ampli retrouvé après un changement d'adresse** sur le réseau (nouveau
  bail de la box) : la télécommande le cherche d'elle-même et garde sa
  nouvelle adresse. La recherche trouve aussi les amplis qui ne répondent
  pas aux annonces diffusées : elle interroge chaque adresse du réseau local.
- **Molette façon iPod** pour la navigation : un appui sur l'anneau pour une
  direction, un tour du doigt (ou la molette de la souris) pour défiler, OK
  au centre. Au choix, une ligne de touches plus compacte la remplace.
- Une longue liste de l'écran réseau défile dans sa case et suit le curseur,
  au lieu d'allonger le panneau.

### Bibliothèque
- **Sélection** : la même case à cocher partout. Dans les listes de
  morceaux (onglet Morceaux, fiches album, artiste, genre, période et
  playlist), elle apparaît même quand la colonne « # » est masquée : la
  colonne revient le temps de la sélection. Un seul bouton « Sélectionner »
  par fiche, celui de la barre du haut, et toujours en toutes lettres.
- **Import plus rapide**, surtout depuis un NAS : la pochette d'un dossier
  (cover.jpg…) est cherchée et lue une fois par dossier, et non plus pour
  chacun de ses morceaux, et chaque fichier n'est plus ouvert qu'une fois.
  Mesuré sur 10 000 FLAC en partage réseau : 15 fichiers par seconde au lieu
  de 1,6. Un rescan ne relit plus les fichiers inchangés.
- **Dossier introuvable au scan** (partage réseau non monté, disque
  débranché) : il n'est plus scanné comme vide, ce qui remettait ses totaux à
  zéro. Il est marqué « Inaccessible », un message le signale et ses morceaux
  sont conservés.
- **Partage réseau remonté sous un autre nom** (le NAS monté par son adresse,
  puis par son nom, ou l'inverse) : la bibliothèque ne trouvait plus rien.
  RustMusic retrouve le même partage et propose de mettre la bibliothèque à
  jour ; morceaux, playlists, favoris et file d'attente suivent.
- **Barre d'état lisible pendant un import** : un rescan affichait deux fois
  la même progression (« Rescan » et « Import en cours »), et les pastilles
  serrées faisaient chevaucher le chemin du fichier et la pastille DSP. Une
  seule pastille suit maintenant le rescan, avec le fichier en cours ; quand
  la place manque, le nom du fichier se tronque d'abord, puis le libellé,
  sans rien recouvrir.

### Lecture
- **Sortie audio perdue en pleine lecture** (câble USB capricieux, DAC
  réinitialisé, format changé dans Windows) : en mode partagé, le son se
  coupait jusqu'au morceau suivant alors que le compteur continuait. La sortie
  est maintenant rouverte d'elle-même et le morceau reprend là où il s'était
  coupé. Si elle ne revient pas en 5 s, la lecture passe sur la sortie du
  système, avec un message ; sans aucune sortie, elle s'arrête et le dit. Le
  morceau suivant retente la sortie choisie.
- **Réglages de sortie appliqués tout de suite** : changer de sortie, passer
  en exclusif ou en partagé, activer le DSD natif ou changer le profil de
  qualité relance le morceau en cours au même endroit, avec la nouvelle
  configuration. Jusque-là, rien ne changeait avant un morceau lancé à la
  main, et pas même au morceau suivant quand ils s'enchaînaient sans blanc :
  il fallait souvent relancer l'appli. En pause, le changement s'applique à
  la reprise.
- **Linux, mode exclusif** : le DAC, retiré à PipeWire le temps de la
  lecture, disparaissait de la liste des sorties, qui ne montrait plus que
  « Dummy Output ». Il y reste désormais, sélectionné.
- Quitter un lecteur réseau pour une sortie locale reprend directement sur
  la sortie choisie, et non sur la précédente.
- **Linux : son retrouvé sans cookie PulseAudio** (PipeWire seul, paquet
  Flatpak) : RustMusic ne joignait pas le serveur son et passait par la
  sortie ALSA « default », qui pouvait rester muette. Il se connecte
  maintenant au serveur son dans tous les cas.

### Soutien
- **Un merci après une mise à jour** : une petite carte, une fois par
  nouvelle version et jamais à la première installation, propose de soutenir
  le projet ; « Plus tard » ou « Ne plus afficher ».
- **Réglages → À propos → Soutenir RustMusic** : Patreon, Ko-fi, PayPal et
  une étoile sur GitHub.

### Divers
- **Licence corrigée dans À propos** : le texte affiché disait « Tous droits
  réservés » et interdisait la modification, à l'opposé de la licence réelle
  du projet. Il présente maintenant la GNU GPL v3 (utiliser, étudier,
  modifier, redistribuer sous la même licence), avec le lien vers le texte
  complet, dans les 5 langues.
- **Linux : description des paquets** : le .deb affichait « Description:
  (none) » ; les logithèques montrent maintenant une description de
  RustMusic.
- **Windows : RustMusic dans winget** (paquet `LaRevueGeeK.RustMusic`) :
  `winget install rustmusic` l'installe, `winget upgrade` le tient à jour.
- **Linux : paquet Flatpak** (`dev.rustmusic.RustMusic`), préparé pour
  Flathub. Les partages réseau ouverts dans le gestionnaire de fichiers sont
  lus en direct, la veille est retenue pendant la lecture par le portail du
  bureau, et la sortie exclusive et le DSD natif sont masqués quand le bac à
  sable n'a pas accès à la carte son. La pause automatique avant la mise en
  veille n'existe pas dans ce paquet. En rendu automatique, DMA-BUF est
  coupé et les pages sont dessinées par le processeur : avec le WebKitGTK
  du runtime, des éléments de l'interface clignotaient. Le GPU garde
  l'assemblage de la fenêtre, et l'interface sa netteté en haute densité.
  L'icône de la barre système s'affiche (trois points à sa place : l'image
  était écrite là où le bureau ne la voyait pas).
- **Logo net sur écran haute densité** : l'icône de la fenêtre et du dock
  était tirée de l'image de 32 px, floue à 200 %. Elle part maintenant de
  l'image de 512 px, que les paquets .deb et AppImage installent aussi.
- **Linux : démarrage sous Wayland sans XWayland** en rendu logiciel : X11
  n'est plus imposé quand il n'est pas disponible.

## [0.3.0] - 2026-10-14

RustMusic sort de l'ordinateur : la musique se lit sur les lecteurs, amplis
et appareils AirPlay de la maison. L'atelier de tags est refait pour être plus
simple, et va chercher ses valeurs sur MusicBrainz autant que sur Deezer.

### Réseau
- **Lire sur un lecteur réseau** (UPnP/DLNA, en bêta), à activer dans Réglages →
  Réseau (désactivé par défaut : sans lui, RustMusic ne cherche aucun appareil
  sur le réseau). WiiM, amplis et lecteurs réseau (Onkyo, Denon, Marantz,
  Yamaha…) apparaissent alors dans le menu de sortie du lecteur, section
  « Lecteurs réseau ». Une fois choisi, c'est l'appareil qui
  lit : le fichier lui est envoyé tel quel, sans conversion, donc en hi-res s'il
  le permet. La file, le seek, la pause, le volume et l'enchaînement sans blanc
  fonctionnent comme en local ; le titre, l'artiste, l'album et la pochette
  s'affichent sur l'appareil. Changer de sortie en cours d'écoute reprend le
  morceau au même endroit. Le lecteur choisi est retrouvé au lancement. Un
  format que l'appareil annonce ne pas lire (DSD, souvent) est signalé tout de
  suite.
- **Sonos** : les enceintes apparaissent sous le nom de leur pièce. Dans un
  groupe, la lecture est confiée d'elle-même à l'enceinte qui le pilote. Les
  Sonos lisent jusqu'à 24 bits / 48 kHz, et enchaînent avec un court blanc.
- **AirPlay** (AirPlay 1, en bêta) : les appareils AirPlay (amplis Onkyo,
  Denon, Marantz…, AirPort Express, Apple TV, Shairport Sync) apparaissent
  dans la même section, avec leur icône. Ici, c'est RustMusic qui lit et
  envoie le son à l'appareil, en qualité CD (16 bits / 44,1 kHz) : la file,
  l'enchaînement sans blanc, le Replay Gain, le seek et la pause restent ceux
  de RustMusic. La connexion reste ouverte d'un morceau à l'autre, la fin de
  l'un couvrant le démarrage du suivant. L'appareil joue environ 2 s après
  l'envoi : la position affichée suit ce qu'on entend, une pause s'entend
  aussitôt et la reprise repart de là où l'oreille s'est arrêtée. Le volume
  agit sur le son envoyé, jamais sur celui de l'ampli. Le titre, l'artiste,
  l'album, la pochette et la progression s'affichent sur l'appareil (selon ce
  qu'il sait montrer), au moment où le morceau s'entend. Un appareil protégé
  par mot de passe, ou qui n'accepte que l'AirPlay 2, n'est pas proposé ; le
  DSD n'est pas encore pris en charge.
- UPnP/DLNA et AirPlay s'activent chacun de leur côté, dans Réglages → Réseau
  (tous deux désactivés par défaut). Dans la barre du lecteur, une seule
  pastille « Réseau » montre par ses icônes ce qui est actif (lecteurs,
  AirPlay, partage DLNA), le détail au survol. Un clic ouvre un menu pour
  activer ou couper chacun d'un geste, avec son état (« Lecture sur
  TX-RZ900 », « En ligne · 192.168.1.24:8200 ») et un raccourci vers les
  réglages réseau.
- La recherche (UPnP et AirPlay en même temps) part de chaque carte réseau
  de l'ordinateur : un VPN ou une carte virtuelle ne cache plus les
  appareils du salon.
- **Serveur DLNA et VPN** : avec un VPN actif (WireGuard, OpenVPN…), le
  serveur s'annonçait avec l'adresse du tunnel, et les amplis du salon
  affichaient « Aucun serveur ». Il ne s'annonce plus que sur les cartes du
  réseau local, jamais dans le tunnel, répond à chaque appareil par la carte
  qui mène à lui, et l'adresse affichée est celle du réseau local.
- **Serveur DLNA : pause et déplacement dans le morceau** depuis l'ampli ou
  son appli. Le serveur ne disait pas qu'il les acceptait, et certains
  appareils (Onkyo notamment) refusaient la pause. C'est aussi le cas des
  morceaux envoyés à un lecteur réseau.
- macOS : RustMusic demande l'autorisation d'accéder au réseau local, sans
  laquelle macOS 15 bloque en silence la recherche et le serveur DLNA.

### Télécommande d'ampli
- **Piloter son ampli depuis RustMusic** (en bêta), à activer dans le nouveau
  menu Réglages → Télécommande (désactivé par défaut : aucun ampli n'est
  contacté sans lui). Les amplis Onkyo, Pioneer et Integra du réseau sont
  trouvés d'eux-mêmes ; une fois l'ampli choisi, une télécommande se range au
  bord droit de la fenêtre et sort par sa languette. En tête, un afficheur
  façon façade d'ampli montre la source, le mode d'écoute et le volume ;
  dessous, le volume (boutons, curseur, sourdine) et trois onglets :
  **Sources** (les vraies entrées de l'ampli, en tuiles), **Son** (modes
  d'écoute, graves et aigus, zone 2) et **Navigation**. Elle s'adapte à
  l'ampli : ses entrées, ses plages de volume et de tonalité, sa zone 2 s'il
  en a une. Ce qu'on change sur l'ampli lui-même (télécommande, façade)
  s'affiche aussitôt. L'ampli choisi est rejoint au lancement, et retrouvé de
  lui-même après une coupure.
- **Navigation** : sur l'entrée NET (ou USB), l'écran de l'ampli s'affiche
  comme dans son appli : services avec leur logo (Spotify, Deezer, TuneIn,
  DLNA, AirPlay…), dossiers et morceaux, curseur, puis la lecture en cours
  avec sa pochette, le temps et les touches lecture/pause, précédent,
  suivant, aléatoire, répétition. Une ligne s'ouvre d'un clic ; les flèches
  et OK pilotent la liste, ou le menu affiché sur la TV sur les autres
  entrées. L'onglet s'ouvre de lui-même au passage sur NET.
- **Mode mini** : la télécommande se réduit à l'afficheur, la mise en marche,
  la source et le volume ; un clic sur l'afficheur la rouvre en entier. Le
  choix est retenu.
- **Mise en veille par appui maintenu** sur le bouton marche (un peu moins
  d'une seconde, le bouton se remplit de rouge) : un clic égaré n'éteint plus
  l'ampli. L'allumage, lui, se fait d'un clic.

### Tags
- **Atelier refait** : un seul tableau, et un panneau d'édition à droite,
  qu'on replie d'un clic. En tête, l'album, le nombre de modifications en
  attente, **Annuler** (Ctrl+Z, avec le nombre d'étapes) et **Écrire N
  fichiers** (Ctrl+S). Les outils sont rangés dans deux menus, **Tags**
  (Compléter en ligne, Nettoyer, Numéroter, Pochette) et **Fichiers**
  (Renommer, Historique), chacun avec une ligne qui dit ce qu'il fait.
- **Points à vérifier** : titre, artiste ou album manquant, genre manquant,
  artiste de l'album vide, « feat. » dans l'artiste. Chaque point se corrige
  d'un bouton, ou filtre le tableau sur les morceaux concernés.
- **Édition dans le tableau** : double-clic ou F2, puis Entrée ou Tab pour
  passer à la cellule suivante. Sélection au clic, Maj et Ctrl. Chaque
  colonne a son menu : recopier la valeur sur la sélection, casse Titre,
  vider, masquer. Les colonnes affichées sont retenues ; l'artiste de
  l'album est masqué par défaut.
- **Panneau d'édition** : les champs du morceau, de l'album et des crédits
  (compositeur, commentaire). Une valeur saisie vaut pour tous les morceaux
  choisis ; un champ aux valeurs différentes le dit et les garde tant qu'on
  n'y écrit rien. Un champ modifié se rétablit d'un clic, et les genres déjà
  présents dans l'album sont proposés.
- **MusicBrainz** rejoint Deezer pour compléter les tags. Il donne aussi les
  compositeurs et la pochette en 1200 px (Cover Art Archive). Les éditions d'un même album se distinguent par l'année, le pays
  et le support (« 1991 · JP · CD »). La source choisie est retenue.
- **Compléter en ligne** refait, en deux étapes. D'abord l'édition : la
  recherche part d'elle-même, les éditions au même nombre de pistes que vos
  fichiers passent en tête, la plus probable est marquée « Meilleur choix ».
  Puis les champs : **Compléter les vides** (rien n'est écrasé), **Tout
  mettre à jour** ou **Personnalisé**, avec pour chaque champ la valeur de
  vos fichiers, celle de la source et l'effet (« Remplit 17 », « Remplace
  1 »), dépliable morceau par morceau. Un fichier mal associé s'écarte d'un
  clic. Tout arrive dans l'atelier, à relire avant d'écrire.
- **Albums à plusieurs disques** : les pistes portant le même numéro sur deux
  disques ne sont plus confondues, avec Deezer comme avec MusicBrainz, et le
  numéro de disque est proposé avec le reste.
- « Corriger les tags » devient « Éditer les tags » dans les menus.

## [0.2.10] - 2026-10-11

Un lecteur qui se fait tout petit, aux couleurs de son choix : volume dans le
mini-lecteur, micro-lecteur réduit à la pochette, couleur d'accent libre et
raccourcis clavier personnalisables. Les playlists passent au tableau de la
bibliothèque, les grandes collections s'ouvrent plus vite, les morceaux
démarrent sans attendre la carte son, et l'enchaînement sans blanc gagne en
précision. Sur une machine modeste ou virtuelle, l'interface s'allège et ne
clignote plus au défilement. La reprise au démarrage repart vraiment, la
fenêtre rouvre comme on l'a laissée, et sous Linux le mini et le micro-lecteur
se comportent enfin comme ailleurs.

### Lecture
- **Volume dans le mini-lecteur** : un bouton au bout des commandes ouvre la
  sourdine et la glissière ; la molette sur le bouton règle le volume. Bulle
  verticale ou horizontale, au choix dans Apparence → Mini-lecteur.
- **Micro-lecteur** : depuis le mini-lecteur, la fenêtre devient un carré de
  pochette. Au survol : précédent, lecture, suivant, la timeline du morceau,
  le volume (petite bulle) et le retour au mini-lecteur ; la molette règle
  aussi le volume. La poignée du coin agrandit la pochette jusqu'à trois
  fois, et la taille est retenue.
- Le mini et le micro-lecteur restent entiers dans l'écran : collés au bord,
  ils ne débordent plus en grandissant (retour au mini, panneau File ou
  Paroles).
- **Linux : le mini et le micro-lecteur prennent leur vraie taille** : la
  fenêtre gardait la taille du mode normal, GTK ignorant tout redimensionnement
  d'une fenêtre figée. La taille passe désormais par GTK directement, et la
  poignée du micro-lecteur fonctionne.
- **Linux : les commandes du micro-lecteur disparaissent** quand la souris
  quitte la fenêtre. WebKitGTK croyait la souris encore dessus ; la sortie est
  maintenant lue auprès de GTK.
- **Reprise au démarrage qui repart vraiment** : relancer le dernier morceau là
  où on l'avait laissé ne donnait souvent que du silence, le temps que le
  décodage rattrape la position (plusieurs secondes sur un fichier réseau ou
  rééchantillonné). Un saut au-delà de ce qui est décodé relance désormais le
  décodage à la position visée ; le début du morceau se décode ensuite. Un
  saut loin dans un morceau à peine lancé est lui aussi immédiat, et la barre
  de lecture se recale si le son tarde. Le bouton « Reprendre » de l'accueil
  suit le réglage « Reprendre là où je m'étais arrêté » (Réglages → Général) :
  coupé, le morceau repart du début.
- **Enchaînement sans blanc plus précis** : la piste suivante démarre dans le
  même bloc de son que la fin de la précédente, au lieu d'attendre un tour du
  lecteur. Avec un rééchantillonnage (fichier 44,1 kHz sur une sortie 48 kHz,
  par exemple), la fin du morceau n'est plus écourtée et le début n'a plus de
  court silence. Après un arrêt, le premier enchaînement de la lecture
  suivante est lui aussi sans blanc. Merci à l'auteur de la PR #3 pour
  l'analyse.
- **Linux : sortie exclusive retrouvée sur certains DAC USB** : quand PipeWire nomme la
  sortie d'après la base USB (« Fiio E10 ») et ALSA d'après la puce
  (« DigiHug USB Audio »), RustMusic ne retrouvait pas la carte et jouait en
  partagé (« Pas d'accès direct à cette sortie »). Il demande désormais à
  PipeWire quelle carte est derrière la sortie. Les capacités du DAC
  (fréquences, profondeur) s'affichent aussi dans la liste des sorties.

- **Démarrage instantané en mode partagé** : la sortie reste ouverte d'un
  morceau à l'autre, comme en mode exclusif. Certains DAC USB mettaient plus
  d'1,5 s à rouvrir leur flux à chaque changement de morceau ; seul le premier
  morceau paie désormais ce délai. La sortie se referme après 30 s sans lecture,
  ou dès qu'une lecture exclusive ou DSD a besoin du DAC.
- **Morceau qui décrochait en cours de lecture** : un tampon momentanément vide
  pendant le décodage (fichier sur un disque réseau, par exemple) pouvait être
  pris pour la fin du morceau et déclencher l'enchaînement trop tôt : le son
  sautait, puis le compteur reculait ou le morceau revenait. L'enchaînement
  attend maintenant que le morceau soit entièrement décodé, et l'affichage se
  recale sur le morceau réellement joué si les deux divergent.
- **Préampli Replay Gain pris en compte tout de suite** : changer le mode ou
  le préampli s'entend aussitôt sur le morceau en cours. Avant, le réglage
  n'agissait qu'à partir du deuxième morceau : le suivant, déjà préchargé pour
  l'enchaînement sans blanc, gardait l'ancien gain. Le gain d'une piste
  préchargée est désormais calculé au moment où elle démarre.

### Playlists
- **Le tableau de la bibliothèque partout** : une playlist, les Titres likés et
  Récemment joués s'affichent avec les mêmes colonnes, le même tri et le même
  menu que les morceaux de la bibliothèque. L'ancien tableau, dont les colonnes
  se décalaient, disparaît. « Retirer de la playlist » et « Retirer des likes »
  sont dans le menu de chaque morceau.
- **Sélectionner** : sur une playlist, et sur la page Playlists pour choisir
  plusieurs playlists d'un coup (lire, ajouter à la file…).

### Bibliothèque
- **Sélectionner sur les fiches** : les fiches artiste, genre et année ont leur
  bouton Sélectionner.
- Les anciens boutons « Sélectionner » et « Type d'affichage », restés sur la
  ligne des onglets (Mix, Années et les fiches), en sont retirés : ils
  doublaient ceux de la page et cachaient l'onglet Playlists.
- **Onglets Albums et Artistes plus rapides** sur les grandes collections : les
  pochettes ne se chargent plus toutes d'un coup, seulement à l'approche de
  l'écran (836 appels au lancement de l'onglet contre 21 sur une collection
  d'essai).
- **Pistes de compilations ignorées à l'import** : une compilation sans tag
  « artiste de l'album », du même titre qu'une autre rangée dans un autre
  dossier, perdait toutes ses pistes sauf la première (169 fichiers sur 2 424
  chez un utilisateur). Elles sont désormais indexées ; un nouveau scan suffit.

- **Gérer une bibliothèque depuis la barre latérale** : dans le sélecteur de
  bibliothèques, le bouton ⋯ de chaque ligne ouvre le même menu que « Gérer »
  (rescanner, dossiers, pochettes, import, suppression), sans avoir à l'ouvrir.
- **Renommer une bibliothèque** et **la définir par défaut**, dans ce menu comme
  dans « Gérer ».

### Apparence
- **La fenêtre rouvre comme on l'a laissée** : taille, fenêtre agrandie ou plein
  écran, et position (sauf sous Wayland, qui place lui-même les fenêtres). Elles
  sont enregistrées au fil des changements, et plus seulement à la sortie de
  l'appli, qui n'a souvent pas lieu avec l'icône de notification. Quitter en
  mini-lecteur ne les écrase plus. Le micro-lecteur retient aussi sa position
  (Windows, macOS).
- **Couleur d'accent au choix** (Apparence → Thème) : dix teintes prêtes ou
  une couleur libre. Le logo, les boutons, les sélections, les liens et
  l'écran de chargement la suivent ; le vert des états validés ne change pas.
- **Masquer les playlists de la barre latérale** (Apparence → Barre latérale) :
  playlists épinglées, Titres likés et Récemment joués disparaissent d'un clic,
  comme les albums et artistes épinglés.

- **Fenêtres au style de l'application** : toutes les popins (création et
  renommage de bibliothèque, playlists, éditeur de tags, renommage de fichiers,
  colonnes, recherche de pochette, import, réinitialisation, détails d'une
  sortie audio…) adoptent l'habillage des réglages : en-tête teinté, champs
  clairs, pied de fenêtre avec le bouton à la couleur d'accent. Elles suivent
  le thème clair ou sombre et la couleur d'accent choisie.
- **Effets visuels allégés** (Apparence → Options expert) : Auto, Complets ou
  Allégés. Allégés coupe les flous d'arrière-plan, le fondu des pochettes et le
  flou en direct des fonds, recalculés à chaque image au défilement. Auto les
  allège en rendu logiciel, sur un GPU virtuel, ancien ou absent, en machine
  virtuelle et sur une machine modeste (2 cœurs ou 4 Go de mémoire au plus) ;
  la raison s'affiche sous le réglage. Effet immédiat, sans relance.
- **Linux : plus de clignotement au défilement** sur les GPU mal gérés par
  DMA-BUF. Le réglage DMA-BUF « Auto » le coupe désormais selon le pilote
  graphique réel : GPU virtuel (VirtualBox, VMware, QEMU, Hyper-V), Radeon
  ancienne ou aucun GPU accéléré, en plus du pilote NVIDIA propriétaire. Une
  machine virtuelle avec un vrai GPU attribué le garde.

### Raccourcis
- **Raccourcis personnalisables** : dans Réglages → Raccourcis, un clic sur les
  touches puis la nouvelle combinaison. Une touche déjà prise est refusée en
  disant par quoi ; chaque raccourci se rétablit seul, ou tous d'un coup.
- **Basculer en mini-lecteur** : Ctrl + Maj + M (⌘ + ⇧ + M sur Mac).
- La sourdine (M) répond à la touche M sur un clavier AZERTY : c'était la
  touche « , ».

### Réglages
- Le bouton Retour des réglages ramène toujours à la page d'où l'on venait,
  même après plusieurs sections parcourues.
- **Relancer maintenant** : après un changement du mode de rendu ou de DMA-BUF
  (Linux), un bouton redémarre RustMusic pour l'appliquer. L'avertissement ne
  disparaît plus en quittant la page avant d'avoir relancé.

### Technique
- En développement, le journal détaille la lecture (enchaînements, fins de
  piste, seeks) et reçoit les incidents de lecture vus par l'interface.
- Dépendances JavaScript et Rust mises à jour (TypeScript reste en 6 : SvelteKit
  3 ne suit pas encore la 7).
- Tests du décodeur : après un saut lointain dès le démarrage, le tampon est
  identique, échantillon pour échantillon, à un décodage d'une traite.

## [0.2.9] - 2026-10-05

Correctifs remontés par les testeurs : un disque externe qui saturait la
mémoire, le DSD converti qui ne passait pas au morceau suivant, et un journal
qui restait vide quand il aurait dû aider.

### Lecture
- **Le DSD converti en PCM enchaîne enfin les morceaux** (macOS, DAC sans
  DSD natif). La fin du morceau n'était jamais signalée quand le décodage
  rendait un peu moins que la durée annoncée : la file restait bloquée.
- **Ouvrir un dossier** : les morceaux arrivent dans l'ordre de l'album et
  s'enchaînent.
- **Mode exclusif sur les haut-parleurs du Mac** : plus de silence avec la
  sortie système (erreur CoreAudio `!obj`). Le DSD natif profite du même
  correctif. Un DAC choisi dans la liste n'était pas concerné.

### Bibliothèque
- **Ajouter un disque externe ne fait plus exploser la mémoire.** Le scan ne
  tourne plus sans fin dans les raccourcis de dossiers qui bouclent (anciens
  disques système), saute la corbeille et « System Volume Information », ne
  garde que la pochette des morceaux qui embarquent tout un livret, et ne
  décode plus les images géantes.

### Fenêtre
- **Boutons de fenêtre façon macOS** dans l'ordre du système (rouge, jaune,
  vert), avec leurs symboles : fermer, réduire, plein écran. L'aperçu des
  réglages aussi.

### Sous le capot
- Le journal enregistre aussi les informations (sortie choisie, chaîne audio,
  conversions), et plus seulement les erreurs : celui d'un testeur restait
  vide.

## [0.2.8] - 2026-10-04

Le son tient enfin sa promesse de bit-perfect, la bibliothèque s'explore par
années, par mix et par playlists, l'application explique ses termes
techniques, et l'import se suit en direct et s'arrête à tout moment. Sous
Linux, l'affichage devient deux fois plus fluide et les sorties audio cessent
de se marcher dessus.

### Audio
- **Bit-perfect réel en mode exclusif**, sous Windows, Linux et macOS. La
  sortie baissait le son de 2 % même en exclusif, et la conversion vers le DAC
  n'était pas l'exacte réciproque du décodage. À volume 100 % et sans Replay
  Gain, chaque échantillon arrive désormais intact, vérifié par des tests sur
  des fichiers étalons 16 et 24 bits.
- La pastille « Bit-perfect » ne l'affirme plus à tort : il faut aussi les
  mêmes canaux, une source sans perte et un Replay Gain neutre. Une pastille
  « Exclusif » indique un DAC réservé dont le son est tout de même modifié.
- **Mode exclusif sous Windows : plus d'attente entre les morceaux.** Le DAC
  restait muet 2 à 4 secondes à chaque changement de morceau, le temps de le
  rouvrir. Il reste désormais ouvert tant que le format ne change pas : le
  morceau suivant démarre en 30 ms.
- **AIFF, ALAC et Opus sont lus.** Ils étaient annoncés sans être décodés. Les
  extensions en majuscules sont reconnues ; WMA et APE, jamais lus, sont
  retirés.
- **DSD natif (DoP) sans coupure sous Linux et macOS**, comme sous Windows.
  Sous Windows, le début du premier morceau DSD n'est plus coupé.
- Mode exclusif sous Linux et macOS : le morceau ne joue plus trop vite ou trop
  lentement quand le DAC refuse la fréquence du fichier.
- **Quand le mode exclusif ou le DSD natif n'aboutit pas, l'app dit pourquoi** :
  format refusé, DAC occupé, sortie sans accès direct, DoP refusé, fréquence
  convertie. Le motif apparaît dans le détail de la chaîne audio, sur la
  pastille du lecteur et dans Réglages → Audio.
- **Pas de mise en veille pendant la lecture** (réglage, activé par défaut).
  Sous Linux, la lecture se met en pause proprement quand l'ordinateur
  s'endort.
- Les commandes multimédia du système ne s'inversent plus (lecture qui
  devenait pause).
- **Un anneau tourne autour du bouton lecture** tant que le son n'a pas
  démarré : on ne croit plus à un plantage pendant l'ouverture du DAC.
- **Sous Linux, la sortie partagée passe par le serveur son** (PipeWire ou
  PulseAudio). Choisir son DAC ne bloque plus les autres applications, et les
  erreurs « format f32 non pris en charge » ou « périphérique occupé »
  disparaissent. La liste montre les vrais noms des sorties et les capacités
  réelles du DAC ; la sortie utilisée est cochée dès le lancement.
- **La sortie choisie est retenue d'un lancement à l'autre.** Elle ne l'était
  jamais : sous Linux, le DSD natif (DoP) retombait sans prévenir en
  conversion PCM après un redémarrage.
- **Mode exclusif et DSD natif sous Linux sans perdre la sortie par défaut.**
  La carte est libérée puis rendue proprement au système : le son des autres
  applications ne part plus vers l'écran HDMI après une lecture, et la carte
  ne disparaît plus jusqu'au redémarrage.
- Sous Linux, le DSD converti en PCM suit le réglage « Sortie exclusive »,
  comme les autres fichiers.
- Mode partagé sous Linux : la pause, le volume et les sauts réagissent tout
  de suite, au lieu de 2 secondes plus tard, et le morceau ne repart plus du
  début après une seconde de lecture.
- Passer au morceau suivant ne bascule plus vers une autre sortie, et le
  morceau précédent ne perturbe plus celui qui démarre.

### Comprendre le son
- **Bulles d'explication** : DAC, DSD, DoP, PCM, bit-perfect,
  rééchantillonnage… sont soulignés, un clic en donne la définition. Elles se
  désactivent dans Apparence.
- **Nouvelle page « Comprendre le son »** dans les réglages : le trajet du son
  en mode partagé et exclusif, ce que disent les pastilles du lecteur, et un
  glossaire.
- Les textes de Réglages → Audio disent plus clairement ce que fait chaque
  mode.

### Années
- **Onglet Années** : un déroulé par décennie, la plus récente d'abord, avec
  toute la décennie ou l'une de ses années, et le mix de l'époque.
- **Page d'une décennie ou d'une année** : ses albums, ses morceaux ou ses
  artistes.
- « Toutes les années » sur l'accueil.

### Mix
- **Page Mix** : les mix du jour, un mix par genre, par décennie, et les
  vôtres.
- **Créer un mix** : une recette (genres, notes, jamais écoutés, Hi-Res…) dont
  RustMusic tire chaque jour 30, 50 ou 100 titres. À la différence d'une
  playlist auto, qui contient tous les titres de ses règles.
- **Des mix variés** : un genre ne fait un mix que s'il réunit au moins
  5 artistes et 30 titres, et un artiste n'y place pas plus de 3 titres
  environ, mix créés compris. Fini les mix d'un seul artiste.
- « Pour vous » change de genres chaque jour, parmi les six plus fournis.
- Éditeur de mix et de playlist auto refait : aperçu de la vignette, recettes
  en cartes, nombre de titres trouvés en direct.
- Sur chaque mix : nouveau tirage, et « Garder en playlist » pour figer celui
  du jour.
- Sur l'accueil : « Tous les mix » et « Vos mix ».

### Playlists
- **Page Playlists** : toutes les playlists, avec recherche, filtre
  classiques / auto, tri, grille ou liste.
- **La barre latérale ne montre que les playlists épinglées**, comme les albums
  et artistes. Toutes le sont à la mise à jour, et une nouvelle l'est d'office.
  On épingle depuis la punaise d'une carte, le clic droit ou la page de la
  playlist ; le titre « Playlists » de la barre mène à la page.
- « Vos playlists » sur l'accueil.
- « Ajouter à une playlist » ne propose plus les playlists auto, qui ne
  recevaient rien.

### Import
- **L'import se suit en direct.** Le dossier ajouté apparaît tout de suite
  dans « Dossiers synchronisés », avec la recherche des fichiers (« 1 234
  trouvés »), puis l'avancement, la barre de progression et le fichier en
  cours. Sur un partage réseau, la longue recherche des fichiers ne laisse
  plus l'écran muet.
- **Un import ou un rescan s'arrête** depuis le popin des dossiers, l'écran de
  progression ou la barre d'état. Ce qui est déjà importé est gardé, et un
  message « Importation partielle » dit combien de titres, d'albums et
  d'artistes ont été ajoutés. Le bouton annuler de la barre d'état arrête
  vraiment l'import, au lieu de seulement masquer la tâche.
- Le compteur de titres de la barre latérale et de l'en-tête monte pendant
  l'import, et ne compte que les morceaux réellement nouveaux.
- Après un import, même arrêté, l'accueil, les Morceaux, les Genres et les
  Dossiers se rechargent : plus de « 0 résultat » alors que des titres ont été
  ajoutés.
- Un seul import à la fois : un second dit « Un import est déjà en cours » au
  lieu d'échouer sans rien dire.
- Une bibliothèque vide affiche la progression de l'import, d'où qu'il ait
  été lancé, avec les boutons Arrêter et Dossiers.
- **Un import n'empêche plus d'enregistrer la lecture** : position, historique
  et compteurs d'écoute échouaient pendant tout l'import, la base restant
  verrouillée.

### Premier lancement
- **Accueil de bienvenue** sans bibliothèque ou avec une bibliothèque vide :
  « Importer ma musique », les trois étapes, et un aperçu des sections. On
  peut aussi juste écouter un dossier sans rien importer.
- **Page d'import** : nom de la bibliothèque, import d'un dossier ou de
  fichiers, formats reconnus. Annuler le choix du dossier ne crée rien.
- Sans bibliothèque, toutes les tuiles de la barre latérale restent
  affichées et mènent à l'import, puis à la section choisie. Dans une
  bibliothèque vide, chaque onglet propose directement l'import.

### Navigation
- Onglets dans l'ordre Morceaux, Artistes, Albums, Genres, Années, Dossiers,
  Mix, Playlists. Années, Mix et Playlists se cochent dans Apparence.
- Mode « onglets en haut » : Accueil ouvre la barre du haut, et la barre
  latérale n'a plus de tuiles. « Les deux » est exactement la somme des deux.
- Les boutons Fichier et Dossier de la barre latérale sont affichés par
  défaut, et l'aperçu d'Apparence les place au-dessus des tuiles, comme dans
  la barre.
- Les vignettes de Titres likés, Récemment joués et des playlists sans
  pochette gardent leur couleur en mode sombre (le rose virait à l'olive) et
  leur icône se lit.

### Bibliothèque
- Les colonnes hors de l'écran à droite ne sont plus vides. Seule la liste
  défile horizontalement, avec sa barre en bas ; les en-têtes suivent.
- Cases de sélection dans Dossiers et dans toutes les listes de morceaux.
  Maj+clic ne surligne plus le texte.
- **Pochettes et portraits** : la récupération s'annule, ne se lance pas deux
  fois, ne retélécharge plus tout, et une panne réseau ne marque plus un
  artiste « introuvable ». Les images trop lourdes sont allégées, et
  « Nettoyer le cache des images » (Stockage) supprime celles qui ne servent
  plus. Les tags des fichiers ne sont jamais touchés.
- Fiche d'un morceau : pochette et portrait de l'artiste même quand le fichier
  n'en contient pas.
- **Fiche d'un genre refaite**, dans le style des autres fiches.
- Mode clair : sélecteur de profil, menus des playlists et des onglets, page
  Commentaires enfin lisibles.
- Panneau « Ensuite » : la vignette d'un morceau sans pochette intégrée
  reprend celle de l'album.
- Un nom de fichier long ne passe plus sous les pastilles du lecteur.
- Les liens « mailto » ouvrent la messagerie au lieu d'un message d'erreur.

### DLNA
- Les menus (Artistes, Albums, Dossiers) s'affichent dans la langue de l'app
  sur l'ampli, et plus toujours en français.

### Fenêtre
- **Plus d'écran blanc au lancement** : la fenêtre s'ouvre aussitôt sur un
  écran de chargement aux couleurs du thème, qui s'efface en fondu quand
  l'application est prête.
- Une bibliothèque s'affiche dès qu'on y entre, sans page vide le temps de la
  relire en base. La page Mix montre ses cartes en attente au lieu de sections
  qui apparaissent l'une après l'autre.
- **macOS : le bouton vert passe en plein écran**, comme dans les autres
  applications ; ⌥ + clic garde l'agrandissement. Le mini-lecteur sait en
  sortir et y revenir.

### Affichage sous Linux
- **Fin de la fenêtre grise au démarrage** : un cache de polices laissé par
  une autre application (Flatpak) bloquait l'affichage. Il est nettoyé au
  lancement.
- **Défilement deux fois plus fluide**, surtout en 4K : 37 images par seconde
  au lieu de 17 (transfert d'image DMA-BUF, sauf avec le pilote NVIDIA
  propriétaire).
- **Fonds flous bien plus légers** (reprise de la lecture, récents, paroles) :
  floutés une fois pour toutes au lieu de l'être à chaque image. Pendant la
  lecture, l'interface n'occupe plus un cœur entier du processeur.
- **Réglages → Apparence → Options expert** : mode de rendu, transfert
  DMA-BUF et fonds flous (pré-calculés ou en direct), pour les machines qui
  en ont besoin.

### Divers
- L'export des réglages conserve les playlists auto, les mix et l'épinglage.
- Le nettoyage du cache d'images épargne les pochettes de playlists.
- L'accueil, les statistiques et la page d'import ne sont plus coupés en bas
  sur une petite fenêtre.

### Sous le capot
- Passage à SvelteKit 3, Tauri 2.12, sqlx 0.9 et rubato 5 ; plus aucune
  faille signalée par `npm audit`. Les requêtes SQL construites à la volée ont
  été relues une à une.
- Les réglages de la base (journal, cache) valent pour toutes ses connexions,
  et plus seulement pour la première.
- Les imports internes passent de `$lib` à `#lib`, comme le veut SvelteKit 3 :
  fin des avertissements au lancement.

## [0.2.7] - 2026-10-01

L'application change de visage d'un bout à l'autre : accueil, barre latérale,
lecteur, bibliothèque, fiches et réglages. Tout est traduit dans les cinq
langues, et une grande bibliothèque ne la fait plus peiner.

### Accueil
- **Reprendre là où l'on s'était arrêté.** Le haut de l'accueil montre le
  morceau en cours ou le dernier écouté, sur sa pochette floutée, avec
  « Reprendre » à la position enregistrée et un panneau « Ensuite » : la suite
  de la file, sinon les derniers écoutés. Cette position n'était en fait
  jamais enregistrée : elle valait toujours zéro.
- **Mix « Pour vous »** : deux genres du jour, « Oubliés » (pas écoutés depuis
  plus de six mois) et « Haute résolution », chacun en mosaïque des albums
  réellement tirés. Un mix du jour reste le même toute la journée ; « Nouveaux
  mix » en tire d'autres.
- **« Au fil des décennies »** : une tuile par décennie présente dans la
  bibliothèque, qui lance un mix de l'époque.
- « Explorer par genre » en tuiles à icônes thématiques, « Récemment
  ajoutés », et « Récemment joués » : le dernier morceau en vedette, puis les
  albums écoutés avant — un clic reprend l'album au dernier morceau joué.

### Barre latérale et barre de titre
- **Barre latérale refaite** : sections en tuiles, sélecteur de bibliothèque,
  playlists avec la mosaïque de leurs pochettes, état du scan en direct
  (« Scan… 12 % ») et profil en pied de colonne. Elle se replie en icônes, et
  son pied se réduit à une ligne.
- **Albums et artistes épinglés** : depuis le menu d'un album ou d'un
  artiste, ils restent à portée de clic dans la barre latérale.
- Une seule barre de 60 px en haut : logo, navigation, recherche (Ctrl K),
  minuteur, thème, profil et boutons de fenêtre. L'ancien en-tête disparaît.
- La fenêtre « Dossiers synchronisés » est refaite, et ne reste plus coincée
  dans la largeur de la barre latérale.
- Le mode clair passe à des tons chauds, plus doux qu'un blanc pur.
- Désactiver les favoris masque aussi « Titres likés » dans la barre latérale.

### Lecteur
- **Un dock flottant teinté par la pochette.** Sa couleur suit l'album en
  cours ; une pochette en noir et blanc donne un lecteur neutre. La ligne
  d'état montre le format, la profondeur, la fréquence, le débit, le chemin du
  fichier et la chaîne audio en pastilles colorées : bit-perfect, DSD natif,
  traitement, sortie, DLNA. Le détail de la chaîne est redessiné étape par
  étape.
- Menu Sortie : noms de périphériques lisibles, badge de qualité par sortie
  (Qualité CD, Hi-Res, Ultra Hi-Res), interrupteurs du mode exclusif et du DSD
  natif (DoP).
- L'année de l'album est cliquable : elle ouvre les albums de cette année.
- **Mini-lecteur refait**, teinté lui aussi, avec la file et les paroles
  synchronisées en onglets. Revenir au lecteur complet ne laisse plus une
  fenêtre minuscule.

### Bibliothèque
- **Un en-tête commun à tous les onglets** : nom de la bibliothèque, chiffres
  (albums, artistes, titres, taille, dernier scan) et menu « Gérer » :
  rescanner, dossiers, pochettes, images d'artistes, import, suppression.
- **Albums** : recherche, filtres Hi-Res, Compilations, Sans pochette, Genre et
  Année, tri par titre sans l'article (« The Doors » à D), artiste, année ou
  date d'ajout, grille ou liste, badge de qualité sur la pochette.
- **Artistes** : même barre d'outils, filtre « Sans portrait », tri par nombre
  d'albums, de titres ou par durée d'écoute.
- **Morceaux** : densités Compacte et Détaillée, colonne Qualité (Hi-Res,
  Lossless, Lossy), filtres Hi-Res, Lossless, Favoris, Sans pochette et Genre,
  colonnes redimensionnables (double-clic sur Titre : largeur automatique).
  « Tout lire » joue la liste telle qu'affichée, filtres et tri compris.
- **Genres** : mosaïques de pochettes à la couleur du genre, trois artistes
  phares, part de la bibliothèque, filtres « Principaux » et « Sans genre »,
  tri par écoute récente, « Mix aléatoire ».
- **Dossiers** : cartes ou lignes, fil d'Ariane, état du scan de chaque
  dossier, « Lire le dossier ».
- **Navigation A–Z** sur Albums, Artistes, Genres et Morceaux quand le tri est
  alphabétique. Sur Morceaux, une lettre mène droit à sa place, même au milieu
  de neuf mille titres qui ne sont pas encore chargés.

### Fiches
- **Album** : badge de la meilleure qualité, lecture, aléatoire, favori de tout
  l'album, genres, date de sortie et label lus dans les tags, dossier, titres
  groupés par disque, autres albums de l'artiste, albums et artistes
  similaires. Précédent et suivant suivent l'ordre de la vue Albums.
- **Artiste** : portrait, période d'activité, discographie en carrousel, tous
  les titres groupés par album (repliables), « Apparaît sur », artistes
  similaires.
- **Morceau** : fiche complète — audio, tags, fichier, écoutes, note, « Lire
  ensuite », et les autres pistes du même album.
- Le haut de chaque fiche s'éclaire de la couleur de sa pochette.

### Réglages
- **Nouvelle présentation** : colonne de sections, recherche dans les
  réglages, compteur « n modifiés · Rétablir » sur chaque page.
- Apparence : aperçus vivants du thème, de la barre latérale et de la
  navigation ; style des boutons de fenêtre en cartes illustrées.
- Audio : liste des sorties avec leur type de connexion et leur palier réel,
  **« Tester le DAC »** sur la sortie choisie (et non plus sur celle par défaut
  de Windows), qualité de décodage et Replay Gain en sélecteurs illustrés.
- **Reprise de lecture** (activée par défaut) : la première lecture de la
  session repart de là où l'on s'était arrêté. Le réglage « Gapless » passe
  dans Général.
- Nouvelle section **Raccourcis** ; Réseau, Stockage et À propos refaits.

### Corrections
- **File d'attente sans pochettes ni titres depuis la vue Dossiers.** Sur un
  lecteur réseau, les chemins prenaient leur forme réseau (`\\?\UNC\…`), que
  la bibliothèque ne reconnaît pas. Ils gardent maintenant celle du dossier
  importé, et une file déjà enregistrée est réparée à l'ouverture. Les écoutes
  et les « j'aime » de ces morceaux comptent de nouveau.
- « Ajouter de la musique » puis Annuler figeait l'interface : la bibliothèque
  se chargeait derrière une boîte qu'on pouvait encore annuler.
- Le glisser-déposer de fichiers dans une bibliothèque fonctionne : les
  chemins des fichiers déposés n'arrivaient jamais.
- Un genre comme « 100% Hits » s'ouvre de nouveau.
- Le bouton agrandir / restaurer suit l'état réel de la fenêtre.
- Une réponse tardive n'affiche plus le contenu d'avant : paroles, dossiers,
  autocomplétion, pochettes, bibliothèque, lettres A–Z.
- L'accueil ne redemande plus un mix en boucle quand le tirage revient vide.
- « Léa » ou « The-Dream » gardent leur initiale dans le tri.
- Une colonne Titre élargie au-delà de 600 px n'est plus oubliée.
- Un rescan annonce son début, sa progression et sa fin.
- Maj+clic sélectionne de nouveau une plage après être sorti puis revenu en
  mode sélection.

### Traductions
- Environ 400 textes restés en français sont traduits en anglais, espagnol,
  allemand et italien : menus, notifications, formulaires, fenêtres, colonnes,
  playlists intelligentes. Les pluriels sont justes (« 1 titre »,
  « 2 titres »), et nombres, tailles, durées et dates suivent la langue.

### Performances
- Ouvrir une bibliothèque ne charge plus tous ses titres, seulement les albums
  et les artistes. Les statistiques des genres viennent de la base, pochettes
  comprises, en une requête au lieu d'une par genre.
- L'onglet Morceaux garde au plus 500 lignes en défilant, sans à-coup, et ne
  dessine que celles qui approchent de l'écran.
- Les listes ne se redessinent plus à chaque mise à jour de la position de
  lecture, soit une dizaine de fois par seconde.
- Fermée, la file d'attente ne dessine rien ; ouverte, elle montre ses titres
  par lots de 150.
- La recherche des Albums et des Artistes ne retrie plus à chaque frappe.
- Les miniatures de pochettes ne sont plus redemandées à chaque affichage.
- Seules les icônes réellement utilisées sont embarquées, au lieu de packs
  entiers : le code de l'interface passe de 12 à 2,5 Mo.

### Sous le capot
- Composants d'interface communs (boutons, menus, champs, carrousels,
  charpente des fiches) ; les grandes fenêtres d'édition de tags et de
  renommage sont découpées en morceaux lisibles.
- L'extension SQL inutilisée est retirée, et le code mort supprimé.
- Dépendances mises à jour pour trois failles signalées par `npm audit`
  (`devalue`, `nanoid`, `sharp`).

## [0.2.6] - 2026-09-27

Deux correctifs de lecture. Le lecteur ne se contredit plus, et un fichier
illisible ne le condamne plus au silence.

### Lecture
- **Un double-clic ne lance plus deux fois le même morceau.** En mode
  « simple clic = lecture », le navigateur envoie `click, click, dblclick` :
  trois demandes de lecture pour un seul geste. La première était déjà partie
  quand les suivantes arrivaient, et deux lectures se marchaient dessus — le
  son partait, mais l'état du lecteur restait celui de l'autre : bouton sur
  « lecture », barre figée à 00:00. Une demande identique qui suit de près
  est ignorée, et le double-clic ne relance plus ce que le clic a déjà lancé.
- **Le lecteur ne se croit plus à l'arrêt pendant qu'il joue.** Couper le son
  peut prendre plusieurs secondes selon le périphérique ; la lecture suivante
  démarrait entre-temps, et l'arrêt retardataire écrasait alors son état.
  Bouton sur « lecture », barre figée à 00:00, plus aucune progression — alors
  que la musique tournait. Un arrêt dépassé par une lecture plus récente
  renonce désormais à écrire.
- **Un fichier illisible ne bloque plus le lecteur.** Lorsque l'ouverture
  échouait — fichier corrompu, piste audio absente, périphérique qui refuse la
  négociation — deux drapeaux internes restaient levés : toute lecture suivante
  était refusée, et chaque arrêt attendait trois secondes dans le vide avant
  d'abandonner. Ils sont maintenant rabaissés en cas d'échec.

## [0.2.5] - 2026-09-25

Les albums et les artistes retrouvent leur identité : une compilation redevient
un album au lieu de cinquante-deux, un invité n'a plus « Various Artists » pour
nom, et un artiste présent sur une bande originale ne compte plus zéro album.
La lecture, elle, enchaîne enfin avec la suite de la liste.

### Albums
- **Une compilation ne forme plus un album par artiste.** L'identité d'un album
  incluait son artiste : sans tag « artiste de l'album », chaque piste fondait
  donc le sien. « NRJ Hit Music Only 2019 » comptait 52 fiches d'un titre
  chacune, « NRJ Music Awards 2012 » 44, « Taxi 4 » 22 — en tout 333 fiches en
  trop sur 1 236. Un album se reconnaît maintenant à son titre et à son
  dossier, les `CD1`, `Disc 2`, `Disque 3` étant rattachés au coffret auquel ils
  appartiennent. Les homonymes légitimes restent séparés : le « Best Of » d'AC/DC
  n'est pas celui de Suprême NTM.
- Le rassemblement se fait tout seul au premier démarrage. L'album fusionné
  appartient au tag « artiste de l'album » majoritaire — un featuring isolé ne
  transforme pas une pochette en compilation — et sinon à « Various Artists »,
  marqué comme compilation.

### Artistes
- **« Various Artists » ne remplace plus le vrai artiste d'un morceau.** Une
  seule variable servait à deux choses, l'artiste de la piste et le
  propriétaire de l'album, et la seconde écrasait la première. 309 morceaux
  portaient un nom qui n'était pas le leur : PJ Harvey, The White Stripes,
  Arctic Monkeys, Nick Cave… Les nouveaux fichiers sont justes d'emblée ; pour
  les anciens, Réglages → Stockage → « Réparer les liaisons d'artistes » les
  corrige, et annonce combien.
- **Un artiste n'affiche plus zéro album.** La page ne montrait que les albums
  qu'il possède ; un invité, un artiste de compilation ou de bande originale
  n'en possède aucun. 504 artistes sur 1 512 affichaient des titres et pas un
  album. Une section « Apparaît dans » réunit désormais les disques où ils ne
  font que figurer, sous leurs albums à eux.

### Lecture
- **La musique enchaîne avec la suivante.** Cliquer un morceau réduisait la
  file d'attente à ce seul morceau : rien à jouer ensuite, et l'option « en
  boucle » ne faisait que le rejouer. La liste d'où vient le clic devient la
  file — bibliothèque, album, artiste, playlists, recherche, dossiers, accueil.
  En aléatoire, le morceau cliqué ouvre le bal et le reste est tiré au sort.
- La file s'écrit en base en arrière-plan. Mettre une bibliothèque entière en
  attente représente près de 6 Mo : l'attendre repoussait le son à chaque clic.
- En mode « double-clic pour lire », un simple clic ne coupe plus le morceau
  en cours : il désigne, il ne commande pas le lecteur.

### Navigation
- **Le lecteur mène quelque part.** Son titre, son artiste et son album
  n'étaient que du texte : pour ouvrir la fiche de ce qu'on écoutait, il
  fallait le retrouver soi-même dans la bibliothèque. Les trois sont
  maintenant cliquables, sans rien changer à leur apparence — seul un
  soulignement apparaît au survol. Un fichier ouvert à la volée, absent de la
  bibliothèque, reste du texte simple.
- Dans « Récemment joués », l'artiste et l'album d'un morceau de la
  bibliothèque ouvrent leur fiche. Le titre continue de lancer la lecture.

### Menus
- Un bouton « … » sur les zones **Playlists**, **Bibliothèque** et **Ouvrir**.
  Il réunit ce qui n'avait sa place nulle part : réafficher les playlists
  masquées, déplacer les sections de la bibliothèque à gauche, en haut ou aux
  deux, choisir celles qui s'affichent, masquer la section « Ouvrir ».
- Clic droit sur une ligne de playlist : lire, modifier, supprimer.
- Option pour masquer « Ajouter aux favoris » partout où il apparaît, menu
  contextuel compris.

### Mémoire et processeur
- **La pochette ne traverse plus deux fois le pont vers l'interface.** Ses
  octets bruts voyageaient en clair dans chaque lecture de fichier, et une
  lecture part deux fois par clic. Sur une pochette ordinaire cela faisait plus
  d'un mégaoctet à sérialiser pour des octets utilisés une seule fois ; un test
  vérifie désormais que la charge utile reste sous 4 Ko.
- Le cache de données est borné à quarante fiches, les listes principales
  protégées, les plus anciennes évincées. Il grossissait sans limite.
- Survoler une grille ne déclenche plus une rafale de requêtes : le délai passe
  à 250 ms et le cache est consulté avant de repartir en base.
- Les lignes hors écran des grilles ne sont plus mises en page ni gardées en
  mémoire tant qu'on ne les atteint pas.

### Affichage
- Les playlists masquées n'apparaissent plus deux secondes au démarrage avant
  de disparaître : les réglages sont lus avant le premier rendu.

### Sécurité des données
- **Revenir à une version antérieure ne casse plus l'ajout de fichiers.** La
  nouvelle identité d'album supprimait la contrainte que l'ancien code vise
  pour ranger un morceau : réinstaller une 0.2.4 par-dessus laissait une
  bibliothèque lisible mais impossible à enrichir. L'ancienne contrainte est
  conservée à côté de la nouvelle.
- **Une sauvegarde est prise avant toute migration de la base**, par instantané
  cohérent plutôt que copie de fichier — le journal d'écriture en fait partie.
  Une seule copie est conservée, à côté de la base.

## [0.2.4] - 2026-09-19

Lancer un morceau d'un clic redevient fiable. Les artistes invités des tags
« X;Y » et « X Feat. Y » sont enfin crédités, et les compteurs d'artistes
disent la vérité. Les sections de la bibliothèque se placent et se choisissent.

### Lecture
- Le clic pour lancer un morceau ne rate plus. Un double-clic envoie
  `click, click, dblclick` : trois actions faites chacune d'une demi-douzaine
  d'appels asynchrones, sans garantie de finir dans l'ordre. Quand une
  sélection terminait après la lecture, son arrêt coupait le morceau qui venait
  de partir. Les actions passent maintenant par une file d'attente, et celles
  qu'une action plus récente rend caduques sont sautées.
- Le titre et la pochette déclenchaient deux actions par clic : leur propre
  gestionnaire, puis celui de la ligne. En mode sélection, la case était cochée
  puis décochée aussitôt ; en « simple clic = lecture », une sélection et une
  lecture partaient en concurrence.
- Liker un morceau ou ouvrir son menu ne le lance plus.
- Dans la liste détaillée, cliquer l'artiste ou l'album navigue au lieu de
  lancer la lecture.

### Tags multi-artistes
- Nouveau bouton « Réparer les liaisons d'artistes » dans Réglages → Stockage.
  Les bibliothèques indexées par une version antérieure ne créditaient que le
  premier artiste d'un tag ; un nouveau scan ne les rattrapait pas, puisqu'il
  saute les fichiers déjà indexés. La reprise ne relit aucun fichier — le tag
  est déjà en base — et peut être relancée sans créer de doublon.
- Sur la fiche d'un morceau, chaque artiste crédité a son propre lien. Le tag
  s'affichait d'une pièce derrière un lien unique, qui menait au premier nommé.
  Tant que les liaisons ne sont pas posées, l'ancien affichage est conservé.

### Artistes
- Le compteur de titres n'affiche plus « 0 ». Un artiste est rattaché à une
  piste de deux façons ; la liste n'en comptait qu'une, alors que la fiche de
  l'artiste interrogeait les deux.
- Les artistes qui n'ont plus ni piste ni album sont écartés de la liste, et
  l'en-tête les compte de la même façon. Retirer un dossier efface en cascade
  les pistes et les albums, mais pas la ligne d'artiste : le total ne
  redescendait jamais.
- Le classement des artistes les plus écoutés, dans les statistiques, n'est
  plus vide.
- « Artistes similaires » et « Albums du même genre » tirent dix entrées au
  hasard au lieu des dix premières par nom. Sur un genre qui compte deux cents
  artistes, les mêmes têtes de liste revenaient toujours.

### Sections de la bibliothèque
- Choix du placement : à gauche, en haut, ou les deux. En mode « en haut »
  elles suivent sur toutes les pages, y compris l'accueil.
- Choix des sections affichées, avec un plancher d'une section.
- Une section masquée reste visible tant qu'on s'y trouve : une barre qui
  n'indique pas où l'on est ment plus qu'elle n'encombre.
- Une entrée « Bibliothèque » apparaît dans la barre latérale quand elle ne
  liste plus aucune section, et mène à la dernière ouverte.
- « Explorateur » devient « Dossiers » partout : la même page portait deux noms
  selon l'endroit d'où on l'ouvrait.

### Affichage
- Les barres de défilement redeviennent visibles en mode clair. Depuis
  Chromium 121, `scrollbar-color` fait ignorer les règles `::-webkit-scrollbar`
  — et sa valeur n'avait jamais été déclinée pour le thème clair.
- Le bouton « choisir les colonnes » redevient cliquable. La même propriété
  bascule sur la barre du système, qui flotte par-dessus le contenu et
  s'élargit à l'approche du curseur : elle prenait le clic à sa place.
- Sur les pages album, artiste et genre, l'en-tête flouté et les sections du
  bas ne sortent plus de l'écran quand on fait défiler un tableau large.
- Masquer « Titres likés » et « Récemment joués », séparément pour la barre
  latérale et pour l'accueil.
- Masquer les boutons « Ouvrir un fichier » et « Ouvrir un dossier » de la
  barre latérale. Masqués par défaut : l'accueil propose déjà ces deux actions,
  nommées en entier.

### Performance
- Le compte d'artistes d'une bibliothèque passe de 8,5 secondes à 16
  millisecondes. Il demandait, pour chacun des artistes et pour chaque
  bibliothèque, jusqu'à trois recherches d'existence ; il rassemble maintenant
  en un seul passage les artistes qui ont du contenu.


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
