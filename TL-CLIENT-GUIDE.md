# Timelapse Client Script - Complete Guide

## Quick Start

Drop `tl-client.sh` into any repository and run:

```bash
./tl-client.sh setup    # Initialize timelapse
./tl-client.sh save     # Create checkpoint
./tl-client.sh log      # View history
```

## For AI Agents

### Core Workflow

```bash
# 1. Setup (once per repository)
./tl-client.sh setup

# 2. Make changes
<edit files>

# 3. Save checkpoint
./tl-client.sh save

# 4. Continue working
<edit more files>

# 5. If something breaks - restore
./tl-client.sh stop     # Stop daemon to read log
./tl-client.sh log      # Find checkpoint
./tl-client.sh restore <checkpoint-id>
```

### When to Use Each Command

#### `setup` - First Time in Repository
**Use when:** Starting work in a new repository
**What it does:** Initializes `.tl` directory and starts background daemon
**Example:**
```bash
cd /path/to/new/project
./tl-client.sh setup
```

#### `save` - Create Checkpoint Now
**Use when:**
- Completed a working feature
- Before attempting risky refactoring
- After fixing a bug
- Any time you want a guaranteed restore point

**What it does:** Forces immediate checkpoint creation
**Example:**
```bash
# After implementing feature
git add .
git commit -m "Add auth"
./tl-client.sh save  # Extra safety net
```

#### `log` - View History
**Use when:** Need to find checkpoint to restore
**What it does:** Shows checkpoint timeline
**Note:** Daemon must be stopped (database lock)
**Example:**
```bash
./tl-client.sh stop
./tl-client.sh log
# Find checkpoint ID
./tl-client.sh start
```

#### `restore` - Undo Changes
**Use when:**
- Code broke and you want to go back
- Exploring alternative implementation
- Accidentally deleted files

**What it does:** Overwrites working directory with checkpoint state
**Warning:** Current changes will be lost!
**Example:**
```bash
./tl-client.sh stop
./tl-client.sh log  # Find checkpoint
./tl-client.sh restore 01HXKJ7NVQW3Y2YMZK5VFZX3G8
./tl-client.sh start
```

#### `diff` - See What Changed
**Use when:** Want to understand differences between checkpoints
**What it does:** Shows file diffs
**Example:**
```bash
./tl-client.sh stop
./tl-client.sh diff <old-id> <new-id>
./tl-client.sh start
```

#### `status` - Check System Health
**Use when:** Debugging or checking daemon state
**What it does:** Shows if daemon is running
**Example:**
```bash
./tl-client.sh status
```

#### `pin` - Name Important Checkpoints
**Use when:** Want to mark milestones
**What it does:** Assigns memorable name to checkpoint
**Example:**
```bash
./tl-client.sh stop
./tl-client.sh log  # Find checkpoint
./tl-client.sh pin 01HXKJ7NVQW3Y2YMZK5VFZX3G8 working-auth
./tl-client.sh start
```

## Installation

### Option 1: Copy Script to Repository
```bash
cp tl-client.sh /path/to/your/repo/
cd /path/to/your/repo
./tl-client.sh setup
```

### Option 2: Add TL to PATH
```bash
cd /path/to/timelapse
./add-to-path.sh
source ~/.zshrc  # or ~/.bashrc
```

Then the script will automatically find `tl` binary.

## Configuration

Set `TL_BINARY` environment variable if `tl` is not in PATH:

```bash
export TL_BINARY=/path/to/tl
./tl-client.sh setup
```

Or in the script:
```bash
# Edit tl-client.sh
TL_BINARY="/path/to/timelapse/target/debug/tl"
```

## Known Limitations

### Database Lock During Daemon Operation

When the daemon is running, some commands (like `log`, `diff`) cannot access the database due to lock contention.

**Workaround:**
```bash
./tl-client.sh stop   # Stop daemon
./tl-client.sh log    # Run command
./tl-client.sh start  # Restart daemon
```

This is a current limitation of the SQLite database implementation.

## Complete Example Workflow

```bash
# Start new feature
cd my-project
./tl-client.sh setup

# Work on feature
echo "function newFeature() {}" >> app.js
./tl-client.sh save  # Checkpoint 1

# Continue development
echo "function helper() {}" >> app.js
./tl-client.sh save  # Checkpoint 2

# Oops, broke something!
./tl-client.sh stop
./tl-client.sh log

# Output:
# 01HXKJ8NVQ 2 seconds ago  [FsBatch] 1 modified
#   M app.js
# 01HXKJ7NVQ 10 seconds ago [FsBatch] 1 modified
#   M app.js

# Restore to working version
./tl-client.sh restore 01HXKJ7NVQ
./tl-client.sh start

# Back to working state!
```

## Tips for AI Agents

1. **Always `setup` first** in new repositories
2. **`save` liberally** - it's cheap and fast
3. **`save` before risky changes** - easy to undo
4. **Stop daemon for `log`** - database lock issue
5. **Check `log` before `restore`** - pick right checkpoint
6. **Use `pin`** for important milestones

## Performance

- **Checkpoint creation:** < 1 second
- **Restore:** < 100ms for most projects
- **Storage:** ~1.2x Git size (with deduplication)
- **Daemon overhead:** Minimal (~10MB RAM)

## Troubleshooting

### "TL binary not found"
```bash
export TL_BINARY=/path/to/tl
# or
./add-to-path.sh
```

### "Daemon is not running"
```bash
./tl-client.sh start
```

### "Could not acquire lock"
```bash
./tl-client.sh stop
# Run your command
./tl-client.sh start
```

### Check daemon logs
```bash
cat .tl/logs/daemon.log
```

## Integration with Git

Timelapse works alongside Git:

```bash
# Work on feature
<edit files>
./tl-client.sh save      # Timelapse checkpoint

# Git commit when ready
git add .
git commit -m "Add feature"
git push

# Both Git and Timelapse track changes
# Timelapse: fine-grained automatic checkpoints
# Git: coarse-grained manual commits
```

## Advanced Usage

### Custom Checkpoint Interval
Edit `.tl/config` (if supported) or modify daemon source.

### Exclude Directories
Add to `.gitignore` - timelapse respects it.

### Storage Management
```bash
tl gc  # Run garbage collection
tl info # View storage stats
```

## Script Aliases

The script supports command aliases:

- `save` = `flush` = `checkpoint`
- `log` = `history`
- `restore` = `revert`

## Environment Variables

- `TL_BINARY` - Path to TL binary (default: `tl`)

## Exit Codes

- `0` - Success
- `1` - Error (details in output)

## Support

For issues or questions:
- Check `./tl-client.sh help`
- Read `BENCHMARKS.md` for performance metrics
- View source code comments for implementation details

---

**Remember:** Timelapse is your safety net. Use it liberally, restore confidently!
