-- Titres passés à l'antenne pendant l'écoute d'une radio (titre ICY entendu), les plus récents gardés.

CREATE TABLE radio_titles (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  profil_id INTEGER NOT NULL,
  station TEXT NOT NULL,
  stream_url TEXT NOT NULL,
  title TEXT NOT NULL,
  played_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (profil_id) REFERENCES profil(id) ON DELETE CASCADE
);

CREATE INDEX idx_radio_titles_profil ON radio_titles(profil_id, id DESC);
