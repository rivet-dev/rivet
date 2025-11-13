# Cloudflare Workers Inline Client Example

Simple example demonstrating accessing Rivet Actors via Cloudflare Workers without exposing a public API. This uses the `createInlineClient` function to connect directly to your Durable Object.

## Getting Started

Install dependencies:

```sh
pnpm install
```

Start the development server:

```sh
pnpm run dev
```

In a separate terminal, test the endpoint:

```sh
pnpm run client-http
```

Or:

```sh
pnpm run client-rivetkit
```

## License

Apache 2.0
