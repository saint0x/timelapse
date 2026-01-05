#!/usr/bin/env bash
# ==============================================================================
# Timelapse Client Wrapper
# ==============================================================================
#
# Drop this script into any repository to enable instant checkpoint management.
# This wrapper provides a simplified interface to the TL (Timelapse) binary
# with built-in guidance for AI agents and developers.
#
# PURPOSE:
#   Timelapse captures automatic checkpoints of your working directory every
#   time files change. Think of it as "git commit" that happens automatically
#   in the background, letting you restore to any previous state instantly.
#
# QUICK START FOR AGENTS:
#   1. Run: ./tl-client.sh setup      # Initialize timelapse in this repo
#   2. Modify files as needed
#   3. Run: ./tl-client.sh save       # Force checkpoint creation
#   4. Run: ./tl-client.sh log        # See checkpoint history
#   5. Run: ./tl-client.sh restore    # Restore to previous checkpoint
#
# ==============================================================================

set -e  # Exit on error

# Configuration
TL_BINARY="${TL_BINARY:-tl}"  # Override with environment variable if needed
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ==============================================================================
# Helper Functions
# ==============================================================================

print_header() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1" >&2
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

check_tl_binary() {
    if ! command -v "$TL_BINARY" &> /dev/null; then
        print_error "TL binary not found at: $TL_BINARY"
        echo ""
        echo "Please install timelapse or set TL_BINARY environment variable:"
        echo "  export TL_BINARY=/path/to/tl"
        echo ""
        echo "To build from source:"
        echo "  cd /path/to/timelapse"
        echo "  cargo build --release"
        echo "  export TL_BINARY=\$PWD/target/release/tl"
        exit 1
    fi
}

is_initialized() {
    [ -d ".tl" ]
}

is_daemon_running() {
    # Check if daemon socket exists
    [ -S ".tl/state/daemon.sock" ] && return 0 || return 1
}

# ==============================================================================
# Command Functions
# ==============================================================================

cmd_setup() {
    # WHEN TO USE: First time setting up timelapse in a repository
    # WHAT IT DOES: Initializes .tl directory and starts the daemon
    # AGENT GUIDANCE: Run this once per repository before any other commands

    print_header "Setting up Timelapse"

    if is_initialized; then
        print_warning "Timelapse already initialized in this repository"
        if ! is_daemon_running; then
            print_info "Starting daemon..."
            "$TL_BINARY" start
            print_success "Daemon started"
        else
            print_success "Daemon already running"
        fi
    else
        print_info "Initializing timelapse..."
        "$TL_BINARY" init
        print_success "Timelapse initialized"

        print_info "Starting daemon..."
        "$TL_BINARY" start
        print_success "Daemon started"
    fi

    echo ""
    print_info "Timelapse is now tracking changes automatically"
    print_info "Checkpoints are created every 5 seconds when files change"
}

cmd_save() {
    # WHEN TO USE: After making significant changes you want to checkpoint immediately
    # WHAT IT DOES: Forces immediate checkpoint creation (doesn't wait for 5s interval)
    # AGENT GUIDANCE: Use after completing a logical unit of work

    print_header "Creating Checkpoint"

    if ! is_daemon_running; then
        print_error "Daemon is not running. Start it with: $0 setup"
        exit 1
    fi

    print_info "Flushing changes to create checkpoint..."
    "$TL_BINARY" flush
    print_success "Checkpoint created"
}

cmd_log() {
    # WHEN TO USE: To see history of checkpoints and find one to restore
    # WHAT IT DOES: Shows checkpoint timeline with timestamps and changes
    # AGENT GUIDANCE: Use before restore to see available checkpoints

    print_header "Checkpoint History"

    if ! is_initialized; then
        print_error "Not initialized. Run: $0 setup"
        exit 1
    fi

    local limit="${1:-20}"
    "$TL_BINARY" log --limit "$limit"
}

cmd_status() {
    # WHEN TO USE: To check if daemon is running and see repository state
    # WHAT IT DOES: Shows daemon status, checkpoint count, and repository info
    # AGENT GUIDANCE: Use for debugging or checking system health

    print_header "Timelapse Status"

    if ! is_initialized; then
        print_error "Not initialized. Run: $0 setup"
        exit 1
    fi

    "$TL_BINARY" status
}

cmd_restore() {
    # WHEN TO USE: To undo changes and go back to a previous checkpoint
    # WHAT IT DOES: Restores working directory to exact state at checkpoint
    # AGENT GUIDANCE: Use when you want to undo recent changes
    # WARNING: This will overwrite current working directory!

    print_header "Restore from Checkpoint"

    if ! is_initialized; then
        print_error "Not initialized. Run: $0 setup"
        exit 1
    fi

    if [ -z "$1" ]; then
        print_error "Checkpoint ID required"
        echo ""
        echo "Usage: $0 restore <checkpoint-id>"
        echo ""
        print_info "Run '$0 log' to see available checkpoints"
        exit 1
    fi

    local checkpoint_id="$1"

    print_warning "This will restore your working directory to checkpoint: $checkpoint_id"
    print_warning "Current changes will be overwritten!"
    echo ""

    "$TL_BINARY" restore "$checkpoint_id"
}

cmd_diff() {
    # WHEN TO USE: To see what changed between two checkpoints
    # WHAT IT DOES: Shows file differences between checkpoints
    # AGENT GUIDANCE: Use to understand what changed before restoring

    print_header "Diff Between Checkpoints"

    if ! is_initialized; then
        print_error "Not initialized. Run: $0 setup"
        exit 1
    fi

    if [ -z "$1" ] || [ -z "$2" ]; then
        print_error "Two checkpoint IDs required"
        echo ""
        echo "Usage: $0 diff <checkpoint-a> <checkpoint-b>"
        exit 1
    fi

    "$TL_BINARY" diff "$1" "$2"
}

cmd_start() {
    # WHEN TO USE: To start the daemon if it's not running
    # WHAT IT DOES: Starts background daemon that creates automatic checkpoints
    # AGENT GUIDANCE: Usually called automatically by 'setup'

    print_header "Starting Daemon"

    if ! is_initialized; then
        print_error "Not initialized. Run: $0 setup"
        exit 1
    fi

    if is_daemon_running; then
        print_warning "Daemon is already running"
        exit 0
    fi

    "$TL_BINARY" start
    print_success "Daemon started"
}

cmd_stop() {
    # WHEN TO USE: To stop the daemon (e.g., before shutting down)
    # WHAT IT DOES: Gracefully stops the daemon after flushing pending changes
    # AGENT GUIDANCE: Usually not needed; daemon stops automatically on exit

    print_header "Stopping Daemon"

    if ! is_daemon_running; then
        print_warning "Daemon is not running"
        exit 0
    fi

    "$TL_BINARY" stop
    print_success "Daemon stopped"
}

cmd_info() {
    # WHEN TO USE: To see detailed repository information
    # WHAT IT DOES: Shows storage stats, checkpoint count, etc.
    # AGENT GUIDANCE: Use for debugging or monitoring

    print_header "Repository Information"

    if ! is_initialized; then
        print_error "Not initialized. Run: $0 setup"
        exit 1
    fi

    "$TL_BINARY" info
}

cmd_pin() {
    # WHEN TO USE: To mark important checkpoints with a memorable name
    # WHAT IT DOES: Assigns a name to a checkpoint for easy reference
    # AGENT GUIDANCE: Use to mark milestones (e.g., "working-auth", "before-refactor")

    if [ -z "$1" ] || [ -z "$2" ]; then
        print_error "Checkpoint ID and name required"
        echo ""
        echo "Usage: $0 pin <checkpoint-id> <name>"
        echo "Example: $0 pin 01HXKJ7NVQ working-version"
        exit 1
    fi

    "$TL_BINARY" pin "$1" "$2"
    print_success "Checkpoint pinned as: $2"
}

cmd_help() {
    cat << 'EOF'
╔══════════════════════════════════════════════════════════════════════════════╗
║                         Timelapse Client Wrapper                              ║
║                    Automatic Checkpoint Management for Git                    ║
╚══════════════════════════════════════════════════════════════════════════════╝

QUICK START (for AI Agents):
  1. ./tl-client.sh setup          # Initialize and start tracking
  2. <make changes to files>
  3. ./tl-client.sh save           # Create checkpoint
  4. ./tl-client.sh log            # View history
  5. ./tl-client.sh restore <id>   # Undo changes

COMMON WORKFLOW:
  • After making changes: ./tl-client.sh save
  • Before risky changes: ./tl-client.sh save (to create restore point)
  • To undo last changes: ./tl-client.sh log, then ./tl-client.sh restore <id>
  • To see what changed: ./tl-client.sh diff <id-a> <id-b>

AVAILABLE COMMANDS:

  setup                 Initialize timelapse and start daemon
                        WHEN: First time in a new repository
                        DOES: Creates .tl directory, starts background tracking

  save                  Create checkpoint immediately
                        WHEN: After completing a logical unit of work
                        DOES: Forces checkpoint creation (doesn't wait for auto)

  log [limit]           Show checkpoint history (default: 20)
                        WHEN: To see available checkpoints before restoring
                        DOES: Lists checkpoints with timestamps and changes

  status                Show daemon status and repository info
                        WHEN: To check if daemon is running
                        DOES: Displays system health and checkpoint count

  restore <id>          Restore working directory to checkpoint
                        WHEN: To undo changes or go back in time
                        DOES: Overwrites current files with checkpoint state
                        WARNING: Current changes will be lost!

  diff <id-a> <id-b>    Show changes between checkpoints
                        WHEN: To understand what changed
                        DOES: Displays file differences

  start                 Start the daemon
                        WHEN: If daemon stopped
                        DOES: Starts background checkpoint tracking

  stop                  Stop the daemon
                        WHEN: Before shutting down (optional)
                        DOES: Gracefully stops daemon

  info                  Show detailed repository information
                        WHEN: For debugging or monitoring
                        DOES: Shows storage stats, checkpoint count

  pin <id> <name>       Name a checkpoint for easy reference
                        WHEN: To mark important milestones
                        DOES: Assigns memorable name to checkpoint

  help                  Show this help message

CONCEPTS FOR AI AGENTS:

  Checkpoints:
    • Automatic snapshots of your entire working directory
    • Created every 5 seconds when files change
    • Stored efficiently using content-addressed storage
    • Can restore to any checkpoint instantly

  Daemon:
    • Background process that watches files
    • Creates checkpoints automatically
    • Must be running for automatic checkpoints
    • Stops gracefully when you exit

  When to Create Manual Checkpoints:
    ✓ Before starting risky refactoring
    ✓ After implementing a working feature
    ✓ Before experimenting with alternative approaches
    ✓ When you want guaranteed restore point

  When to Restore:
    ✓ Code broke and you want to go back
    ✓ Exploring alternative implementation
    ✓ Reverting accidental changes
    ✓ Testing different approaches

PERFORMANCE:
  • Checkpoint creation: < 1 second
  • Restore operation: < 100ms for most projects
  • Storage: ~1.2x Git size (deduplication + compression)
  • No impact on normal development workflow

EXAMPLES:

  # Initialize in new project
  $ ./tl-client.sh setup

  # Work on feature
  $ <edit files>
  $ ./tl-client.sh save

  # Check history
  $ ./tl-client.sh log

  # Restore to previous checkpoint
  $ ./tl-client.sh restore 01HXKJ7NVQW3Y2YMZK5VFZX3G8

  # See what changed
  $ ./tl-client.sh diff 01HXKJ7NVQ 01HXKJ8NVQ

  # Mark important checkpoint
  $ ./tl-client.sh pin 01HXKJ7NVQ working-auth-implementation

INTEGRATION WITH WORKFLOW:

  For AI Agents:
    1. ALWAYS run 'setup' when working in a new repository
    2. Run 'save' after each successful code generation
    3. Run 'save' before attempting risky changes
    4. Use 'log' + 'restore' if generated code breaks things
    5. Use 'diff' to understand what you changed

  Best Practices:
    • Create checkpoints at logical boundaries
    • Use descriptive pin names for important milestones
    • Check 'log' before 'restore' to pick the right checkpoint
    • 'save' is cheap - use it liberally!

TROUBLESHOOTING:

  Daemon not running:
    $ ./tl-client.sh start

  Daemon stuck:
    $ ./tl-client.sh stop
    $ ./tl-client.sh start

  Check system health:
    $ ./tl-client.sh status
    $ ./tl-client.sh info

  Binary not found:
    $ export TL_BINARY=/path/to/tl
    $ ./tl-client.sh setup

For more information: https://github.com/anthropics/timelapse
EOF
}

# ==============================================================================
# Main Command Router
# ==============================================================================

main() {
    check_tl_binary

    local command="${1:-help}"
    shift || true

    case "$command" in
        setup)
            cmd_setup "$@"
            ;;
        save|flush|checkpoint)
            cmd_save "$@"
            ;;
        log|history)
            cmd_log "$@"
            ;;
        status)
            cmd_status "$@"
            ;;
        restore|revert)
            cmd_restore "$@"
            ;;
        diff)
            cmd_diff "$@"
            ;;
        start)
            cmd_start "$@"
            ;;
        stop)
            cmd_stop "$@"
            ;;
        info)
            cmd_info "$@"
            ;;
        pin)
            cmd_pin "$@"
            ;;
        help|--help|-h)
            cmd_help
            ;;
        *)
            print_error "Unknown command: $command"
            echo ""
            echo "Run '$0 help' for usage information"
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
