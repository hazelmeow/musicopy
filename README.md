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

Idea is to identify roots as (node ID, name).
The path *could* be displayed, we're not concerned about privacy, but we need an identifier which fits in one path segment.
Then files can be downloaded to `musicopy-$node-$root/` to prevent conflicts.
We keep a list of downloaded files for incremental syncing with the remote.

We could also re-serve remote files (ie. mobile to mobile) which would be nice.
Or serve local files from mobile.
We won't do this at first because it's more UI to build,
but if we keep all the logic shared instead of having separate desktop/mobile crates,
it could be possible in the future.
To do this, we might want to remember the original remote paths so we can display them on the new client even if the path for the transfer is a `musicopy-` path.

Not 100% happy with this schema yet

SQLite roots table:
- Node ID
- Name
- Path (on origin)

SQLite files table:
- Hash Kind
- Hash
- Node ID
- Root Name
- Path (without root)
- Local Path (local)

Or with separate tables...?

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
- Path (local)

Database workflows:

Scan:
- 

Serve local files:
- Get list of local files
- Need local paths for transfer
- Send as list of (root name, path without root) and list of (root name, root path)

Re-serve remote files:
- Get list of remote files
- Need local paths for transfer
- Potentially done in combination with local files
    - (node id, root name, path without root)

Remote file integrity check:
- For all remote files, check if their path still exists locally
- Possibly also check that the hash matches?

Connections:
- Open QUIC connection
- Open bidirectional stream
- Client sends Identify
- Server sends Identify
- Server waits for user to accept/deny connection
- Server sends Accepted
- Server sends Index
- Client can open more bidirectional streams for downloading

## Theming

- Rounded rectangles
- Pastel background cards
- Lucide vs Material Symbols?

## TODO
- Can we use Cargo features for desktop/mobile to not ship unused stuff to mobile?
    - Bindings are generated based on one file, so the interface should be the same... maybe just stubs if the desktop feature is disabled?
- Or can we easily build two separate crates for desktop/mobile? Should desktop just be its own codebase?

## Style guide

For consistency:
- Show file sizes in MB, not MiB
- Show transfer speeds in MB/s
- Show file sizes with one decimal point
- Show percents with no decimal points
