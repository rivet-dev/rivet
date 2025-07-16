# Cross-Origin Resource Sharing

Cross-Origin Resource Sharing (CORS) is a security mechanism that allows a web application running at one origin to access resources from a different origin. Without CORS, browsers block cross-origin HTTP requests by default as a security measure.

You'll need to configure CORS when:

- **Local Development**: You're developing locally and your client runs on a different port than your actor service
- **Different Domain**: Your frontend application is hosted on a different domain than your actor service

## Registry-Level CORS

Configure CORS directly in your registry setup for simple cases:

```typescript
const registry = setup(,
  cors: 
});
```

This approach works well for basic setups but has limitations for complex scenarios.

## Router-Level CORS (Recommended)

For production applications, configure CORS at the router level for maximum control:

```typescript
const  = registry.createServer();
const app = new Hono();

app.use("*", cors());

serve(app);
```

### Required Headers for Rivet

Rivet requires specific headers for communication. Always include `ALLOWED_PUBLIC_HEADERS`:

```typescript
// ALLOWED_PUBLIC_HEADERS includes:
// - "Content-Type"
// - "User-Agent" 
// - "X-RivetKit-Query"
// - "X-RivetKit-Encoding"
// - "X-RivetKit-Conn-Params"
// - "X-RivetKit-Actor"
// - "X-RivetKit-Conn"
// - "X-RivetKit-Conn-Token"

const corsConfig = ;
```

Without `ALLOWED_PUBLIC_HEADERS`, Rivet clients won't be able to communicate with your actors from the browser.

## Framework-Specific Examples

### Express.js

```typescript
const  = registry.createServer();
const app = express();

app.use(cors());

app.use("/registry", handler);
app.listen(8080);
```

### Elysia

```typescript
const  = registry.createServer();

const app = new Elysia()
  .use(cors())
  .mount("/registry", handler)
  .listen(8080);
```

## Configuration Options

### `origin`

`string | string[] | (origin: string) => boolean | string`

Specifies which domains can access your resources:

```typescript
// Single domain
origin: "https://example.com"

// Multiple domains
origin: ["https://app.com", "https://admin.com"]

// Dynamic validation
origin: (origin) => 

// All domains (not recommended for production)
origin: "*"
```

### `allowMethods`

`string[]`

HTTP methods clients are allowed to use:

```typescript
allowMethods: ["GET", "POST", "OPTIONS"]  // Common for Rivet
```

### `allowHeaders`

`string[]`

Headers that clients can send in requests:

```typescript
allowHeaders: [
  "Authorization",           // Your auth headers
  "Content-Type",           // Standard content type
  "X-API-Key",              // Custom API key header
  ...ALLOWED_PUBLIC_HEADERS // Required Rivet headers
]
```

### `credentials`

`boolean`

Whether to allow credentials (cookies, auth headers):

```typescript
credentials: true  // Required for authentication
```

When `credentials: true`, you cannot use `origin: "*"`. Specify exact origins instead.

### `maxAge`

`number`

How long browsers cache CORS preflight responses (in seconds):

```typescript
maxAge: 600  // Cache for 10 minutes
```

### `exposeHeaders`

`string[]`

Server headers that browsers can access:

```typescript
exposeHeaders: ["Content-Length", "X-Request-Id"]
```

## Development vs Production

### Development Setup

For local development, allow localhost origins:

```typescript
const isDev = process.env.NODE_ENV !== "production";

const corsConfig = ;
```

### Production Setup

For production, be restrictive with origins:

```typescript
const corsConfig = ;
```

## Troubleshooting

### Common CORS Errors

**"Access to fetch blocked by CORS policy"**
- Add your frontend's origin to the `origin` list
- Ensure `ALLOWED_PUBLIC_HEADERS` are included in `allowHeaders`

**"Request header not allowed"**
- Add the missing header to `allowHeaders` 
- Include `ALLOWED_PUBLIC_HEADERS` in your configuration

**"Credentials mode mismatch"**
- Set `credentials: true` in CORS config
- Cannot use `origin: "*"` with credentials

### Debug CORS Issues

Enable CORS logging to debug issues:

```typescript
// Log CORS requests in development
if (process.env.NODE_ENV === "development") );
    await next();
  });
}
```

## Best Practices

1. **Use Router-Level CORS**: More flexible than registry-level configuration
2. **Include ALLOWED_PUBLIC_HEADERS**: Required for Rivet communication
3. **Specify Exact Origins**: Avoid wildcards in production
4. **Enable Credentials**: Needed for authentication  
5. **Cache Preflight Requests**: Use appropriate `maxAge` values
6. **Environment-Specific Config**: Different settings for dev/prod