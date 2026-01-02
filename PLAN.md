Cool — let’s nail it down cleanly, but I’m going to be strict about one thing:

“Store diffs” is a UX description, not a storage primitive.
If you literally store text diffs as your source of truth, you will get bitten (binary files, rename dances, partial writes, newline normalization, mode bits, conflict states, etc.). The correct way is:
	•	Store snapshots as content-addressed file versions (blobs) + trees
	•	Compute diffs on demand (or cache them for UI)

You still get your value prop (“lossless diff history”), but with correctness.

That said, your “sliding window” idea is valid — just apply it to checkpoint nodes / file versions, not “diff bytes only.”

⸻

The crisp product definition

A repo-local, always-on checkpoint stream that captures working-copy state changes losslessly and lets you restore to any checkpoint instantly.
	•	snap init: enroll repo
	•	Background watcher: records checkpoints
	•	snap log: shows a timeline (diff summaries)
	•	snap restore <checkpoint>: rewinds working tree
	•	Git compat: via JJ bridge (jj git push/fetch)

Repo-facing is right. Tool integration is optional sugar, not required.

⸻

What exactly is a “checkpoint”?

A checkpoint is a node:

Checkpoint {
  id: Hash,
  parent: Hash,
  root_tree: Hash,     // snapshot of entire repo tree at that moment
  touched_paths: [Path],
  meta: { ts, reason, ... }
}

Where root_tree references a tree:

Tree: path -> Entry { type, mode, blob_hash }
Blob: content-addressed bytes (or chunk list)

This gives you losslessness, rename correctness (via tree path mapping), and binary safety.

Diff = Tree(A) vs Tree(B).
Not the other way around.

⸻

The watcher pipeline (the real “sliding window”)

You were describing “hello.txt changed 6→7→… and we only care about last state.” That maps to:

1) Watch events are dirty hints
	•	inotify/FSEvents tells you “these paths probably changed”
	•	you debounce + coalesce

2) You sample the current bytes

For each dirty path:
	•	read file bytes (or stream)
	•	hash them (BLAKE3 is perfect here)
	•	if hash unchanged: ignore
	•	if hash changed: write blob if new

3) You create a checkpoint node

Checkpoint includes:
	•	parent pointer
	•	new root_tree hash (built by applying changed entries to previous tree)
	•	touched_paths list

This is the key: you do not rescan the whole repo. You update the previous tree with only touched paths.

4) Sliding window / retention

Retention is applied at the checkpoint level:
	•	keep last N checkpoints (or last T minutes) dense
	•	periodically “pin” milestones (last-good, pre-push, manual label)
	•	GC drops old checkpoints unless pinned

Per-file “keep last 10 changes” is just a view built from checkpoints that touched that file. You don’t need a separate per-file store unless you want fast queries.

⸻

How JJ fits cleanly (and where it doesn’t)

Use JJ for:
	•	presenting history / undo semantics to users (optional)
	•	Git interoperability (push/fetch)
	•	“bookmarks” as named pins

Do not rely on JJ to detect working-copy micro-changes

JJ/Git aren’t meant to be “every keystroke journaling.” You’ll either miss intermediate states or create insane operational overhead if you turn every tiny change into JJ changes.

So the clean split is:

Your engine = high-frequency checkpoint stream
JJ = low-frequency publication layer

Concrete mapping:
	•	Many checkpoints exist locally (your store)
	•	When user wants to “commit/push,” you materialize a JJ change/bookmark from a chosen checkpoint (or compact a range)
	•	snap push uses jj git push -b <bookmark>

This keeps JJ doing what it’s good at, and you doing what you’re good at.

⸻

The one place your “diff-only” framing is accurate

You can store only “what changed” between checkpoints, but again: that means:
	•	storing only new blobs for changed files, and
	•	storing a small tree delta (path→new entry)

That’s the true “diff store,” and it’s safe.

Avoid “patch diffs as truth.”

⸻

Correctness: why “sliding window” isn’t the hard part

The tricky correctness isn’t “file ended at 6 vs 9.” That’s easy.

The tricky parts are:
	•	atomic save patterns (temp file + rename)
	•	partial writes (you read mid-write unless you debounce)
	•	event drops / reorder
	•	symlinks and executable bit changes
	•	deletes and rapid recreate

Your solution: debounced sampling + periodic reconciliation
	•	Debounce 200–500ms by default
	•	If watcher overflows: mark “dirty_unknown” and do a quick targeted scan (not full) using mtime/size heuristics
	•	Keep an append-only journal so crash recovery is deterministic

This isn’t mystical, but it is where quality lives.

⸻

Clean scope for v1 (so this ships)

v1 goals
	•	repo-local store
	•	watcher → checkpoint creation
	•	log + restore
	•	retention + pinned checkpoints
	•	JJ-backed Git push/pull from selected checkpoint

v1 non-goals
	•	AST / semantic diffs
	•	multi-machine sync of checkpoints
	•	fancy replay/provenance
	•	cross-repo dedupe

You can layer AST later for “what changed” summaries. The store must stay byte-truth.

⸻

Concrete CLI (minimal but crisp)
	•	snap init
	•	snap status (daemon health, last checkpoint, store size)
	•	snap log (timeline)
	•	snap diff <a> <b> (tree diff)
	•	snap restore <id|label>
	•	snap pin <id> <name> / snap unpin
	•	snap gc (retention)
	•	snap publish (turn checkpoint range into a JJ change/bookmark)
	•	snap push / snap pull (via JJ)

⸻

If we “really nail it down,” here’s the exact design choice you should commit to now

Commit to “tree+blob store” as your truth, not patches.
Everything else (sliding windows, retention, UI diffs, JJ interop) becomes straightforward once that’s fixed.

If you want, next message I’ll draft:
	•	the on-disk format (.snap/ layout),
	•	the checkpoint journal schema,
	•	and the algorithm for “update tree hash from touched paths” without rescanning the repo (the performance linchpin).


	====================    PART  2     =======================================



	Alright. Here’s the MVP-level, low-level design you should commit to. This is the “nail it down” version: tree+blob store is truth, patches are derived. I’m going to be precise about formats, invariants, and the incremental tree update algorithm.

⸻

0) Core commitment (the design choice)

Truth is:
	•	Blobs: content-addressed file bytes (or chunk lists later)
	•	Trees: full repo state mapping path -> entry
	•	Checkpoints: DAG nodes that point at a tree hash

Not truth:
	•	textual diffs / patches
	•	AST
	•	JJ commits (these are publication artifacts)

Everything else becomes a view or export.

⸻

1) On-disk format: .snap/ layout (MVP)

Put this at repo root: .snap/ (like .git/).

.snap/
  config.toml
  HEAD                      # current checkpoint id (optional)
  locks/
    daemon.lock
    gc.lock
  journal/
    ops.log                 # append-only checkpoint log
    ops.log.idx             # optional sparse index
  objects/
    blobs/
      ab/cdef...            # blob object files by hash prefix
    trees/
      12/34ab...            # tree objects by hash prefix
  refs/
    pins/
      last-good             # file contains checkpoint id
      pre-push
      manual/foo
    heads/
      workspace             # current working lineage pointer
  state/
    pathmap.bin             # current tree index for fast updates
    watcher.state           # cursors/checkpoints for fs watcher
    metrics.json
  tmp/
    ingest/
    gc/

Why this layout works
	•	objects/ is immutable content (CAS).
	•	journal/ is your authoritative timeline.
	•	refs/ is stable names → checkpoint ids (pins).
	•	state/ is performance/cache state you can rebuild.
	•	tmp/ is for atomic writes.

⸻

2) Hashing + Object encoding

Hash function

Use BLAKE3 (fast, secure enough for integrity, Rust libs great). Hash is 32 bytes.

Represent as hex string in human outputs; on disk you can store raw bytes.

⸻

Blob object: file bytes (truth)

Blob object stores exact bytes + metadata needed for correct restore.

Blob header (binary or CBOR)

You want minimal overhead. Use a small binary header:

struct BlobHeaderV1 {
  magic: [u8; 4] = "SNB1",
  flags: u8,                 // compression, type
  orig_len: u64,
  stored_len: u64,
  // future: chunking, etc
}
payload: [u8; stored_len]

	•	flags:
	•	bit0: compressed (zstd)
	•	bit1: is_symlink_target (optional; I prefer storing symlink as tree entry type, not blob)
	•	others reserved

Compression policy (MVP)
	•	compress blobs > 4KB with zstd level ~1–3
	•	keep tiny files uncompressed

Blob id = blake3(headerless_plain_bytes) OR blake3(normalized representation).

Pick one and stick with it. I recommend:
	•	blob hash = hash of raw file bytes (before compression)
	•	compression is a storage detail

This makes dedupe stable.

Blob path:
objects/blobs/<hh>/<rest> where <hh> are first 2 hex chars.

⸻

Tree object: path map (truth)

A tree represents the entire repo state at a checkpoint.

Entry types
	•	file → blob hash + mode
	•	dir → implicit via path prefix (you can store directory nodes explicitly or not; MVP can be implicit)
	•	symlink → store link target bytes + mode (or store as blob; either is fine)
	•	submodule → treat as special entry (optional for MVP; you can ignore initially)

Canonical tree encoding (must be deterministic)

You need a stable serialization so the same tree always hashes identically.

I recommend sorted entries (lexicographic path bytes) and a compact binary format:

TreeV1:
  magic "SNT1"
  entry_count u32
  repeated entry:
    path_len u16
    path_bytes [u8; path_len]   // UTF-8, store as bytes
    kind u8                     // 0=file,1=symlink,2=submodule (optional)
    mode u32                    // unix perms bits + executable
    blob_hash [u8; 32]          // for file; for symlink store hash of link target bytes

Tree id = blake3(tree_bytes).

Tree path:
objects/trees/<hh>/<rest>.

Important: paths must be normalized:
	•	store relative paths with /
	•	no ./
	•	reject ..
	•	enforce canonical case handling based on FS (hard, but MVP: preserve path as seen, and treat OS accordingly)

⸻

3) Checkpoint journal schema (append-only)

This is your source of truth for history. Do not store checkpoints as mutable DB rows.

Journal record structure

Use newline-delimited CBOR or a compact binary record format.

Text JSON is easy but bigger/slower; NDJSON is fine for MVP. If you want to stay serious: CBOR.

I’ll describe it as a logical schema:

CheckpointRecordV1 {
  seq: u64,                     // monotonic journal offset
  id: [u8; 32],                 // checkpoint hash
  parent: [u8; 32] | null,
  root_tree: [u8; 32],
  ts_unix_ms: u64,
  reason: enum { fs_batch, manual, restore, publish, gc_compact },
  touched_paths: Vec<Path>      // optional but great UX/perf
  stats: {
    files_changed: u32,
    bytes_added: u64,
    bytes_removed: u64,
  }
  meta: {
    hostname?, user?, pid?,
    // future: tool ids, test status
  }
}

Checkpoint ID derivation

Make checkpoint id content-addressed too:

checkpoint_id = blake3(parent_id || root_tree || ts || reason || touched_paths_hash)

You can exclude timestamp if you want purely structural IDs, but timestamp in the hash is fine—IDs remain stable after creation.

Index

ops.log.idx can be a sparse index:
	•	every N records, store byte offset → seq
	•	allows fast seek by seq/id

⸻

4) State cache: pathmap.bin (performance linchpin)

To avoid rescanning + rebuilding trees, you maintain an in-memory and persisted map:

PathMap: path -> Entry { kind, mode, blob_hash }

Persist it as:
	•	a compact binary sorted list OR
	•	a lightweight embedded KV store

For MVP, do a sorted binary snapshot (rebuildable) + incremental redo from journal:

state/pathmap.bin
  magic "SNP1"
  root_tree [u8; 32]      // tree this map corresponds to
  entry_count u32
  entries sorted by path

On startup:
	•	load pathmap.bin
	•	verify root_tree matches HEAD checkpoint tree
	•	if mismatch, rebuild by replaying journal from last known good

Do not use sqlite unless you love pain; a flat binary + append-only journal is simpler and more robust.

⸻

5) Incremental “update tree hash from touched paths” algorithm

This is the part you asked for. Here’s the actual algorithm you implement.

Inputs
	•	base_map: current PathMap for parent checkpoint (path→entry)
	•	dirty_set: paths reported by watcher (plus maybe parent dirs)
	•	repo root path on disk

Output
	•	new PathMap
	•	new root_tree_hash
	•	checkpoint record appended to journal

⸻

Step A: coalesce and normalize dirty paths

Watcher gives noisy paths. Normalize:
	1.	Convert to repo-relative path
	2.	Drop anything under .snap/ and .git/
	3.	Apply ignore rules (configurable; start with .gitignore-like support later)
	4.	Deduplicate
	5.	If a directory is dirty (some watchers report dirs), expand by scanning that directory shallowly (list children) into file paths

Result: candidate_paths.

⸻

Step B: reconcile each candidate path

For each p in candidate_paths:

Case 1: file exists
	•	stat(p) → size, mtime, mode, type
	•	If type == regular file:
	•	compute quick fingerprint fp = (size, mtime_ns, inode?)
	•	if fp unchanged from cached entry AND you trust mtime: skip hashing (fast path)
	•	else read file bytes and hash h = blake3(bytes)
	•	if cached entry has same blob hash: update mode if needed; else:
	•	store blob if missing in CAS
	•	update entry in map: {kind=file, mode, blob_hash=h}
	•	If type == symlink:
	•	readlink target bytes
	•	hash target bytes as “symlink blob hash”
	•	store (optional) + update entry kind=symlink
	•	If type == directory:
	•	do nothing directly; dirs are implicit (unless you store explicit dirs)

Case 2: path does not exist
	•	Remove p from map if present (deletion)

Case 3: rename detection (MVP optional)

Rename detection is not required for correctness; delete+add is fine.
But for nicer diffs, you can detect renames by:
	•	mapping blob_hash → previous path(s)
	•	if a blob appears at new path and disappeared at old path in same batch, tag as rename in UI

Do not bake renames into truth. It’s a presentation layer.

⸻

Step C: produce new root tree without full scan

Now you have new_map = base_map plus modifications.

To compute the new tree hash, you must serialize the full path list deterministically. You have two options:

Option 1 (MVP-simple, still fast): serialize all entries every checkpoint
	•	entries = new_map.iter_sorted_by_path()
	•	encode into TreeV1 bytes
	•	hash to get root_tree_hash
	•	store tree object if missing

This is O(number_of_tracked_paths) per checkpoint.
For very big monorepos + frequent checkpoints, this can be heavy.

Option 2 (better, still MVP-feasible): Merkleize by directory

Represent tree as hierarchical directory nodes:
	•	A directory tree hash depends only on its direct children hashes
	•	Updating a file only recomputes hashes along its directory path to root

This yields O(changed_paths * log(depth)) hashing.

Directory Merkle structure

Each directory node stores sorted children:
	•	child file entry: (name, kind=file, mode, blob_hash)
	•	child dir entry: (name, kind=dir, dir_hash)
Hash directory node bytes → dir_hash

Root hash = hash of root dir node.

Update algorithm

For each changed path, update leaf entry, then recompute directory hashes up to root.

This requires a directory index in state (DirMap) to avoid rebuilding.
It’s more work but pays off.

Recommendation:
	•	MVP v0: Option 1 (flat tree) + debounce (don’t checkpoint every keystroke)
	•	MVP v1.1: upgrade to merkle dirs when you hit perf limits

Given your “always-on” ambition, plan for Option 2, but ship Option 1 first.

⸻

Step D: append journal + update HEAD
	•	Create checkpoint record
	•	Append to ops.log atomically (write → fsync → rename)
	•	Update .snap/HEAD to new checkpoint id
	•	Update state/pathmap.bin snapshot periodically (not every checkpoint; e.g. every 100 checkpoints)

⸻

6) Retention / sliding window (low-level, MVP)

You described “keep last N changes per file.” Implement retention at checkpoint level:
	•	Config:
	•	retain_dense_count = 2000
	•	retain_dense_window = 24h
	•	retain_pins = forever
	•	GC algorithm:
	1.	determine live checkpoint set:
	•	all pins
	•	last N from HEAD
	•	any checkpoint newer than T window
	2.	walk reachable trees/blobs
	3.	delete unreferenced objects

This is classic mark-and-sweep.

If you really want per-file retention later, it’s a policy layer that pins checkpoints that touch that file. Don’t do it first.

⸻

7) Correctness policies you must commit to (MVP)

These are the “don’t get sued by reality” rules.

7.1 Debounce + stable read

Never hash a file while it’s mid-write.
	•	debounce window
	•	optional “double-stat”: stat → read → stat again; if changed, requeue

7.2 Watcher overflow recovery

If watcher drops events:
	•	mark repo “dirty_unknown”
	•	do a targeted scan:
	•	walk filesystem looking only at mtimes since last checkpoint (still can be big, but rare)
	•	reconcile map

7.3 Atomic writes for store/journal
	•	write temp file in .snap/tmp
	•	fsync temp
	•	rename into place
	•	fsync parent directory (yes, do it)

7.4 Ignore .snap/ always

No recursion. Ever.

⸻

8) Minimal daemon model (MVP)

Repo-facing “init spawns background” needs a clean lifecycle.
	•	snapd per-user supervisor (optional later)
	•	MVP: per-repo daemon launched on demand:
	•	snap init can install a launch entry (macOS LaunchAgent / systemd user / etc.) OR just instruct snap start
	•	Daemon stores PID + lock in .snap/locks/daemon.lock
	•	CLI commands talk to daemon over:
	•	unix socket: .snap/state/daemon.sock
	•	fallback: if daemon not running, run in-process snapshot for that command

You must support “no daemon, still works” for trust/adoption.

⸻

9) JJ bridge (MVP-level mechanics)

You keep this simple:
	•	snap publish <checkpoint|range>:
	•	materialize working tree to that checkpoint (temporary)
	•	call jj commit / jj describe creating a JJ change
	•	set bookmark snap/<name>
	•	snap push:
	•	jj git push -b snap/<name>
	•	snap pull:
	•	jj git fetch + jj rebase per your policy
	•	optional: ingest pulled head as a pinned baseline checkpoint

Don’t try to mirror every checkpoint into JJ. JJ stays coarse.

⸻

10) The MVP “done means done” checklist

If you ship MVP, it must do these reliably:
	•	create checkpoints continuously during editing (debounced)
	•	restore to any checkpoint and get identical bytes back
	•	survive daemon crash + reboot without losing history
	•	handle deletes, renames (as delete+add), symlinks, exec bit
	•	GC store safely with pins
	•	export a checkpoint to JJ + push to Git

That’s it. Everything else is garnish.

⸻

If you want the next layer, I can immediately follow up with:
	•	a concrete Rust module breakdown (watcher, store, tree, journal, daemon, cli, jj_bridge)
	•	and the exact file formats as Rust structs (so you can start coding without bikeshedding).