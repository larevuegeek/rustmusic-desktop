# Privacy policy

RustMusic is a local music player. Your library, playlists, listening history and
settings stay on your computer, in RustMusic's data folder. RustMusic has **no
analytics, no telemetry and no advertising**, and never uploads your music files.

It only contacts other systems in the cases listed below. Each one either happens
because you asked for it, or can be turned off in the settings.

## Automatic, can be turned off

| What | Sent to | Data sent | Turn off |
| --- | --- | --- | --- |
| Update check, a few seconds after startup | `rustmusic.dev` | RustMusic version, operating system and processor architecture | Settings → General → *Check for updates* |
| Artist portraits for the artists of your library | Deezer, Wikipedia / Wikimedia | Artist names | Settings → Network & DLNA → *Artist portrait downloads* |
| Artist check, if you enable it (off by default) | MusicBrainz | Artist names | Settings → Storage & data |

## Only when you ask for it

| What | Sent to | Data sent |
| --- | --- | --- |
| Synced lyrics, when you show the lyrics of a track | LRCLIB | Artist, title, album and duration of that track |
| Tag lookups and cover search in the tag workshop | MusicBrainz, Cover Art Archive, Deezer, iTunes | The artist, album or title you search for |
| Podcasts | The feeds you subscribe to; the podcast directory (iTunes) when you search it | Your search terms; the feed requests |
| Internet radio | radio-browser.info when you browse the directory; the stations you play | Your search terms and filters; the stream requests |
| Scrobbling, once you connect an account | Last.fm, ListenBrainz | For each listened track: artist, title, album, duration and time |
| Media servers you add | Your Jellyfin, Navidrome or Subsonic server | What that server needs to browse and stream your library |
| Network players, DLNA server, AirPlay, amplifier remote, once enabled | Devices on your **local network** only | The tracks you choose to play and the commands you send |
| Feedback and bug reports | GitHub or e-mail, opened by you | What you write |

Each of these services has its own privacy policy, which applies to what it receives.
Every network request also reveals your IP address to the server it reaches.

## Contact

Questions about this policy: [contact@rustmusic.dev](mailto:contact@rustmusic.dev).
