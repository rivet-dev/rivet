# Effect SDK API Design

> **This is a design proposal, not a working example.** The `@rivetkit/effect` package does not exist yet. The code here shows the proposed API surface for an Effect-based SDK for Rivet Actors.

## Overview

This example demonstrates the proposed API design for `@rivetkit/effect`, an [Effect](https://effect.website/) SDK for Rivet Actors. The design leverages Effect's type system to provide:

- Schema-validated actions with typed errors
- Layer-based composition for actor registration, transport, and testing
- Compile-time tracking of actor dependencies via Effect's `R` type parameter
- Per-actor transport overrides and selective test mocking

## Files

- [`src/actors/Counter.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/effect/src/actors/Counter.ts) - Actor definition (public contract) and implementation (server-only Layer)
- [`src/main.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/effect/src/main.ts) - Server entry point using `Registry.layer`
- [`src/client.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/effect/src/client.ts) - Client usage with typed actor dependencies

