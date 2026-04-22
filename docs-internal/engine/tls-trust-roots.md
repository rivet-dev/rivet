# TLS trust roots

Rules for outbound TLS client configuration across the repo.

## rustls clients: always union both root stores

For rustls-based outbound TLS clients (`tokio-tungstenite`, `reqwest`), always enable BOTH `rustls-tls-native-roots` and `rustls-tls-webpki-roots` together so the crates build a union root store.

- Operator-installed corporate CAs work via native.
- Empty native stores (Distroless / Cloud Run / Alpine without `ca-certificates`) fall through to the bundled Mozilla list.
- Never enable only one: native-only breaks on Distroless, webpki-only silently breaks corporate CAs.

Pinned in workspace `Cargo.toml` (`tokio-tungstenite`) and in `rivetkit-rust/packages/client/Cargo.toml` (`reqwest` + `tokio-tungstenite`).

## hyper-tls / native-tls clients stay on OpenSSL

Engine-internal HTTPS clients on `hyper-tls` / `native-tls` intentionally stay on OpenSSL. These include:

- workspace `reqwest`
- ClickHouse pool
- guard HTTP proxy

They run in operator-controlled containers and already honor the system trust store.

## Maintenance

- Bump `webpki-roots` periodically so the bundled Mozilla CA list does not go stale.
