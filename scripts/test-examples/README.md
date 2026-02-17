# test-examples

Runtime validation for RivetKit examples. Starts each example's dev server and verifies endpoints are reachable.

## Usage

```bash
# Test all examples
pnpm start

# Test specific example
pnpm start --example chat-room

# Skip certain examples
pnpm start --skip ai-agent,drizzle
```

## What it tests

For each example:

1. Starts `pnpm dev`
2. Checks index page at `http://localhost:5173`
3. Checks RivetKit API at `/api/rivet/`
4. Checks RivetKit manager at `http://localhost:6420`

## Configuration validation

Static validation of example configurations (README format, package.json structure, turbo.json, etc.) is handled by repository CI checks.
