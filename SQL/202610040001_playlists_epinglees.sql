-- Playlists affichées dans la barre latérale, comme les albums et artistes épinglés.
-- Toutes le sont au départ : la barre ne perd rien à la mise à jour, et une nouvelle
-- playlist y apparaît d'office.
ALTER TABLE playlists ADD COLUMN pinned INTEGER NOT NULL DEFAULT 1;
