-- =========================
-- _sqlx_migrations
-- =========================
DROP TABLE IF EXISTS "_sqlx_migrations";
CREATE TABLE _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    success BOOLEAN NOT NULL,
    checksum BLOB NOT NULL,
    execution_time BIGINT NOT NULL
);

-- =========================
-- artists
-- =========================
DROP TABLE IF EXISTS "artists";
CREATE TABLE artists (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  name TEXT NOT NULL,
  name_normalized TEXT NOT NULL,
  sort_name TEXT NOT NULL,
  bio TEXT,
  image_url TEXT,
  musicbrainz_id TEXT UNIQUE,
  country TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

DROP INDEX IF EXISTS "idx_artists_name_normalized";
CREATE UNIQUE INDEX idx_artists_name_normalized
ON artists(name_normalized);

DROP INDEX IF EXISTS "idx_artists_musicbrainz";
CREATE INDEX idx_artists_musicbrainz ON artists(musicbrainz_id);

-- =========================
-- library
-- =========================
DROP TABLE IF EXISTS "library";
CREATE TABLE library (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profil_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT NULL,
    cover TEXT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    total_tracks INTEGER NOT NULL DEFAULT 0,
    total_albums INTEGER NOT NULL DEFAULT 0,
    total_artists INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NULL,
    FOREIGN KEY (profil_id) REFERENCES profil(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE
);

DROP INDEX IF EXISTS "idx_library_position";
CREATE INDEX idx_library_position ON library(position);

DROP INDEX IF EXISTS "idx_library_profil_id";
CREATE INDEX idx_library_profil_id ON library(profil_id);

DROP INDEX IF EXISTS "uniq_library_name_per_profil";
CREATE UNIQUE INDEX uniq_library_name_per_profil
ON library(profil_id, name);

-- =========================
-- library_albums
-- =========================
DROP TABLE IF EXISTS "library_albums";
CREATE TABLE library_albums (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  library_id INTEGER NOT NULL,
  artist_id TEXT NOT NULL,
  title TEXT NOT NULL,
  title_normalized TEXT NOT NULL,
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
  UNIQUE (library_id, artist_id, title_normalized)
);

DROP INDEX IF EXISTS "idx_library_albums_artist";
CREATE INDEX idx_library_albums_artist ON library_albums(artist_id);

DROP INDEX IF EXISTS "idx_library_albums_library";
CREATE INDEX idx_library_albums_library ON library_albums(library_id);

DROP INDEX IF EXISTS "idx_library_albums_year";
CREATE INDEX idx_library_albums_year ON library_albums(year);

DROP INDEX IF EXISTS "idx_library_albums_library_title";
CREATE INDEX idx_library_albums_library_title ON library_albums(library_id, title_normalized);

-- =========================
-- library_albums_artists
-- =========================
DROP TABLE IF EXISTS library_album_artists;

CREATE TABLE library_album_artists (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  library_id INTEGER NOT NULL,
  library_album_id TEXT NOT NULL,
  artist_id TEXT NOT NULL,
  role TEXT DEFAULT 'primary', -- primary | featured | composer | etc

  FOREIGN KEY (library_album_id) REFERENCES library_albums(id) ON DELETE CASCADE,
  FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE,

  UNIQUE(library_album_id, artist_id)
);

DROP INDEX IF EXISTS "idx_laa_library_artist_album";
CREATE INDEX idx_laa_library_artist_album
ON library_album_artists(library_id, artist_id, library_album_id);

DROP INDEX IF EXISTS "idx_laa_album_id";
CREATE INDEX idx_laa_album_id
ON library_album_artists(library_album_id);

-- =========================
-- library_artists
-- =========================
DROP TABLE IF EXISTS "library_artists";
CREATE TABLE library_artists (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  library_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,
  total_albums INTEGER DEFAULT 0,
  total_tracks INTEGER DEFAULT 0,
  total_duration REAL DEFAULT 0,
  custom_image_url TEXT,
  notes TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (library_id) REFERENCES library(id) ON DELETE CASCADE,
  FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE,
  UNIQUE(library_id, artist_id)
);

DROP INDEX IF EXISTS "idx_library_artists_artist";
CREATE INDEX idx_library_artists_artist ON library_artists(artist_id);

DROP INDEX IF EXISTS "idx_library_artists_created_at";
CREATE INDEX idx_library_artists_created_at ON library_artists(created_at);

DROP INDEX IF EXISTS "idx_library_artists_library";
CREATE INDEX idx_library_artists_library ON library_artists(library_id);

-- =========================
-- library_cache
-- =========================
DROP TABLE IF EXISTS "library_cache";
CREATE TABLE library_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    title TEXT,
    artist TEXT,
    album TEXT,
    album_artist TEXT,
    year TEXT,
    genre TEXT,
    track_number INTEGER,
    disc_number INTEGER,
    duration REAL,
    bitrate INTEGER,
    bits_per_sample INTEGER,
    sample_rate INTEGER,
    channels INTEGER,
    audio_format TEXT,
    mime_type TEXT,
    file_size INTEGER,
    extra_tags TEXT,
    thumbnail_path TEXT,
    last_scanned_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

DROP INDEX IF EXISTS "idx_library_title";
CREATE INDEX idx_library_title ON library_cache(title);

DROP INDEX IF EXISTS "idx_library_artist";
CREATE INDEX idx_library_artist ON library_cache(artist);

DROP INDEX IF EXISTS "idx_library_album";
CREATE INDEX idx_library_album ON library_cache(album);

-- =========================
-- library_dirs
-- =========================
DROP TABLE IF EXISTS "library_dirs";
CREATE TABLE library_dirs (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  library_id INTEGER NOT NULL,
  path TEXT NOT NULL,
  name TEXT NOT NULL,
  is_recursive BOOLEAN DEFAULT 1,
  is_active BOOLEAN DEFAULT 1,
  watch_enabled BOOLEAN DEFAULT 1,
  include_patterns TEXT,
  exclude_patterns TEXT,
  total_files INTEGER DEFAULT 0,
  total_size INTEGER DEFAULT 0,
  last_scan_at DATETIME,
  scan_status TEXT DEFAULT 'pending',
  scan_error TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (library_id) REFERENCES library(id) ON DELETE CASCADE,
  UNIQUE(library_id, path)
);

DROP INDEX IF EXISTS "idx_library_dirs_active";
CREATE INDEX idx_library_dirs_active ON library_dirs(library_id, is_active);

DROP INDEX IF EXISTS "idx_library_dirs_library";
CREATE INDEX idx_library_dirs_library ON library_dirs(library_id);

-- =========================
-- library_files
-- =========================
DROP TABLE IF EXISTS "library_files";
CREATE TABLE library_files (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  library_id INTEGER NOT NULL,
  library_dir_id TEXT,
  cache_id INTEGER,
  path TEXT NOT NULL,
  filename TEXT NOT NULL,
  extension TEXT,
  size INTEGER,
  file_hash TEXT,
  modified_at DATETIME,
  status TEXT DEFAULT 'pending',
  is_available BOOLEAN DEFAULT 1,
  error_message TEXT,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  last_verified_at DATETIME,
  FOREIGN KEY (library_id) REFERENCES library(id) ON DELETE CASCADE,
  FOREIGN KEY (library_dir_id) REFERENCES library_dirs(id) ON DELETE CASCADE,
  FOREIGN KEY (cache_id) REFERENCES library_cache(id) ON DELETE SET NULL,
  UNIQUE (library_id, path)
);

DROP INDEX IF EXISTS "idx_library_files_hash";
CREATE INDEX idx_library_files_hash ON library_files(file_hash);

DROP INDEX IF EXISTS "idx_library_files_library";
CREATE INDEX idx_library_files_library ON library_files(library_id);

DROP INDEX IF EXISTS "idx_library_files_parent";
CREATE INDEX idx_library_files_parent ON library_files(library_dir_id);

DROP INDEX IF EXISTS "idx_library_files_status";
CREATE INDEX idx_library_files_status ON library_files(library_id, status);

-- =========================
-- library_tracks
-- =========================
DROP TABLE IF EXISTS "library_tracks";
CREATE TABLE library_tracks (
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
  rating INTEGER,
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

DROP INDEX IF EXISTS "idx_library_tracks_album";
CREATE INDEX idx_library_tracks_album ON library_tracks(library_album_id);

DROP INDEX IF EXISTS "idx_library_tracks_artist";
CREATE INDEX idx_library_tracks_artist ON library_tracks(artist_id);

DROP INDEX IF EXISTS "idx_library_tracks_library";
CREATE INDEX idx_library_tracks_library ON library_tracks(library_id);

DROP INDEX IF EXISTS "idx_library_tracks_played";
CREATE INDEX idx_library_tracks_played ON library_tracks(last_played_at);

DROP INDEX IF EXISTS "idx_library_tracks_title";
CREATE INDEX idx_library_tracks_title ON library_tracks(title_normalized);


-- =========================
-- library_tracks_artists
-- =========================
CREATE TABLE library_track_artists (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),

    library_id INTEGER NOT NULL,
    library_track_id TEXT NOT NULL,
    artist_id TEXT NOT NULL,

    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (library_id) REFERENCES library(id) ON DELETE CASCADE,
    FOREIGN KEY (library_track_id) REFERENCES library_tracks(id) ON DELETE CASCADE,
    FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE,

    -- Empêche les doublons track ↔ artist
    UNIQUE (library_track_id, artist_id)
);

CREATE INDEX idx_lta_track_id
ON library_track_artists (library_track_id);

CREATE INDEX idx_lta_artist_id
ON library_track_artists (artist_id);

CREATE INDEX idx_lta_library_id
ON library_track_artists (library_id);

CREATE INDEX idx_lta_library_track
ON library_track_artists (library_id, library_track_id);

-- =========================
-- playlist
-- =========================
DROP TABLE IF EXISTS "playlists";
CREATE TABLE playlists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- relations
    profil_id INTEGER NOT NULL,
    library_id INTEGER NULL, -- optionnel (playlist dans une library)

    -- données
    name TEXT NOT NULL,
    description TEXT NULL,

    -- apparence
    color TEXT NOT NULL DEFAULT '#8b5cf6',
    icon TEXT NOT NULL DEFAULT 'mynaui:music',
    cover TEXT NULL,

    -- stats simples
    track_count INTEGER NOT NULL DEFAULT 0,
    duration INTEGER NOT NULL DEFAULT 0, -- en secondes

    -- ordre / tri
    position INTEGER NOT NULL DEFAULT 0,

    -- timestamps
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NULL,

    FOREIGN KEY (profil_id) REFERENCES profil(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE,

    FOREIGN KEY (library_id) REFERENCES library(id)
        ON DELETE SET NULL
        ON UPDATE CASCADE
);

-- =========================
-- playlist_items
-- =========================
DROP TABLE IF EXISTS "playlist_items";
CREATE TABLE playlist_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    playlist_id INTEGER NOT NULL,
    library_track_id TEXT NOT NULL,

    sort_index INTEGER NOT NULL DEFAULT 0,

    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (playlist_id) REFERENCES playlists(id)
        ON DELETE CASCADE,

    FOREIGN KEY (library_track_id) REFERENCES library_tracks(id)
        ON DELETE CASCADE,

    UNIQUE (playlist_id, library_track_id)
);

DROP INDEX IF EXISTS "idx_playlist_items_playlist";
CREATE INDEX idx_playlist_items_playlist
    ON playlist_items(playlist_id);

DROP INDEX IF EXISTS "idx_playlist_items_sort";
CREATE INDEX idx_playlist_items_sort
    ON playlist_items(playlist_id, sort_index);


-- =========================
-- profil
-- =========================
DROP TABLE IF EXISTS "profil";
CREATE TABLE profil (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- identité
    name TEXT NOT NULL,
    avatar TEXT NULL,
    color TEXT NOT NULL DEFAULT '#22c55e',

    -- infos complémentaires
    bio TEXT NULL,
    is_active INTEGER NOT NULL DEFAULT 1, -- 0 / 1
    role TEXT NOT NULL DEFAULT 'user',     -- user | admin | guest

    -- timestamps
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NULL
);

DROP INDEX IF EXISTS "idx_profil_active";
CREATE INDEX idx_profil_active ON profil(is_active);

DROP INDEX IF EXISTS "idx_profil_name";
CREATE INDEX idx_profil_name ON profil(name);

-- INSERT PROFIL PAR DEFAULT
INSERT INTO profil (id, name, role, is_active)
VALUES (1, 'Default', 'admin', 1);

-- =========================
-- Queue
-- =========================
CREATE TABLE IF NOT EXISTS queue_state (
    profil_id INTEGER PRIMARY KEY,
    current_index INTEGER NOT NULL DEFAULT -1,
    is_shuffled BOOLEAN NOT NULL DEFAULT 0,
    repeat_mode TEXT NOT NULL DEFAULT 'off',
    FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE
);

-- On initialise la ligne par défaut
INSERT OR IGNORE INTO queue_state (profil_id, current_index, is_shuffled, repeat_mode) 
VALUES (1, -1, 0, 'off');

-- 2. Table des pistes
CREATE TABLE IF NOT EXISTS queue_tracks (
    queue_id TEXT PRIMARY KEY,
    profil_id INTEGER NOT NULL DEFAULT 1,
    position INTEGER NOT NULL,
    path TEXT NOT NULL,
    title TEXT NOT NULL,
    artist TEXT,
    duration REAL,
    cover TEXT,
    FOREIGN KEY (profil_id) REFERENCES queue_state(profil_id) ON DELETE CASCADE
);

-- 3. L'index optimisé pour charger la file ultra vite
CREATE INDEX IF NOT EXISTS idx_queue_tracks_profil_position 
ON queue_tracks(profil_id, position);
-- =========================
-- recent_files
-- =========================
DROP TABLE IF EXISTS "recent_files";
CREATE TABLE recent_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    library_id INTEGER NOT NULL,
    path TEXT NOT NULL UNIQUE,
    last_played_at DATETIME NOT NULL,
    last_position REAL DEFAULT 0,
    play_count INTEGER DEFAULT 1,

    FOREIGN KEY(library_id) REFERENCES library_cache(id)
        ON DELETE CASCADE
);

DROP INDEX IF EXISTS "idx_recent_files_last_played";
CREATE INDEX idx_recent_files_last_played
    ON recent_files(last_played_at DESC);

DROP INDEX IF EXISTS "idx_recent_files_library";
CREATE INDEX idx_recent_files_library
    ON recent_files(library_id);

DROP INDEX IF EXISTS "idx_recent_files_path";
CREATE INDEX idx_recent_files_path
    ON recent_files(path);

-- =========================
-- settings
-- =========================
DROP TABLE IF EXISTS "settings";
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT
);

-- =========================
-- track_liked
-- =========================
DROP TABLE IF EXISTS "track_liked";
CREATE TABLE track_liked (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  profil_id INTEGER NOT NULL,
  library_cache_id INTEGER,
  path TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE,
  FOREIGN KEY (library_cache_id) REFERENCES library_cache(id) ON DELETE SET NULL,

  UNIQUE(profil_id, path)
);


-- =========================
-- TRIGGERS
-- =========================
DROP TRIGGER IF EXISTS "prevent_delete_admin_profil";
CREATE TRIGGER prevent_delete_admin_profil
BEFORE DELETE ON profil
FOR EACH ROW
WHEN OLD.id = 1
BEGIN
    SELECT RAISE(ABORT, 'Impossible de supprimer le profil administrateur par défaut');
END;

CREATE TRIGGER IF NOT EXISTS prevent_delete_default_queue_state
BEFORE DELETE ON queue_state
FOR EACH ROW
WHEN OLD.profil_id = 1
BEGIN
    SELECT RAISE(ABORT, 'Action interdite : Impossible de supprimer la config du profil par défaut');
END;

DROP TRIGGER IF EXISTS "trg_playlists_update";
CREATE TRIGGER trg_playlists_update 
AFTER UPDATE ON playlists
FOR EACH ROW
BEGIN
    UPDATE playlists
    SET updated_at = CURRENT_TIMESTAMP
    WHERE id = NEW.id;
END;


-- TRIGGERS FOR LIBRARY
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

CREATE TRIGGER trg_library_artists_insert
AFTER INSERT ON library_artists
FOR EACH ROW
BEGIN
  UPDATE library
  SET total_artists = total_artists + 1
  WHERE id = NEW.library_id;
END;

CREATE TRIGGER trg_library_artists_delete
AFTER DELETE ON library_artists
FOR EACH ROW
BEGIN
  UPDATE library
  SET total_artists = total_artists - 1
  WHERE id = OLD.library_id;
END;

DROP TRIGGER IF EXISTS trg_library_album_artists_insert;

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

DROP TRIGGER IF EXISTS trg_library_album_artists_delete;

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

