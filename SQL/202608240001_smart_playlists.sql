-- ============================================================================
-- Playlists intelligentes
-- ============================================================================
--
-- Une playlist intelligente n'est pas une liste de morceaux : c'est une
-- question posée à la bibliothèque, réévaluée à chaque ouverture. Un morceau
-- noté quatre étoiles ce soir entre dans « Mes préférés » sans qu'on l'y ajoute.
--
-- # Pourquoi étendre `playlists` plutôt que créer une table
-- Une playlist intelligente est une playlist : elle a un nom, une couleur, une
-- icône, une place dans la barre latérale, et elle se lit. Tout ce plumbing
-- existe. Une table parallèle aurait obligé à le doubler — deux listes à
-- fusionner dans la barre, deux chemins de lecture, deux routes — pour ne
-- distinguer que la façon dont le contenu est obtenu.
--
-- `playlist_items` reste vide pour ces playlists : leur contenu ne se stocke
-- pas, il se calcule. C'est précisément ce qui les rend vivantes.

ALTER TABLE playlists ADD COLUMN is_smart INTEGER NOT NULL DEFAULT 0;

-- Les règles, en JSON. Un format textuel plutôt que des colonnes : le nombre de
-- conditions varie, elles s'imbriquent, et les modéliser en tables aurait
-- demandé trois jointures pour lire ce qui se relit ici d'un coup.
ALTER TABLE playlists ADD COLUMN rules TEXT NULL;
