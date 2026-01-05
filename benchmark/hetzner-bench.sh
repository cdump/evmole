#!/usr/bin/env bash
set -euo pipefail

# Hetzner Cloud Benchmark Runner for evmole
# Creates a dedicated vCPU server, runs benchmarks, downloads results, destroys server

# === Configuration ===
API_BASE="https://api.hetzner.cloud/v1"
DEFAULT_SERVER_TYPE="ccx13"
DEFAULT_LOCATION="nbg1"
DEFAULT_IMAGE="ubuntu-24.04"
DEFAULT_REPO="https://github.com/cdump/evmole.git"
DEFAULT_BRANCH="master"
VALID_BENCHMARKS="selectors arguments mutability storage flow all"

# === Global State ===
SERVER_ID=""
SERVER_IP=""
KEEP_SERVER=false
SKIP_SETUP=""  # empty = auto (skip if --server-ip, run otherwise)
VERBOSE=false
USE_EXISTING_SERVER=false
SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=10 -o BatchMode=yes"

# === Colors (if terminal) ===
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' NC=''
fi

# === Helper Functions ===
log()   { echo -e "${GREEN}[$(date '+%H:%M:%S')]${NC} $*" >&2; }
warn()  { echo -e "${YELLOW}[$(date '+%H:%M:%S')] WARN:${NC} $*" >&2; }
error() { echo -e "${RED}[$(date '+%H:%M:%S')] ERROR:${NC} $*" >&2; }
debug() { [[ "$VERBOSE" == "true" ]] && echo -e "${BLUE}[$(date '+%H:%M:%S')] DEBUG:${NC} $*" >&2 || true; }

die() {
    error "$*"
    exit 1
}

usage() {
    cat <<EOF
Usage: $(basename "$0") -b <benchmark> -k <ssh-key> [options]
       $(basename "$0") -b <benchmark> --server-ip <ip> [options]

Run evmole benchmarks on Hetzner Cloud dedicated vCPU servers.

Required:
  -b, --benchmark <name>   Benchmark to run: selectors|arguments|mutability|storage|flow|all

Server (one of):
  -k, --ssh-key <name>     SSH key name - creates new server (must exist in Hetzner account)
  -s, --server-ip <ip>     Use existing server IP (skips creation, setup, and destruction)

Options:
  -p, --providers <list>   Space-separated list of providers to run (default: all for benchmark type)
  -d, --datasets <list>    Space-separated list of datasets (default: largest1k random50k vyper)
  -t, --server-type <type> Server type (default: $DEFAULT_SERVER_TYPE)
                           CCX series: ccx13 (2 vCPU), ccx23 (4 vCPU), ccx33 (8 vCPU), etc.
  -l, --location <dc>      Datacenter location (default: $DEFAULT_LOCATION)
                           Options: fsn1, nbg1, hel1, ash, hil
  -o, --output <dir>       Local directory for results (default: ./results-hetzner-<timestamp>)
  -r, --repo <url>         Git repository URL (default: $DEFAULT_REPO)
      --branch <branch>    Git branch to checkout (default: $DEFAULT_BRANCH)
      --keep-server        Do not destroy server after completion (for debugging)
      --setup              Run setup (Docker, clone) even with --server-ip (default: skip)
  -v, --verbose            Enable verbose output
  -h, --help               Show this help message

Environment:
  HETZNER_API_TOKEN        Required when creating new server (-k)

Examples:
  # Run selectors benchmark (creates new server)
  export HETZNER_API_TOKEN="your-token-here"
  ./hetzner-bench.sh -b selectors -k my-ssh-key

  # Run all benchmarks on larger server
  ./hetzner-bench.sh -b all -k my-ssh-key -t ccx23

  # Keep server for debugging, then rerun with different benchmark
  ./hetzner-bench.sh -b selectors -k my-ssh-key --keep-server
  ./hetzner-bench.sh -b arguments --server-ip 95.216.x.x

  # Rerun benchmark on existing server (setup skipped by default)
  ./hetzner-bench.sh -b selectors --server-ip 95.216.x.x

  # Run only specific providers
  ./hetzner-bench.sh -b selectors -k my-ssh-key -p "evmole-rs evmole-go"

  # Run on specific dataset
  ./hetzner-bench.sh -b selectors -k my-ssh-key -d "largest1k"
EOF
    exit "${1:-0}"
}

# API call wrapper with error handling
api_call() {
    local method="$1"
    local endpoint="$2"
    local data="${3:-}"
    local response http_code body

    debug "API $method $endpoint"
    [[ -n "$data" ]] && debug "Request body: $data"

    if [[ -n "$data" ]]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            -H "Authorization: Bearer $HETZNER_API_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$data" \
            "${API_BASE}${endpoint}" 2>&1) || die "curl failed"
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" \
            -H "Authorization: Bearer $HETZNER_API_TOKEN" \
            "${API_BASE}${endpoint}" 2>&1) || die "curl failed"
    fi

    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')

    debug "Response ($http_code): $body"

    if [[ ! "$http_code" =~ ^2 ]]; then
        local err_msg
        err_msg=$(echo "$body" | jq -r '.error.message // .message // "Unknown error"' 2>/dev/null || echo "$body")
        error "API call failed: $method $endpoint (HTTP $http_code)"
        error "Response: $err_msg"
        return 1
    fi

    echo "$body"
}

# Create server and return ID
create_server() {
    local name="$1"
    local server_type="$2"
    local location="$3"
    local ssh_key_name="$4"
    local response server_id

    log "Creating server '$name' ($server_type in $location)..."

    response=$(api_call POST "/servers" "$(cat <<EOF
{
  "name": "$name",
  "server_type": "$server_type",
  "location": "$location",
  "image": "$DEFAULT_IMAGE",
  "ssh_keys": ["$ssh_key_name"],
  "start_after_create": true,
  "labels": {
    "purpose": "evmole-benchmark",
    "created-by": "hetzner-bench"
  }
}
EOF
    )") || die "Failed to create server"

    server_id=$(echo "$response" | jq -r '.server.id')
    [[ -z "$server_id" || "$server_id" == "null" ]] && die "Failed to parse server ID from response"

    echo "$server_id"
}

# Wait for server to be running and get IP
wait_for_server() {
    local server_id="$1"
    local max_attempts=60
    local attempt=0
    local status ip

    log "Waiting for server to be ready..."

    while [[ $attempt -lt $max_attempts ]]; do
        response=$(api_call GET "/servers/$server_id") || die "Failed to get server status"
        status=$(echo "$response" | jq -r '.server.status')
        ip=$(echo "$response" | jq -r '.server.public_net.ipv4.ip')

        debug "Server status: $status, IP: $ip"

        if [[ "$status" == "running" && -n "$ip" && "$ip" != "null" ]]; then
            echo "$ip"
            return 0
        fi

        ((attempt++))
        sleep 2
    done

    die "Timeout waiting for server to be ready (status: $status)"
}

# Wait for SSH to be accessible
wait_for_ssh() {
    local ip="$1"
    local max_attempts=30
    local attempt=0

    log "Waiting for SSH access..."

    while [[ $attempt -lt $max_attempts ]]; do
        if ssh $SSH_OPTS "root@${ip}" "exit" 2>/dev/null; then
            log "SSH ready"
            return 0
        fi
        debug "SSH attempt $((attempt+1))/$max_attempts failed, retrying..."
        ((attempt++))
        sleep 5
    done

    die "Timeout waiting for SSH access"
}

# Execute command on remote server
remote_exec() {
    local ip="$1"
    shift
    debug "Remote exec: $*"
    if [[ "$VERBOSE" == "true" ]]; then
        ssh $SSH_OPTS "root@${ip}" "$@"
    else
        ssh $SSH_OPTS "root@${ip}" "$@" 2>&1
    fi
}

# Setup server (install Docker, git-lfs, clone repo)
setup_server() {
    local ip="$1"
    local repo="$2"
    local branch="$3"

    log "Installing dependencies..."
    remote_exec "$ip" "apt-get update && apt-get install -y build-essential git-lfs" >/dev/null || die "Failed to install dependencies"
    remote_exec "$ip" "git lfs install" >/dev/null || die "Failed to setup git-lfs"

    log "Installing Docker..."
    remote_exec "$ip" "curl -fsSL https://get.docker.com | sh" >/dev/null || die "Failed to install Docker"

    log "Cloning repository (branch: $branch)..."
    remote_exec "$ip" "git clone --branch '$branch' '$repo' /root/evmole" >/dev/null || die "Failed to clone repository"

    log "Initializing submodules and pulling LFS files..."
    remote_exec "$ip" "cd /root/evmole && git submodule update --init --recursive" >/dev/null || die "Failed to init submodules"
    remote_exec "$ip" "cd /root/evmole/benchmark/datasets && git lfs pull" >/dev/null || die "Failed to pull LFS files"

    log "Server setup complete"
}

# Run benchmarks
run_benchmarks() {
    local ip="$1"
    local providers="$2"
    local datasets="$3"
    shift 3
    local benchmarks=("$@")

    # Build make arguments
    local make_args=""
    if [[ -n "$providers" ]]; then
        make_args+=" PROVIDERS_SELECTORS='$providers' PROVIDERS_ARGUMENTS='$providers' PROVIDERS_MUTABILITY='$providers' PROVIDERS_STORAGE='$providers' PROVIDERS_BLOCKS='$providers' PROVIDERS_FLOW='$providers'"
    fi
    if [[ -n "$datasets" ]]; then
        make_args+=" DATASETS='$datasets' DATASETS_STORAGE='$datasets'"
    fi

    for bench in "${benchmarks[@]}"; do
        local info="$bench"
        [[ -n "$providers" ]] && info+=" | providers: $providers"
        [[ -n "$datasets" ]] && info+=" | datasets: $datasets"
        log "Running benchmark: $info"
        local start_time=$SECONDS

        if ! remote_exec "$ip" "cd /root/evmole/benchmark && make benchmark-$bench $make_args"; then
            error "Benchmark '$bench' failed"
            return 1
        fi

        local elapsed=$((SECONDS - start_time))
        log "Benchmark '$bench' completed in ${elapsed}s"
    done
}

# Download results
download_results() {
    local ip="$1"
    local output_dir="$2"

    log "Downloading results to $output_dir..."
    mkdir -p "$output_dir"

    scp $SSH_OPTS "root@${ip}:/root/evmole/benchmark/results/*.json" "$output_dir/" || die "Failed to download results"

    local count
    count=$(find "$output_dir" -name "*.json" | wc -l)
    log "Downloaded $count result files"
}

# Cleanup: destroy server
cleanup() {
    # Don't destroy if using existing server or no server was created
    if [[ "$USE_EXISTING_SERVER" == "true" || -z "$SERVER_ID" ]]; then
        return 0
    fi

    if [[ "$KEEP_SERVER" == "true" ]]; then
        warn "Server kept running (--keep-server): ID=$SERVER_ID, IP=$SERVER_IP"
        warn "Don't forget to destroy it manually: curl -X DELETE -H 'Authorization: Bearer \$HETZNER_API_TOKEN' $API_BASE/servers/$SERVER_ID"
    else
        log "Destroying server $SERVER_ID..."
        api_call DELETE "/servers/$SERVER_ID" >/dev/null 2>&1 || warn "Failed to destroy server (may already be deleted)"
    fi
}

# === Main ===
main() {
    local benchmarks=()
    local ssh_key=""
    local server_type="$DEFAULT_SERVER_TYPE"
    local location="$DEFAULT_LOCATION"
    local output_dir=""
    local repo="$DEFAULT_REPO"
    local branch="$DEFAULT_BRANCH"
    local providers=""
    local datasets=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        # Handle --option=value syntax
        if [[ "$1" == *=* ]]; then
            set -- "${1%%=*}" "${1#*=}" "${@:2}"
        fi
        case "$1" in
            -b|--benchmark)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                benchmarks+=("$2")
                shift 2
                ;;
            -k|--ssh-key)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                ssh_key="$2"
                shift 2
                ;;
            -t|--server-type)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                server_type="$2"
                shift 2
                ;;
            -l|--location)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                location="$2"
                shift 2
                ;;
            -p|--providers)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                providers="$2"
                shift 2
                ;;
            -d|--datasets)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                datasets="$2"
                shift 2
                ;;
            -o|--output)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                output_dir="$2"
                shift 2
                ;;
            -r|--repo)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                repo="$2"
                shift 2
                ;;
            --branch)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                branch="$2"
                shift 2
                ;;
            -s|--server-ip)
                [[ -z "${2:-}" ]] && die "Missing argument for $1"
                SERVER_IP="$2"
                USE_EXISTING_SERVER=true
                shift 2
                ;;
            --keep-server)
                KEEP_SERVER=true
                shift
                ;;
            --setup)
                SKIP_SETUP=false
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -h|--help)
                usage 0
                ;;
            *)
                die "Unknown option: $1"
                ;;
        esac
    done

    # Validate required arguments
    [[ ${#benchmarks[@]} -eq 0 ]] && die "Missing required argument: -b/--benchmark"

    if [[ "$USE_EXISTING_SERVER" == "true" ]]; then
        # Using existing server - no need for SSH key or API token
        [[ -z "$SERVER_IP" ]] && die "Missing server IP for --server-ip"
        # Default to skip setup for existing server
        [[ -z "$SKIP_SETUP" ]] && SKIP_SETUP=true
    else
        # Creating new server - need SSH key and API token
        [[ -z "$ssh_key" ]] && die "Missing required argument: -k/--ssh-key or -s/--server-ip"
        [[ -z "${HETZNER_API_TOKEN:-}" ]] && die "HETZNER_API_TOKEN environment variable is required"
        SKIP_SETUP=false
    fi

    # Validate benchmarks
    local expanded_benchmarks=()
    for bench in "${benchmarks[@]}"; do
        if [[ "$bench" == "all" ]]; then
            expanded_benchmarks+=(selectors arguments mutability storage flow)
        elif [[ " $VALID_BENCHMARKS " =~ " $bench " ]]; then
            expanded_benchmarks+=("$bench")
        else
            die "Invalid benchmark: $bench (valid: $VALID_BENCHMARKS)"
        fi
    done
    benchmarks=("${expanded_benchmarks[@]}")

    # Set default output directory
    [[ -z "$output_dir" ]] && output_dir="./results-hetzner-$(date +%Y%m%d-%H%M%S)"

    # Check dependencies
    for cmd in curl jq ssh scp; do
        command -v "$cmd" >/dev/null || die "Required command not found: $cmd"
    done

    # Setup cleanup trap
    trap cleanup EXIT INT TERM

    local start_time=$SECONDS

    log "Starting benchmark automation"
    log "  Benchmarks: ${benchmarks[*]}"

    if [[ "$USE_EXISTING_SERVER" == "true" ]]; then
        # Use existing server
        log "  Using existing server: $SERVER_IP"

        # Wait for SSH
        wait_for_ssh "$SERVER_IP" || exit 5

        # Setup if not skipped
        if [[ "$SKIP_SETUP" != "true" ]]; then
            setup_server "$SERVER_IP" "$repo" "$branch" || exit 6
        else
            log "Skipping setup (--skip-setup)"
        fi
    else
        # Create new server
        log "  Server: $server_type @ $location"
        log "  Repo: $repo (branch: $branch)"

        # Create server
        local server_name="evmole-bench-$(date +%s)"
        SERVER_ID=$(create_server "$server_name" "$server_type" "$location" "$ssh_key") || exit 4
        log "Server created: ID=$SERVER_ID"

        # Wait for server
        SERVER_IP=$(wait_for_server "$SERVER_ID") || exit 4
        log "Server IP: $SERVER_IP"

        # Wait for SSH
        wait_for_ssh "$SERVER_IP" || exit 5

        # Setup server
        setup_server "$SERVER_IP" "$repo" "$branch" || exit 6
    fi

    # Run benchmarks
    run_benchmarks "$SERVER_IP" "$providers" "$datasets" "${benchmarks[@]}" || exit 7

    # Download results
    download_results "$SERVER_IP" "$output_dir" || exit 8

    local total_time=$((SECONDS - start_time))
    log "Done! Total time: $((total_time / 60))m $((total_time % 60))s"
    log "Results saved to: $output_dir"
    # Build compare.py command suggestion
    local compare_cmd="python3 compare.py --results-dir '$output_dir'"
    [[ ${#benchmarks[@]} -eq 1 ]] && compare_cmd+=" --mode ${benchmarks[0]}"
    [[ -n "$providers" ]] && compare_cmd+=" --providers $providers"
    [[ -n "$datasets" ]] && compare_cmd+=" --datasets $datasets"
    log "Analyze with: $compare_cmd"
}

main "$@"
