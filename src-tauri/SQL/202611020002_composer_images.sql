-- Photos des compositeurs (tag composer), qui n'ont pas de fiche dans `artists`.
CREATE TABLE composer_images (
  -- Nom normalisé, comme `artists.name_normalized`.
  name_key TEXT PRIMARY KEY,
  -- Chemin de la photo ; "" = cherchée sans résultat.
  image_url TEXT NOT NULL,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
