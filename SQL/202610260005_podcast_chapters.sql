-- Chapitres : lien du fichier JSON (Podcasting 2.0) et liste connue (flux, JSON ou ID3, en JSON ; '[]' = aucun).
ALTER TABLE podcast_episodes ADD COLUMN chapters_url TEXT NULL;
ALTER TABLE podcast_episodes ADD COLUMN chapters TEXT NULL;
