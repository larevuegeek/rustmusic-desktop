-- Une piste relue gardait ses anciennes valeurs (tags écrits, puis rescan) : compositeur, titre, artiste figés.
-- Les fichiers modifiés après leur import seront relus au prochain rescan (date effacée = « changé »).
UPDATE library_files
SET modified_at = NULL
WHERE id IN (
  SELECT lf.id
  FROM library_files lf
  JOIN library_tracks lt ON lt.file_id = lf.id
  WHERE CAST(lf.modified_at AS INTEGER) > CAST(strftime('%s', lt.created_at) AS INTEGER)
);
