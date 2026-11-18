-- Émission retirée de « À écouter » : revient au prochain épisode paru après, ou à la reprise d'un épisode.
ALTER TABLE podcasts ADD COLUMN listen_hidden_at DATETIME NULL;
