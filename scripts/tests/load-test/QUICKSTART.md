# Quick Start Guide

Get started with Rivet Actor load testing in 5 minutes.

## Prerequisites Check

```bash
# 1. Check if k6 is installed
k6 version

# If not installed:
# macOS: brew install k6
# Linux: See README.md for installation instructions
# Windows: choco install k6
```

## Step 1: Start Test Runner

Open a terminal and start the test runner:

```bash
cd engine/sdks/typescript/test-runner
pnpm install  # First time only
pnpm dev
```

Keep this terminal open. You should see:
```
Starting runner
Runner started
```

## Step 2: Run Your First Load Test

Open a new terminal and run:

```bash
cd scripts/tests
tsx load-test/run.ts --stages "1m:5"
```

This will:
- ✓ Check if k6 is installed
- ✓ Verify test runner is healthy
- 🚀 Start a 1-minute test with 5 virtual users
- 📊 Show real-time metrics
- ✅ Display summary with pass/fail thresholds

## Step 3: View Results

After the test completes, you'll see a summary like:

```
     ✓ actor_create_success.........: 98.50% ✓ 197  ✗ 3
     ✓ actor_destroy_success........: 99.00% ✓ 198  ✗ 2
     ✓ actor_ping_success...........: 98.00% ✓ 196  ✗ 4
     ✓ websocket_success............: 95.00% ✓ 190  ✗ 10
     ✓ http_req_duration............: avg=234ms min=120ms med=210ms max=890ms p(90)=345ms p(95)=456ms p(99)=678ms
```

✓ means the test passed all thresholds!

## Next Steps

### Run Different Test Scenarios

```bash
# Quick test (30 seconds)
tsx load-test/run.ts --stages "30s:5"

# Stress test (ramp up to 50 users)
tsx load-test/run.ts --stages "2m:10,5m:50,2m:0"

# Save results to file
tsx load-test/run.ts --stages "1m:10" --out json=results.json
```

### Use pnpm Scripts

```bash
cd scripts/tests

# Quick 1-minute test
pnpm load-test:quick

# Stress test
pnpm load-test:stress

# Custom options
pnpm load-test -- --stages "2m:15" --quiet
```

### View All Options

```bash
tsx load-test/run.ts --help
```

## Common Issues

### "k6 is not installed"
Install k6: https://k6.io/docs/get-started/installation/

### "Test Runner Not Running"
Make sure the test runner is started:
```bash
cd engine/sdks/typescript/test-runner && pnpm dev
```

### High Failure Rates
- Reduce the number of VUs (e.g., `--stages "1m:3"`)
- Check test runner logs for errors
- Verify Rivet engine is running

## Understanding the Output

### Key Metrics

- **actor_create_success**: % of actors successfully created
- **actor_destroy_success**: % of actors successfully destroyed
- **websocket_success**: % of successful WebSocket connections
- **http_req_duration**: HTTP request latency (p95 should be < 5s)
- **actor_create_duration**: Time to create an actor (p95 should be < 3s)

### Thresholds

Tests fail if:
- Actor operations < 95% success rate
- WebSocket connections < 90% success rate
- HTTP p95 latency > 5 seconds
- HTTP p99 latency > 10 seconds

## What Gets Tested

Each virtual user:
1. Creates a unique actor
2. Pings the actor via HTTP
3. Connects via WebSocket and exchanges messages
4. Puts the actor to sleep
5. Wakes the actor with a ping
6. Destroys the actor

This tests the complete actor lifecycle under load.

## More Information

See [README.md](./README.md) for comprehensive documentation.
