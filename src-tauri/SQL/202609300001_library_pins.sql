-- Albums et artistes épinglés dans la barre latérale, par bibliothèque.
-- Une ligne vise l'un ou l'autre, jamais les deux : d'où le CHECK.

CREATE TABLE library_pins (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  library_id INTEGER NOT NULL,
  library_album_id TEXT NULL,
  artist_id TEXT NULL,
  position INTEGER NOT NULL DEFAULT 0,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (library_id) REFERENCES library(id) ON DELETE CASCADE,
  FOREIGN KEY (library_album_id) REFERENCES library_albums(id) ON DELETE CASCADE,
  FOREIGN KEY (artist_id) REFERENCES artists(id) ON DELETE CASCADE,
  CHECK ((library_album_id IS NULL) <> (artist_id IS NULL)),
  UNIQUE (library_id, library_album_id),
  UNIQUE (library_id, artist_id)
);

CREATE INDEX idx_library_pins_library ON library_pins(library_id);
