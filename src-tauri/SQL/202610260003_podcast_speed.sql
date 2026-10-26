-- Vitesse de lecture propre à une émission (NULL : la vitesse par défaut des réglages).
ALTER TABLE podcasts ADD COLUMN playback_speed REAL NULL;
