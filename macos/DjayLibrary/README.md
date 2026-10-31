# DjayLibrary

Read-only model of djay Pro's media library for the S8 screens.

- `DjayLibrarySnapshot.load(databaseURL:)` copies `MediaLibrary.db` with its `-wal` and `-shm` into a temp directory, opens the copy read-only with the system SQLite, and removes it afterwards. The live files are never opened.
- `DjayLibrarySnapshot.defaultDatabaseURL` points at `~/Music/djay/djay Media Library.djayMediaLibrary/MediaLibrary.db`.
- Values: `tracks` (`DjayTrack`: id, title, artist, album, genre, BPM, Camelot key, duration, path), `playlists` (ordered `trackIDs`, folder `childIDs`), `queue`, and `historySessions` (plays ordered by start time, resolved to library tracks by title ID, ISRC, then normalized title and artist).
- `coPlayNeighbours(of:limit:)` returns the tracks played immediately before and after a track within the same history session, ranked by play count then recency.
- Blobs are decoded by a port of djaykit's TSAF decoder (`TSAFDecoder.swift`) and a small `NSKeyedArchiver` reader (`KeyedArchive.swift`).

Playlist items and queue items are covered by synthetic tests only; the reference library has none. Decoding is checked against djaykit output using fixtures extracted from a real library (`DjayLibraryTests/Fixtures`).
