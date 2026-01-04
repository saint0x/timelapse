# JJ Integration Guide

## Overview

Timelapse provides seamless integration with [Jujutsu (JJ)](https://github.com/martinvonz/jj), a next-generation version control system. This integration enables you to:

- **Maintain dual workflows**: Keep Timelapse's automatic checkpoints while selectively publishing to JJ
- **Bridge to Git**: Use JJ as an intermediary to publish Timelapse checkpoints to Git remotes
- **Preserve history**: Map checkpoints to JJ commits bidirectionally for full traceability

### Philosophy

**Timelapse checkpoints** capture every significant change automatically - think of them as "save points" in a video game.

**JJ commits** represent curated, publishable milestones - the versions you want to share with your team.

This integration lets you have the best of both worlds: comprehensive local history without cluttering your team's commit graph.

---

## Installation

### Prerequisites

1. Install Jujutsu:
   ```bash
   cargo install jj-cli
   ```

2. Verify installation:
   ```bash
   jj --version
   ```

3. Ensure you have Timelapse initialized:
   ```bash
   tl init
   ```

---

## Setup

### Initialize JJ Workspace

When you run `tl init`, it automatically detects and sets up JJ if you already have a `.jj/` directory. Otherwise, you can initialize JJ manually:

```bash
# Option 1: JJ with Git backend (recommended for most users)
jj git init

# Option 2: JJ-only workspace (no Git integration)
jj init
```

### Verify Integration

Check that both systems are properly initialized:

```bash
tl info
```

You should see:
```
JJ Integration: ✓ Enabled
JJ Workspace: /path/to/repo/.jj
```

---

## Basic Workflow

### 1. Work Normally with Timelapse

Timelapse automatically creates checkpoints as you work:

```bash
# Make changes to your files
echo "new feature" > feature.txt

# Checkpoints are created automatically
tl log
# Output:
# 01HN8XYZ  2 seconds ago    FS batch    1 file
```

### 2. Publish Checkpoints to JJ

When you're ready to create a commit, publish one or more checkpoints:

```bash
# Publish the latest checkpoint
tl publish HEAD -b feature-name

# Publish a range of checkpoints
tl publish HEAD~5..HEAD -b feature-name

# Publish using short syntax (HEAD~5 means HEAD~5..HEAD)
tl publish HEAD~5 -b feature-name
```

This creates:
- A JJ commit with your changes
- A bookmark `snap/feature-name` pointing to it
- A bidirectional mapping between the checkpoint and JJ commit

### 3. Push to Git Remote

Push your JJ commits to a Git remote:

```bash
# Push a specific bookmark
tl push -b feature-name

# Push all bookmarks
tl push --all
```

### 4. Pull from Git Remote

Fetch changes and import them as checkpoints:

```bash
# Fetch and import JJ HEAD
tl pull

# Fetch only (no import)
tl pull --fetch-only
```

---

## Advanced Usage

### Publishing Modes

#### Compact Mode (Default for Ranges)

Squash multiple checkpoints into a single JJ commit:

```bash
tl publish HEAD~10 --compact -b feature
```

**Use when:**
- You want a clean commit history
- The intermediate checkpoints aren't meaningful to your team
- You're publishing a completed feature

#### Expand Mode

Create one JJ commit per checkpoint:

```bash
tl publish HEAD~10 --no-compact -b feature
```

**Use when:**
- You want to preserve the checkpoint history
- Each checkpoint represents a logical step
- You're publishing work-in-progress for code review

### Custom Commit Messages

Use a template to customize commit messages:

```bash
tl publish HEAD -b feature -m "feat: Add user authentication

- Implement JWT token generation
- Add login/logout endpoints
- Update user model with password hashing"
```

The commit message will include:
- Your custom message
- Checkpoint metadata (timestamp, file count)
- File list (up to 10 files by default)

### Auto-Pinning

Published checkpoints are automatically pinned with the "published" label:

```bash
# Publish without pinning
tl publish HEAD -b feature --no-pin

# View pinned checkpoints
tl info
# Output:
# Pins:
#   published: 01HN8XYZ (2 minutes ago)
```

---

## Best Practices

### When to Publish

✅ **DO publish when:**
- You've completed a feature or bug fix
- You want to share work for code review
- You've reached a stable, working state
- You're about to start a different task

❌ **DON'T publish:**
- After every checkpoint (defeats the purpose)
- Work that doesn't compile or pass tests
- Experimental changes you might revert

### Bookmark Naming

Timelapse automatically prefixes bookmarks with `snap/` to distinguish them from manual JJ bookmarks:

```bash
tl publish HEAD -b feature-auth
# Creates bookmark: snap/feature-auth
```

**Recommended naming:**
- `feature-name`: For new features
- `fix-description`: For bug fixes
- `refactor-component`: For refactoring work
- `wip-experiment`: For work-in-progress

### Checkpoint Selection

```bash
# Last checkpoint
tl publish HEAD

# Last 5 checkpoints
tl publish HEAD~5

# Specific range
tl publish <checkpoint-id-1>..<checkpoint-id-2>

# By pin name
tl publish milestone-pin

# By relative offset
tl publish HEAD~10..HEAD~5
```

---

## Troubleshooting

### JJ Workspace Not Found

**Error:**
```
No JJ workspace found. Run 'jj git init' first.
```

**Solution:**
```bash
jj git init
```

### Authentication Failed (Push)

**Error:**
```
Error: Authentication failed
```

**Solutions:**

For GitHub:
```bash
# Use SSH keys
ssh-add ~/.ssh/id_ed25519

# Or use GitHub CLI
gh auth login
```

For GitLab:
```bash
# Use SSH keys or personal access tokens
git remote set-url origin git@gitlab.com:user/repo.git
```

### Push Rejected (Non-Fast-Forward)

**Error:**
```
Error: Push rejected by remote
The remote has changes you don't have locally.
```

**Solution:**
```bash
tl pull
jj rebase -d main
tl push -b your-bookmark
```

### Checkpoint Already Published

**Error:**
```
Warning: The following checkpoints are already published:
  01HN8XYZ → a1b2c3d4e5f6
```

**Solution:**

Option 1 - Publish unpublished checkpoints:
```bash
tl publish <newer-checkpoint-id>
```

Option 2 - Use compact mode to squash:
```bash
tl publish HEAD~10 --compact -b feature
```

### No Git Remotes Configured

**Error:**
```
Warning: No git remotes configured.
Add a remote first: git remote add origin <url>
```

**Solution:**
```bash
git remote add origin git@github.com:user/repo.git
```

---

## Mapping Database

Timelapse maintains a bidirectional mapping between checkpoints and JJ commits in `.tl/state/jj-mapping/`:

### Forward Mapping
Checkpoint ID → JJ Commit ID

Used when:
- Checking if a checkpoint is already published
- Finding the JJ commit for a checkpoint

### Reverse Mapping
JJ Commit ID → Checkpoint ID

Used when:
- Importing JJ commits during `tl pull`
- Avoiding duplicate imports

### Inspection

View mappings using the JJ crate API (future feature):
```bash
tl jj mappings
# Output:
# 01HN8XYZ → a1b2c3d4e5f6  (published 2 hours ago)
# 01HN8ABC → b2c3d4e5f6a1  (published 1 hour ago)
```

---

## Integration with Git

JJ uses Git as a backend, so your commits are automatically available in Git:

```bash
# View JJ commits in Git
git log

# View JJ bookmarks in Git
git branch -a
```

### Workflow Example

```bash
# 1. Work with Timelapse (automatic checkpoints)
echo "code" > file.txt

# 2. Publish to JJ
tl publish HEAD -b feature

# 3. Push to Git remote
tl push -b feature

# 4. On GitHub/GitLab, create PR from snap/feature branch
```

---

## Performance Considerations

### Checkpoint Import (Pull)

Importing JJ commits as checkpoints:
- **Time**: ~50-100ms per commit (depends on tree size)
- **Storage**: Deduplicated blobs (no wasted space)
- **Network**: Only fetches once, imports locally

### Checkpoint Publishing

Publishing checkpoints to JJ:
- **Time**: < 100ms per checkpoint
- **Storage**: Uses JJ's content-addressed storage
- **Conflicts**: None (JJ handles all conflicts)

---

## Future Enhancements

Planned features for JJ integration:

1. **Smart publish**: `tl publish --smart` (auto-detect unpublished work)
2. **Bidirectional sync**: `tl sync` (pull + publish + push in one command)
3. **Conflict resolution**: Interactive merge tool for pull conflicts
4. **Mapping inspection**: `tl jj mappings` command
5. **Template customization**: Custom commit message templates
6. **Incremental publish**: Only publish changed files (not full tree)

---

## FAQ

### Q: Can I use Timelapse without JJ?

**A:** Yes! JJ integration is completely optional. Timelapse works independently and only uses JJ when you explicitly run `tl publish`, `tl push`, or `tl pull`.

### Q: What happens if I delete a checkpoint that's published?

**A:** The JJ commit remains intact. The mapping is removed, but the commit is safe in JJ's history.

### Q: Can I publish the same checkpoint twice?

**A:** No, Timelapse prevents this by default. Use `--compact` mode if you need to include published checkpoints in a range.

### Q: How do I unpublish a checkpoint?

**A:** You can't unpublish, but you can:
1. Create a new checkpoint with reverted changes
2. Publish the new checkpoint
3. The old JJ commit remains in history (as with Git)

### Q: Does `tl pull` affect my working directory?

**A:** No, `tl pull` only imports JJ commits as checkpoints. Your working directory is unchanged. Use `tl restore` to actually restore files.

### Q: Can I use JJ commands directly?

**A:** Yes! Timelapse doesn't interfere with JJ. You can use `jj` commands freely. Timelapse only manages the checkpoint ↔ commit mapping.

---

## See Also

- [Jujutsu Documentation](https://martinvonz.github.io/jj/)
- [Timelapse README](../README.md)
- [Phase 5 Implementation Plan](../plan-ascending/5.md)
