# musicopy

## Protocol
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

Connections:
- Open QUIC connection
- Open bidirectional stream
- Client sends Identify
- Server sends Identify
- Server waits for user to accept/deny connection
- Server sends Accept
- Server sends Index
- Client can open more bidirectional streams for downloading

## TODO
- Can we use Cargo features for desktop/mobile to not ship unused stuff to mobile?
    - Bindings are generated based on one file, so the interface should be the same... maybe just stubs if the desktop feature is disabled?
- Or can we easily build two separate crates for desktop/mobile? Should desktop just be its own codebase?
