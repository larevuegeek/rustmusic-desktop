export interface Playlist {
  id: number;
  profil_id: number;
  library_id: number | null;
  name: string;
  description: string | null;
  color: string;
  icon: string;
  cover: string | null;
  track_count: number;
  duration: number;
  position: number;
  /** Vrai si le contenu se calcule à partir de règles au lieu d'être rangé. */
  is_smart: boolean;
  created_at: string;
  updated_at: string | null;
}