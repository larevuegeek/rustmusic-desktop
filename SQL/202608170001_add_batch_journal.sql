-- Journal des lots, pour pouvoir revenir en arrière.
--
-- Sans lui, un déplacement interrompu au milieu de trois mille fichiers laisse
-- la bibliothèque à cheval sur deux arborescences, et rien ne dit où elle en
-- est. Ce n'est donc pas du confort : c'est ce qui permet d'oser lancer un lot.

CREATE TABLE batch_journal (
  id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
  library_id INTEGER,
  -- 'rename' | 'restructure'
  kind TEXT NOT NULL,
  -- Le motif employé, pour reconnaître un lot dans la liste.
  pattern TEXT,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  total INTEGER NOT NULL DEFAULT 0,
  succeeded INTEGER NOT NULL DEFAULT 0,
  failed INTEGER NOT NULL DEFAULT 0,
  -- Renseigné une fois le lot annulé — un lot ne s'annule pas deux fois.
  undone_at DATETIME,
  FOREIGN KEY (library_id) REFERENCES library(id) ON DELETE SET NULL
);

CREATE TABLE batch_journal_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  batch_id TEXT NOT NULL,
  before_path TEXT NOT NULL,
  after_path TEXT NOT NULL,
  -- Date de modification du fichier **juste après** l'opération. Si elle a
  -- changé depuis, le fichier a été retouché entre-temps : l'annuler le
  -- ramènerait à son ancien nom en laissant croire qu'on a tout remis en état.
  modified_at INTEGER,
  -- 'done' | 'failed' | 'undone'
  status TEXT NOT NULL DEFAULT 'done',
  error TEXT,
  FOREIGN KEY (batch_id) REFERENCES batch_journal(id) ON DELETE CASCADE
);

CREATE INDEX idx_batch_journal_items_batch ON batch_journal_items(batch_id);
CREATE INDEX idx_batch_journal_created ON batch_journal(created_at DESC);
