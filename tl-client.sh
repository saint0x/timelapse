#!/usr/bin/env bash
# ==============================================================================
# Timelapse Client Wrapper - Full Featured
# ==============================================================================
#
# TL replaces Git for version control. Automatic checkpoints + JJ integration.
#
# QUICK START:
#   ./tl-client.sh setup     # Initialize
#   ./tl-client.sh save      # Checkpoint
#   ./tl-client.sh push      # Push to remote
#   ./tl-client.sh pull      # Pull from remote
#
# CHECKPOINT COMMANDS:
#   setup, save, log, status, restore, diff, info, pin, unpin, gc
#
# REMOTE COMMANDS (via JJ):
#   push, pull, publish
#
# WORKSPACE COMMANDS:
#   worktree list|add|remove|switch
#
# ==============================================================================

set -e

TL_BINARY="${TL_BINARY:-tl}"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'

print_header() { echo -e "${BLUE}━━━ $1 ━━━${NC}"; }
print_success() { echo -e "${GREEN}✓${NC} $1"; }
print_error() { echo -e "${RED}✗${NC} $1" >&2; }
print_warning() { echo -e "${YELLOW}⚠${NC} $1"; }
print_info() { echo -e "${BLUE}ℹ${NC} $1"; }

check_tl() {
    command -v "$TL_BINARY" &>/dev/null || { print_error "TL binary not found. Set TL_BINARY env var."; exit 1; }
}

check_init() {
    [ -d ".tl" ] || { print_error "Not initialized. Run: $0 setup"; exit 1; }
}

check_daemon() {
    [ -S ".tl/state/daemon.sock" ] || { print_error "Daemon not running. Run: $0 setup"; exit 1; }
}

# ==============================================================================
# CHECKPOINT COMMANDS
# ==============================================================================

cmd_setup() {
    # Initialize TL and start daemon. Run once per repo.
    print_header "Setup"
    if [ -d ".tl" ]; then
        print_warning "Already initialized"
        [ -S ".tl/state/daemon.sock" ] || { "$TL_BINARY" start; print_success "Daemon started"; }
    else
        "$TL_BINARY" init && "$TL_BINARY" start
        print_success "Initialized and daemon started"
    fi
}

cmd_save() {
    # Force immediate checkpoint. Use after completing work.
    check_daemon
    print_info "Creating checkpoint..."
    "$TL_BINARY" flush
    print_success "Checkpoint created"
}

cmd_log() {
    # Show checkpoint history. Usage: log [limit]
    check_init
    "$TL_BINARY" log --limit "${1:-20}"
}

cmd_status() {
    # Show daemon and checkpoint status
    check_init
    "$TL_BINARY" status
}

cmd_restore() {
    # Restore to checkpoint. Usage: restore <id>
    # WARNING: Overwrites working directory!
    check_init
    [ -z "$1" ] && { print_error "Usage: $0 restore <checkpoint-id>"; exit 1; }
    print_warning "Restoring to: $1 (current changes will be lost)"
    "$TL_BINARY" restore "$1"
}

cmd_diff() {
    # Show diff between checkpoints. Usage: diff <id-a> <id-b>
    check_init
    [ -z "$1" ] || [ -z "$2" ] && { print_error "Usage: $0 diff <id-a> <id-b>"; exit 1; }
    "$TL_BINARY" diff "$1" "$2"
}

cmd_info() {
    # Show detailed repo info (storage stats, checkpoint count)
    check_init
    "$TL_BINARY" info
}

cmd_pin() {
    # Name a checkpoint. Usage: pin <id> <name>
    [ -z "$1" ] || [ -z "$2" ] && { print_error "Usage: $0 pin <id> <name>"; exit 1; }
    "$TL_BINARY" pin "$1" "$2"
    print_success "Pinned as: $2"
}

cmd_unpin() {
    # Remove a pin. Usage: unpin <name>
    [ -z "$1" ] && { print_error "Usage: $0 unpin <name>"; exit 1; }
    "$TL_BINARY" unpin "$1"
    print_success "Unpinned: $1"
}

cmd_gc() {
    # Run garbage collection to reclaim space
    check_init
    print_info "Running garbage collection..."
    "$TL_BINARY" gc
    print_success "GC complete"
}

cmd_start() {
    # Start the daemon
    check_init
    [ -S ".tl/state/daemon.sock" ] && { print_warning "Daemon already running"; exit 0; }
    "$TL_BINARY" start
    print_success "Daemon started"
}

cmd_stop() {
    # Stop the daemon
    [ -S ".tl/state/daemon.sock" ] || { print_warning "Daemon not running"; exit 0; }
    "$TL_BINARY" stop
    print_success "Daemon stopped"
}

# ==============================================================================
# REMOTE COMMANDS (via JJ integration)
# ==============================================================================

cmd_push() {
    # Push to Git remote via JJ
    # Usage: push [-b bookmark] [--all] [--force]
    check_init
    print_info "Pushing to remote..."
    "$TL_BINARY" push "$@"
    print_success "Pushed"
}

cmd_pull() {
    # Pull from Git remote via JJ
    # Usage: pull [--fetch-only] [--no-pin]
    check_init
    print_info "Pulling from remote..."
    "$TL_BINARY" pull "$@"
    print_success "Pulled"
}

cmd_publish() {
    # Publish checkpoint(s) to JJ for pushing
    # Usage: publish <checkpoint> [-b bookmark] [--compact] [--no-pin]
    # Example: publish HEAD or publish HEAD~10..HEAD
    check_init
    [ -z "$1" ] && { print_error "Usage: $0 publish <checkpoint> [-b bookmark]"; exit 1; }
    "$TL_BINARY" publish "$@"
    print_success "Published"
}

# ==============================================================================
# WORKSPACE COMMANDS
# ==============================================================================

cmd_worktree() {
    # Manage JJ workspaces
    # Usage: worktree list|add|remove|switch [args]
    check_init
    [ -z "$1" ] && { print_error "Usage: $0 worktree list|add|remove|switch"; exit 1; }
    "$TL_BINARY" worktree "$@"
}

# ==============================================================================
# HELP
# ==============================================================================

cmd_help() {
    cat << 'EOF'
TL Client - Timelapse Version Control (replaces Git)

CHECKPOINT COMMANDS:
  setup              Initialize TL in repo, start daemon
  save               Create checkpoint immediately
  log [n]            Show last n checkpoints (default: 20)
  status             Show daemon/checkpoint status
  restore <id>       Restore to checkpoint (overwrites working dir!)
  diff <a> <b>       Show diff between two checkpoints
  info               Show repo stats
  pin <id> <name>    Name a checkpoint
  unpin <name>       Remove a pin
  gc                 Garbage collection
  start/stop         Control daemon

REMOTE COMMANDS (via JJ):
  push               Push to Git remote
                     Options: -b <bookmark>, --all, --force
  pull               Pull from Git remote
                     Options: --fetch-only, --no-pin
  publish <id>       Publish checkpoint to JJ for pushing
                     Options: -b <bookmark>, --compact

WORKSPACE COMMANDS:
  worktree list      List workspaces
  worktree add       Add workspace
  worktree remove    Remove workspace
  worktree switch    Switch workspace

WORKFLOW:
  1. ./tl-client.sh setup           # Once per repo
  2. <make changes>
  3. ./tl-client.sh save            # Checkpoint
  4. ./tl-client.sh publish HEAD    # Prepare for push
  5. ./tl-client.sh push            # Push to remote

RESTORE WORKFLOW:
  ./tl-client.sh log                # Find checkpoint
  ./tl-client.sh restore <id>       # Restore

Checkpoints auto-created every 5 seconds. Manual 'save' for immediate checkpoint.
EOF
}

# ==============================================================================
# MAIN
# ==============================================================================

main() {
    check_tl
    local cmd="${1:-help}"; shift 2>/dev/null || true

    case "$cmd" in
        setup)                      cmd_setup "$@" ;;
        save|flush|checkpoint)      cmd_save "$@" ;;
        log|history)                cmd_log "$@" ;;
        status)                     cmd_status "$@" ;;
        restore|revert)             cmd_restore "$@" ;;
        diff)                       cmd_diff "$@" ;;
        info)                       cmd_info "$@" ;;
        pin)                        cmd_pin "$@" ;;
        unpin)                      cmd_unpin "$@" ;;
        gc)                         cmd_gc "$@" ;;
        start)                      cmd_start "$@" ;;
        stop)                       cmd_stop "$@" ;;
        push)                       cmd_push "$@" ;;
        pull)                       cmd_pull "$@" ;;
        publish)                    cmd_publish "$@" ;;
        worktree|wt)                cmd_worktree "$@" ;;
        help|--help|-h)             cmd_help ;;
        *)                          print_error "Unknown: $cmd. Run '$0 help'"; exit 1 ;;
    esac
}

main "$@"
