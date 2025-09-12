# Rivet Studio

Rivet Studio is a web-based development tool for debugging and monitoring your Rivet Actors in real-time.

## Features

- **View running actors**: See all active Rivet Actors in your application
- **Edit actor state**: Modify actor state in real-time for debugging
- **REPL for actions**: Interactive console to call actor actions directly
- **Event monitoring**: Track actor events including actions, broadcasts, events, and subscriptions
- **Connection viewer**: Monitor all connected connections & their associated state

## How it works

RivetKit automatically mounts a route at `/registry` on your server for Studio access. When your application starts:

1. RivetKit generates a secure token for authentication on startup
2. Open to [studio.rivet.gg](https://studio.rivet.gg) in your browser or click the `studio.rivet.gg` URL that's printed to your console on startup
3. Rivet Studio connects to your application

Rivet Studio is automatically disabled in production when `NODE_ENV=production`.

## Configuration

### Token configuration

By default, Rivet Studio generates and stores a token automatically. You can configure it:

- **Environment variable**: Set `RIVETKIT_STUDIO_TOKEN`
- **Code configuration**:
  ```typescript }
  registry.runServer(
  })
  ```

### Disabling the Studio

Disable Studio using any of these methods:

- Set `RIVETKIT_STUDIO_DISABLE` environment variable
- Set `NODE_ENV=production`
- Configure in code:
  ```typescript }
  registry.runServer(
  })
  ```

### CORS configuration

Configure CORS for custom Studio deployments:

```typescript }
registry.runServer(
  }
})
```

See the [CORS documentation](/docs/general/cors/) for more details.

### Default endpoint

On startup, RivetKit prints a URL for connecting to Studio. By default, Studio connects to `localhost:8080` if no endpoint is provided. Override with:

```typescript }
registry.runServer(
})
```