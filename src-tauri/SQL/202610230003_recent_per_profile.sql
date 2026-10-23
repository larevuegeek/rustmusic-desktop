-- Récents rangés par profil (chacun son historique), et la colonne de l'entrée du cache
-- sous son vrai nom (`library_id` contenait un id de `library_cache`).
CREATE TABLE recent_files_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profil_id INTEGER NOT NULL,
    library_cache_id INTEGER NULL,
    path TEXT NOT NULL,
    last_played_at DATETIME NOT NULL,
    last_position REAL DEFAULT 0,
    play_count INTEGER DEFAULT 1,
    source_id INTEGER NOT NULL DEFAULT 0,
    UNIQUE (profil_id, path),
    FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE,
    FOREIGN KEY (library_cache_id) REFERENCES library_cache(id) ON DELETE CASCADE
);

-- Une écoute va au profil qui possède le fichier, sinon au premier profil.
INSERT INTO recent_files_new (id, profil_id, library_cache_id, path, last_played_at, last_position, play_count, source_id)
SELECT rf.id,
       COALESCE(
           (SELECT l.profil_id FROM library_files lf JOIN library l ON l.id = lf.library_id WHERE lf.path = rf.path LIMIT 1),
           (SELECT MIN(id) FROM profil)),
       (SELECT lc.id FROM library_cache lc WHERE lc.path = rf.path),
       rf.path, rf.last_played_at, rf.last_position, rf.play_count, rf.source_id
FROM recent_files rf
WHERE EXISTS (SELECT 1 FROM profil);

DROP TABLE recent_files;
ALTER TABLE recent_files_new RENAME TO recent_files;

CREATE INDEX idx_recent_files_profile ON recent_files (profil_id, source_id, last_played_at DESC);
CREATE INDEX idx_recent_files_cache ON recent_files (library_cache_id);
