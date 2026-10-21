-- Chemin lisible de l'épisode dans le cache (« Émission/2026-10-21 Titre.mp3 »), fixé une fois :
-- il identifie l'épisode pour le lecteur, la file d'attente et la reprise.
ALTER TABLE podcast_episodes ADD COLUMN local_path TEXT NULL;
CREATE INDEX idx_podcast_episodes_local_path ON podcast_episodes(local_path);
