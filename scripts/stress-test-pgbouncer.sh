#!/usr/bin/env bash
# Stress test comparing PgBouncer vs direct PostgreSQL.
# Runs as a Kubernetes Job in the rivet-engine namespace.
# Uses pgbench (bundled with postgres:17) against both endpoints.

set -euo pipefail

PG="postgres"
PB="pgbouncer"
USER="postgres"
DB="rivet"

header() {
    echo ""
    echo "================================================================"
    echo "  $1"
    echo "================================================================"
}

run() {
    local host=$1 label=$2
    shift 2
    printf "\n  [%-12s] pgbench %s\n" "$label" "$*"
    PGPASSWORD="$PGPASSWORD" pgbench -h "$host" -U "$USER" -d "$DB" "$@" 2>&1 \
        | grep -E "tps|latency|connection time|number of|ERROR|FATAL" \
        || echo "  *** FAILED (exit $?) ***"
}

# ---------------------------------------------------------------------------
# Init
# ---------------------------------------------------------------------------
header "Initializing pgbench schema (scale factor 10)"
PGPASSWORD="$PGPASSWORD" pgbench -h "$PG" -U "$USER" -d "$DB" -i -s 10 -q

# ---------------------------------------------------------------------------
# TEST 1 — Throughput, sustained connections
#
# Both targets should perform similarly here. This is the baseline.
# Each client opens one connection and keeps it for the whole run.
# ---------------------------------------------------------------------------
header "TEST 1: Throughput — 50 clients, sustained connections, 30s"
echo "  Expect: similar TPS. PgBouncer adds negligible overhead in session mode."

run "$PG" "PostgreSQL"  -c 50 -j 10 -T 30 -S -P 10
run "$PB" "PgBouncer"   -c 50 -j 10 -T 30 -S -P 10

# ---------------------------------------------------------------------------
# TEST 2 — Connection churn
#
# -C reconnects to the server on every transaction, simulating short-lived
# connections (serverless functions, scripts, etc.).
# Direct postgres: full TCP + SCRAM/MD5 handshake per transaction.
# PgBouncer: client reconnects cheaply; server connection stays in the pool.
# ---------------------------------------------------------------------------
header "TEST 2: Connection churn — 50 clients, reconnect per transaction, 30s"
echo "  Expect: PgBouncer TPS >> PostgreSQL TPS (connection reuse in pool)."
echo "  'connection time' in pgbench output shows per-connection overhead."

run "$PG" "PostgreSQL"  -c 50 -j 10 -T 30 -S -C
run "$PB" "PgBouncer"   -c 50 -j 10 -T 30 -S -C

# ---------------------------------------------------------------------------
# TEST 3 — Overload (more clients than postgres max_connections)
#
# postgres max_connections=200; we use 210 clients to exceed the limit.
# Direct postgres: connections beyond max_connections are rejected immediately.
# PgBouncer (session mode): clients reconnect per transaction (-C), so the
# 90-connection pool is shared over time — all 210 clients complete successfully.
# Without -C, session mode would also fail because each sustained client holds
# its server connection for the full duration.
# ---------------------------------------------------------------------------
header "TEST 3: Overload — 210 clients (exceeds postgres max_connections=200)"
echo "  Expect: PostgreSQL rejects connections/errors; PgBouncer serves all via pool reuse."

run "$PG" "PostgreSQL"  -c 210 -j 20 -T 30 -S -C || true
run "$PB" "PgBouncer"   -c 210 -j 20 -T 30 -S -C

echo ""
echo "================================================================"
echo "  Done."
echo "================================================================"
