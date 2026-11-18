-- ============================================================================
-- Notation : la demi-étoile, en décimal
-- ============================================================================
--
-- `library_tracks.rating` passe de INTEGER à REAL. Une note vaut désormais 0,5
-- à 5,0 par pas d'un demi : 3.5 se lit « trois étoiles et demie », sans
-- conversion mentale.
--
-- # Aucune donnée n'est transformée
-- Les notes existantes sont des entiers de 1 à 5, qui sont déjà exactement les
-- décimaux 1,0 à 5,0. La migration ne touche donc pas une seule valeur : elle
-- ne change que le type déclaré.
--
-- # Pourquoi le décimal ne coûte rien en exactitude
-- L'objection habituelle au flottant — deux valeurs égales qui ne se comparent
-- pas égales — ne s'applique pas ici. Les dénominateurs sont des puissances de
-- deux : 0,5 · 1,0 · 1,5 … 5,0 sont toutes représentables exactement en IEEE
-- 754. Tri, égalité et regroupement restent donc exacts, au bit près.
--
-- # Pourquoi reconstruire la table plutôt que laisser INTEGER
-- SQLite ne sait pas changer le type d'une colonne en place, et son typage est
-- souple : une colonne déclarée INTEGER accepterait 3.5 sans broncher. Mais
-- elle convertirait 4.0 en entier 4 — la conversion étant sans perte — si bien
-- que `typeof(rating)` vaudrait tantôt 'real', tantôt 'integer' selon la note.
-- Un type qui varie d'une ligne à l'autre est un piège pour tout ce qui lira
-- cette base ensuite. Avec REAL, toutes les notes sont des réels.
--
-- La reconstruction est la procédure documentée par SQLite pour ce cas. Elle
-- est sûre ici : les clés étrangères ne sont jamais activées par l'application,
-- et la nouvelle table reprend le nom d'origine — les trois tables qui
-- référencent `library_tracks(id)` continuent donc de pointer au bon endroit.
-- sqlx enveloppe la migration dans une transaction : en cas d'échec, rien.

-- # L'ordre compte
-- Quatre déclencheurs touchent `library_tracks` : deux posés dessus, qui
-- disparaîtraient avec elle, et deux posés sur `library_album_artists` qui la
-- lisent — ceux-là rendraient le schéma invalide dès la suppression. Tous sont
-- donc retirés d'abord, et rétablis à la fin, mot pour mot.
--
-- Rétablis **après** la copie, et c'est essentiel : `trg_library_tracks_insert`
-- incrémente les compteurs de la bibliothèque, de l'artiste et de l'album à
-- chaque ligne insérée. Recréé trop tôt, il compterait une deuxième fois toute
-- la discothèque.

DROP TRIGGER IF EXISTS trg_library_album_artists_insert;
DROP TRIGGER IF EXISTS trg_library_album_artists_delete;
DROP TRIGGER IF EXISTS trg_library_tracks_insert;
DROP TRIGGER IF EXISTS trg_library_tracks_delete;

CREATE TABLE library_tracks_migration_rating (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  library_id INTEGER NOT NULL,
  file_id TEXT NOT NULL,
  cache_id INTEGER,
  artist_id TEXT,
  library_album_id TEXT,
  title TEXT NOT NULL,
  title_normalized TEXT NOT NULL,
  track_number INTEGER,
  disc_number INTEGER DEFAULT 1,
  tags TEXT,
  duration REAL,
  bitrate INTEGER,
  sample_rate INTEGER,
  play_count INTEGER DEFAULT 0,
  last_played_at DATETIME,
  rating REAL,
  favorite BOOLEAN DEFAULT 0,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (library_id) REFERENCES library(id) ON DELETE CASCADE,
  FOREIGN KEY (file_id) REFERENCES library_files(id) ON DELETE CASCADE,
  FOREIGN KEY (cache_id) REFERENCES library_cache(id) ON DELETE SET NULL,
  FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE SET NULL,
  FOREIGN KEY (library_album_id) REFERENCES library_albums(id) ON DELETE SET NULL,
  UNIQUE (library_id, file_id)
);

-- Colonnes nommées une à une, jamais `SELECT *` : l'ordre des colonnes n'est
-- alors plus une hypothèse silencieuse, et l'ajout d'une colonne ailleurs ne
-- peut pas décaler la copie.
INSERT INTO library_tracks_migration_rating (
  id, library_id, file_id, cache_id, artist_id, library_album_id,
  title, title_normalized, track_number, disc_number, tags,
  duration, bitrate, sample_rate, play_count, last_played_at,
  rating, favorite, created_at, updated_at
)
SELECT
  id, library_id, file_id, cache_id, artist_id, library_album_id,
  title, title_normalized, track_number, disc_number, tags,
  duration, bitrate, sample_rate, play_count, last_played_at,
  rating, favorite, created_at, updated_at
FROM library_tracks;

DROP TABLE library_tracks;

ALTER TABLE library_tracks_migration_rating RENAME TO library_tracks;

-- Les index disparaissent avec la table : on les recrée à l'identique.
--
-- Les cinq premiers viennent de la migration d'initialisation. Les six suivants
-- sont posés au démarrage par `state.rs`, qui s'exécute juste après les
-- migrations et les rétablirait donc de lui-même — mais une migration qui
-- laisse le schéma amputé en comptant sur une étape ultérieure est une
-- migration fausse. On les remet ici, et `IF NOT EXISTS` rend l'ordre
-- indifférent.
CREATE INDEX idx_library_tracks_album ON library_tracks(library_album_id);
CREATE INDEX idx_library_tracks_artist ON library_tracks(artist_id);
CREATE INDEX idx_library_tracks_library ON library_tracks(library_id);
CREATE INDEX idx_library_tracks_played ON library_tracks(last_played_at);
CREATE INDEX idx_library_tracks_title ON library_tracks(title_normalized);

CREATE INDEX IF NOT EXISTS idx_lt_library_id ON library_tracks(library_id);
CREATE INDEX IF NOT EXISTS idx_lt_file_id ON library_tracks(file_id);
CREATE INDEX IF NOT EXISTS idx_lt_cache_id ON library_tracks(cache_id);
CREATE INDEX IF NOT EXISTS idx_lt_play_count ON library_tracks(library_id, play_count DESC);
CREATE INDEX IF NOT EXISTS idx_lt_artist_id ON library_tracks(artist_id);
CREATE INDEX IF NOT EXISTS idx_lt_album_id ON library_tracks(library_album_id);

-- Les déclencheurs, repris à l'identique de la migration d'initialisation.

CREATE TRIGGER trg_library_tracks_insert
AFTER INSERT ON library_tracks
FOR EACH ROW
BEGIN
  -- Update library
  UPDATE library
  SET total_tracks = total_tracks + 1
  WHERE id = NEW.library_id;

  -- Update library_artists
  UPDATE library_artists
  SET total_tracks = total_tracks + 1
  WHERE library_id = NEW.library_id
  AND artist_id = NEW.artist_id;

  -- Update library_albums
  UPDATE library_albums
  SET total_tracks = total_tracks + 1
  WHERE id = NEW.library_album_id;
END;

CREATE TRIGGER trg_library_tracks_delete
AFTER DELETE ON library_tracks
FOR EACH ROW
BEGIN
  -- Update library
  UPDATE library
  SET total_tracks = total_tracks - 1
  WHERE id = OLD.library_id;

  -- Mise à jour de library_artists
  UPDATE library_artists
  SET total_tracks = total_tracks - 1
  WHERE library_id = OLD.library_id
    AND artist_id = OLD.artist_id;

  -- Update library_albums
  UPDATE library_albums
  SET total_tracks = total_tracks - 1
  WHERE id = OLD.library_album_id;
END;

CREATE TRIGGER trg_library_album_artists_insert
AFTER INSERT ON library_album_artists
FOR EACH ROW
BEGIN
  -- incrémente albums pour cet artiste
  UPDATE library_artists
  SET total_albums = total_albums + 1
  WHERE library_id = NEW.library_id
    AND artist_id = NEW.artist_id;

  -- incrémente tracks existantes de l'album
  UPDATE library_artists
  SET total_tracks = total_tracks + (
      SELECT COUNT(*)
      FROM library_tracks
      WHERE library_album_id = NEW.library_album_id
  )
  WHERE library_id = NEW.library_id
    AND artist_id = NEW.artist_id;
END;

CREATE TRIGGER trg_library_album_artists_delete
AFTER DELETE ON library_album_artists
FOR EACH ROW
BEGIN
  -- -1 album
  UPDATE library_artists
  SET total_albums = CASE
      WHEN total_albums > 0 THEN total_albums - 1
      ELSE 0
  END
  WHERE library_id = OLD.library_id
    AND artist_id = OLD.artist_id;

  -- - (nb de tracks dans l'album)
  UPDATE library_artists
  SET total_tracks = CASE
      WHEN total_tracks - (
          SELECT COUNT(*)
          FROM library_tracks
          WHERE library_id = OLD.library_id
            AND library_album_id = OLD.library_album_id
      ) > 0
      THEN total_tracks - (
          SELECT COUNT(*)
          FROM library_tracks
          WHERE library_id = OLD.library_id
            AND library_album_id = OLD.library_album_id
      )
      ELSE 0
  END
  WHERE library_id = OLD.library_id
    AND artist_id = OLD.artist_id;
END;
