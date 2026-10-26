-- Doublon de idx_recent_files_profile (profil, source, date) : les récents sont toujours lus par profil.
DROP INDEX IF EXISTS idx_rf_last_played;
