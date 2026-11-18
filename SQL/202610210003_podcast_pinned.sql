-- Podcasts favoris : épinglés dans la barre latérale et en tête de « Mes podcasts ».
ALTER TABLE podcasts ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
