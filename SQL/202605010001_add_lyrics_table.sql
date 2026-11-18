-- =========================
-- lyrics
-- =========================
-- Table dédiée pour les paroles synchronisées et statiques.
-- Séparée de library_tracks pour ne pas alourdir cette table très requêtée
-- (les paroles peuvent peser 5-15 KB par morceau, soit potentiellement
-- plusieurs centaines de Mo sur une grosse bibliothèque).
--
-- Cascade : si un morceau est supprimé, ses paroles partent avec lui.
-- Si l'utilisateur réimporte plus tard, on re-fetchera.
CREATE TABLE IF NOT EXISTS lyrics (
    track_id      TEXT PRIMARY KEY,
    plain         TEXT,
    synced        TEXT,
    source        TEXT NOT NULL,
    fetched_at    INTEGER NOT NULL,
    lrclib_id     INTEGER,
    FOREIGN KEY (track_id) REFERENCES library_tracks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_lyrics_source ON lyrics(source);
CREATE INDEX IF NOT EXISTS idx_lyrics_fetched_at ON lyrics(fetched_at);
