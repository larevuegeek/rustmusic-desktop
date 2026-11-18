-- =========================
-- Bibliothèque par défaut
-- =========================
--
-- Jusqu'ici, la bibliothèque ouverte au démarrage était simplement la première
-- de la liste. Impossible d'en désigner une : un profil qui a une bibliothèque
-- « Archives » avant sa bibliothèque principale tombait dessus à chaque
-- lancement.
--
-- Le marqueur est **par profil** : chaque profil a ses propres bibliothèques,
-- donc son propre défaut.

ALTER TABLE library ADD COLUMN is_default INTEGER NOT NULL DEFAULT 0;

-- Reprise de l'existant : on promeut celle qui était déjà choisie de fait,
-- c'est-à-dire la première dans l'ordre d'affichage. Sans ça, personne n'aurait
-- de défaut tant qu'il n'en aurait pas désigné un à la main, et le comportement
-- changerait sans raison visible.
UPDATE library
SET is_default = 1
WHERE id IN (
    SELECT (
        SELECT id
        FROM library AS candidate
        WHERE candidate.profil_id = profils.profil_id
        ORDER BY position ASC, id ASC
        LIMIT 1
    )
    FROM (SELECT DISTINCT profil_id FROM library) AS profils
);

-- L'unicité est garantie par le schéma, pas par une convention que le code
-- devrait se rappeler d'appliquer. Un index partiel ne contraint que les lignes
-- marquées : il autorise autant de non-défauts qu'on veut, et un seul défaut
-- par profil.
DROP INDEX IF EXISTS "uniq_library_default_per_profil";
CREATE UNIQUE INDEX uniq_library_default_per_profil
ON library(profil_id) WHERE is_default = 1;
