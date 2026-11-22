-- Un MP3 portant un ID3v1 en plus de son ID3v2 était lu par son ID3v1 : lettres hors
-- ISO-8859-1 remplacées par « ? », champs coupés à 30 caractères, ni pochette ni Replay Gain.
-- Les MP3 seront relus au prochain rescan (date effacée = « changé »).
UPDATE library_files
SET modified_at = NULL
WHERE extension = 'mp3';
