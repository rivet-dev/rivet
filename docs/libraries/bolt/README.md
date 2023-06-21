# 🔩 Bolt

Bolt is the internal tool used to manage Rivet infrastructure & services. If you plan to develop on Rivet, you'll be using Bolt heavily.

### Start a service

Once you make a change to a service, run the following:

```
bolt up api-identity
```

If you want to start all services, run:

```
bolt up
```

### Test a service

Rivet uses unit & integration tests heavily. Run a test against a service like so:

```
bolt test user-get
```

This will automatically boot any services required to run the test.

To run a test with a specific name, pass the `-n` flag like so:

```
bolt test user-get -n fetch
```

### More commands

Bolt has a built-in help system. Explore for yourself.

```
bolt --help
```

### Hacking on Bolt

The Bolt source code lives in [`lib/bolt/cli/`](../../../lib/bolt/cli). Find something that Bolt could do better? Submit a PR!
