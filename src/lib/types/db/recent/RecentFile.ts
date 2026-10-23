import type { LibraryCache } from "../library/LibraryCache";

export interface RecentFile {
  id: number;
  library_cache_id: number;
  path: string;
  last_played_at: string;
  last_position: number;
  play_count: number;

  library?: LibraryCache | null;

}