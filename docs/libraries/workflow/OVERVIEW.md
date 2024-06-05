# Overview

Workflows are designed to provide highly durable code executions for distributed systems. The main goal is to allow for writing easy to understand multi-step programs with effective error handling, retryability, and a rigid state.

## Goals

**Primary**

-   Performance
-   Quick iteration speed
-   Architectural simplicity (only depends on CockroachDB)

**Secondary**

-   Easy to operate, managable via simple SQL queries
-   Easier to write, understand, and maintain than event-driven architectures
-   Rust-native
    -   Run in-process and as part of the binary to simplify architecture
    -   Leverage traits to reduce copies and needless ser/de
    -   ## Use native serde instead of Protobuf for simplicity (**this comes at the cost of verifiable backwards compatibility with protobuf**)

## Use cases

-   Billing cron jobs with batch
-   Creating servers
-   Email loops
-   Creating dynamic servers
-   Automating Cloudflare APIs (Cloudflare workers, DNS, issuing SSL certs)

## Relation to existing Chirp primitives

### Messages

Workflows replace the use case of messages for durable execution, which is almost all uses of messages.

The biggest pain point with messages is the lack of a rigid state. Message executions always match the following outline:

1.  Read whatever data is required
2.  Perform some action(s)
3.  Update data as needed
4.  Finish (possibly publish more messages) OR upon failure, start all over at #1

The issue with this is that messages do not have any knowledge of messages that came before them, their own previous failed executions, or even other messages of the same system executing in parallel. Without thorough manually written sync checks and consistency validations (which are verbose and hard to follow), this type of execution often results in an overall broken state of whatever system the message is acting on (i.e. matchmaking, server provisioning).

**Once a broken state is reached, the retry system for messages _practically never_ successfully retries the message.**

### Cross-package hooks

We currently use messages for hooking in to events from other workflows so we don't have to bake in support directly.

This is potentially error prone since it makes control flow more opaque.

We will use sub workflows instead.

## Post-workflow message uses

Messages should still be used, but much less frequently. They're helpful for:

-   Real-time Data Processing
-   Complex Event Processing (CEP)
-   Data Transformation and Enrichment
-   Continuous Data Integration
-   Real-time Monitoring and Alerting
-   High-throughput, Low-latency Processing
