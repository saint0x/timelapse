# Timelapse (tl)

**Automatic version control for AI-assisted development.** Every file change creates a checkpoint you can restore instantly.

Built on [Jujutsu](https://github.com/martinvonz/jj) for Git compatibility.

## Quick Start

```bash
# Install
./build.sh install

# Initialize in your project
cd /your/project
tl init

# That's it. Checkpoints are created automatically every 5 seconds.
```

## Core Commands

| Command | Description |
|---------|-------------|
| `tl init` | Initialize timelapse in current directory |
| `tl status` | Show daemon and checkpoint status |
| `tl log` | View checkpoint history |
| `tl restore <id>` | Restore to a checkpoint |
| `tl diff <a> <b>` | Compare two checkpoints |

## Git Integration

```bash
tl publish HEAD           # Publish checkpoint to JJ
tl push                   # Push to Git remote
tl pull                   # Pull from Git remote
```

## Checkpoint References

- **Short ID**: `01KE5RW` (first 7+ chars from `tl log`)
- **Pin name**: `tl pin <id> my-feature` then `tl restore my-feature`
- **HEAD**: Latest checkpoint

## All Commands

### Setup
```bash
tl init                   # Initialize (also starts daemon)
tl start                  # Start daemon
tl stop                   # Stop daemon
tl status                 # Show status
tl info                   # Detailed repo info
```

### Checkpoints
```bash
tl log                    # Show history (default: 20)
tl log --limit 50         # Show more
tl flush                  # Force immediate checkpoint
tl restore <id>           # Restore to checkpoint
tl restore <id> -y        # Skip confirmation
tl diff <a> <b>           # File-level diff
tl diff <a> <b> -p        # Line-level diff
```

### Pins
```bash
tl pin <id> <name>        # Name a checkpoint
tl unpin <name>           # Remove pin
```

### Git/JJ Integration
```bash
tl publish <id>           # Publish to JJ
tl publish HEAD --compact # Squash into one commit
tl push                   # Push to remote
tl push -b feature        # Push specific bookmark
tl pull                   # Pull from remote
```

### Maintenance
```bash
tl gc                     # Garbage collection
```

## How It Works

1. **Background daemon** watches for file changes
2. **Every 5 seconds**, creates a content-addressed checkpoint
3. **Checkpoints are cheap** - only changed files are stored
4. **Restore instantly** - working directory replaced in <100ms
5. **Push to Git** - publish checkpoints as commits via Jujutsu

## Performance

| Operation | Time |
|-----------|------|
| Checkpoint creation | <10ms |
| Restore | <100ms |
| Storage overhead | ~1.2x vs Git |

## Development

```bash
./build.sh check      # Fast compile check
./build.sh debug      # Debug build
./build.sh release    # Release build
./build.sh install    # Build + install to PATH

./test.sh test-quick  # Fast tests (~10s)
./test.sh test-all    # Full suite (~2min)
```

## Requirements

- macOS (FSEvents) or Linux (inotify)
- Rust 1.75+

## License

MIT or Apache-2.0
