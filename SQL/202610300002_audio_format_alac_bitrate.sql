-- Un ALAC scanné avant 0.2.8 n'a pas de profondeur : 202610300001 l'a pris pour de l'AAC. Son débit le trahit (l'AAC plafonne à 320 kb/s).
UPDATE library_cache SET audio_format = 'ALAC'
WHERE audio_format = 'AAC' AND COALESCE(bits_per_sample, 0) = 0 AND bitrate >= 400
  AND path NOT LIKE '%://%'
  AND (LOWER(path) LIKE '%.m4a' OR LOWER(path) LIKE '%.m4b' OR LOWER(path) LIKE '%.mp4');

-- Le prochain rescan relit ces fichiers : format et profondeur exacts, quoi qu'aient deviné ces migrations.
UPDATE library_files SET modified_at = NULL
WHERE path IN (
    SELECT path FROM library_cache
    WHERE audio_format IN ('ALAC', 'AAC', 'AIFF') AND COALESCE(bits_per_sample, 0) = 0 AND path NOT LIKE '%://%'
);
