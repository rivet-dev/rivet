# Load Tests

## Running

To run a single iteration to test a lod test script, run:

```
just k6-test <name>
```

To run the full test, run:

```
just k6-run <name>
```

## Configuring

Load tests respect these env vars:

```
RIVET_ENDPOINT
RIVET_SERVICE_TOKEN
RIVET_PROJECT
RIVET_ENVIRONMENT
REGION
```

