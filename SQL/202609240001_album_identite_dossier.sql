-- L'identité d'un album ne peut pas dépendre de son artiste : sur une
-- compilation sans tag « artiste de l'album », chaque piste fondait le sien
-- (52 fiches pour un seul disque). Elle repose désormais sur le titre et le
-- dossier, `album_dir`, rempli et consolidé au démarrage par `album_merge`.
--
-- Changer la contrainte impose de reconstruire la table. Or sqlx enveloppe
-- chaque migration dans une transaction, où `PRAGMA foreign_keys` ne fait
-- rien : le `DROP TABLE` exécute donc les actions `ON DELETE` et emporte le
-- lien piste → album ainsi que `library_album_artists`. Essayé et mesuré :
-- 14 085 pistes détachées. `defer_foreign_keys` et `legacy_alter_table` n'y
-- changent rien. On sauvegarde donc ces deux liens et on les remet.

CREATE TABLE _sauv_piste_album AS
  SELECT id, library_album_id FROM library_tracks WHERE library_album_id IS NOT NULL;

-- `CREATE TABLE AS` n'indexe rien : la restauration ci-dessous relisait les
-- 14 085 lignes pour chacune des 14 085 pistes. 6,1 s contre 0,18 s avec.
CREATE UNIQUE INDEX _sauv_piste_album_id ON _sauv_piste_album(id);

CREATE TABLE _sauv_album_artiste AS SELECT * FROM library_album_artists;

-- Les déclencheurs de comptage fausseraient les totaux pendant l'opération, et
-- ceux de `library_tracks` citent une table momentanément absente.
DROP TRIGGER IF EXISTS trg_library_albums_insert;
DROP TRIGGER IF EXISTS trg_library_albums_delete;
DROP TRIGGER IF EXISTS trg_library_tracks_insert;
DROP TRIGGER IF EXISTS trg_library_tracks_delete;
DROP TRIGGER IF EXISTS trg_library_album_artists_insert;
DROP TRIGGER IF EXISTS trg_library_album_artists_delete;

CREATE TABLE library_albums_neuf (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  library_id INTEGER NOT NULL,
  artist_id TEXT NOT NULL,
  title TEXT NOT NULL,
  title_normalized TEXT NOT NULL,
  album_dir TEXT,
  year INTEGER,
  genre TEXT,
  cover_url TEXT,
  musicbrainz_id TEXT UNIQUE,
  album_type TEXT DEFAULT 'album',
  total_tracks INTEGER DEFAULT 0,
  total_duration REAL DEFAULT 0,
  notes TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (library_id) REFERENCES library(id) ON DELETE CASCADE,
  FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE SET NULL,
  UNIQUE (library_id, title_normalized, album_dir)
);

-- `album_dir` reste vide ici : les doublons d'aujourd'hui buteraient sinon sur
-- la contrainte. NULL n'égale jamais NULL, donc la copie passe entière.
INSERT INTO library_albums_neuf (
  id, library_id, artist_id, title, title_normalized, year, genre, cover_url,
  musicbrainz_id, album_type, total_tracks, total_duration, notes,
  created_at, updated_at
)
SELECT
  id, library_id, artist_id, title, title_normalized, year, genre, cover_url,
  musicbrainz_id, album_type, total_tracks, total_duration, notes,
  created_at, updated_at
FROM library_albums;

DROP TABLE library_albums;
ALTER TABLE library_albums_neuf RENAME TO library_albums;

-- ─── Les liens que le DROP a effacés ───
UPDATE library_tracks
SET library_album_id = (SELECT s.library_album_id FROM _sauv_piste_album s WHERE s.id = library_tracks.id)
WHERE id IN (SELECT id FROM _sauv_piste_album);

INSERT INTO library_album_artists SELECT * FROM _sauv_album_artiste;

DROP TABLE _sauv_piste_album;
DROP TABLE _sauv_album_artiste;

CREATE INDEX idx_library_albums_artist ON library_albums(artist_id);
CREATE INDEX idx_library_albums_library ON library_albums(library_id);
CREATE INDEX idx_library_albums_year ON library_albums(year);
CREATE INDEX idx_library_albums_library_title ON library_albums(library_id, title_normalized);

-- ─── Les déclencheurs, repris à l'identique ───

CREATE TRIGGER trg_library_albums_insert
AFTER INSERT ON library_albums
FOR EACH ROW
BEGIN
  -- Update library
  UPDATE library
  SET total_albums = total_albums + 1
  WHERE id = NEW.library_id;

  -- Update library_artists (exemple)
  UPDATE library_artists
  SET total_albums = total_albums + 1
  WHERE library_id = NEW.library_id
  AND artist_id = NEW.artist_id;
END;

CREATE TRIGGER trg_library_albums_delete
AFTER DELETE ON library_albums
FOR EACH ROW
BEGIN
  -- Update library
  UPDATE library
  SET total_albums = total_albums - 1
  WHERE id = OLD.library_id;

  -- Mise à jour de library_artists
  UPDATE library_artists
  SET total_albums = total_albums - 1
  WHERE library_id = OLD.library_id
    AND artist_id = OLD.artist_id;
END;

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
