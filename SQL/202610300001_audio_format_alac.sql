-- ALAC, AAC et AIFF étaient enregistrés « Unknown » (affichés avec perte) : rattrapage sans relire les fichiers.
-- Dans un .m4a, une profondeur connue signale l'ALAC ; l'AAC n'en a pas.
UPDATE library_cache SET audio_format = CASE
    WHEN LOWER(path) LIKE '%.aif' OR LOWER(path) LIKE '%.aiff' OR LOWER(path) LIKE '%.aifc' THEN 'AIFF'
    WHEN LOWER(path) LIKE '%.wav' THEN 'WAV'
    WHEN bits_per_sample > 0 THEN 'ALAC'
    ELSE 'AAC'
END
WHERE audio_format = 'Unknown'
  AND (LOWER(path) LIKE '%.m4a' OR LOWER(path) LIKE '%.m4b' OR LOWER(path) LIKE '%.mp4' OR LOWER(path) LIKE '%.aac'
       OR LOWER(path) LIKE '%.aif' OR LOWER(path) LIKE '%.aiff' OR LOWER(path) LIKE '%.aifc' OR LOWER(path) LIKE '%.wav');
