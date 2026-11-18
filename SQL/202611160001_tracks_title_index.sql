-- Page des morceaux triée par titre : chaque fenêtre (LIMIT … OFFSET …) parcourait toute la
-- bibliothèque et la triait, jointures comprises (≈ 400 ms pour 10 000 titres). L'index
-- donne directement l'ordre des titres d'une bibliothèque (≈ 75 ms loin dans la liste).
CREATE INDEX IF NOT EXISTS idx_lt_library_title ON library_tracks(library_id, title_normalized);
