-- Un mix est une playlist auto dont on tire chaque jour une sélection au hasard.
-- Il vit sur la page Mix et l'accueil, jamais dans la barre latérale.
ALTER TABLE playlists ADD COLUMN is_mix INTEGER NOT NULL DEFAULT 0;
