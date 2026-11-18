-- Artistes multiples : « A, B » ou « A/B » est-il un groupe ou plusieurs artistes ? Une règle par nom.
CREATE TABLE artist_check_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  finished_at DATETIME NULL,
  checked INTEGER NOT NULL DEFAULT 0,
  -- Interrompue (hors connexion…) : le reste sera vérifié la fois suivante.
  error TEXT NULL
);

CREATE TABLE artist_split_rules (
  -- Nom en minuscules, sans espaces autour.
  name_key TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  -- whole (un seul artiste) | split (plusieurs) | unknown (à vérifier)
  decision TEXT NOT NULL,
  -- Les artistes, en JSON ; pour « unknown », la découpe proposée.
  parts TEXT NULL,
  -- tag (ARTISTS) | musicbrainz | user
  source TEXT NOT NULL,
  run_id INTEGER NULL REFERENCES artist_check_runs(id) ON DELETE SET NULL,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_artist_split_rules_run ON artist_split_rules (run_id);
