# musicopy

- User adds library paths to server
- Server recursively scans library paths
- Server hashes library files
- Server stores list of files in SQLite
- Server transcodes files in background (?)
- Client connects to server
- Client receives list of hashes
- Client opens streams to download missing hashes
- Client saves downloaded files to structured paths
- Client stores list of hashes in SQLite

SQLite local files table:
- Hash Kind
- Hash
- Root
- Path

SQLite remote files table:
- Hash Kind
- Hash
- Remote Node ID
- Remote Root
- Remote Path
