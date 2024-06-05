## Goals

**Primary**

- Performance
- Fast to write for
- Only depend on CockroachDB

**Secondary**

- Easy to monitor & manage via simple SQL queries
- Easier to understand than messages
- Rust-native
    - Run in-process and as part of the binary to simplify architecture
    - Leverage traits to reduce copies and needless ser/de
    - Use native serde instead of Protobuf for simplicity (**this comes at the cost of verifiable backwards compatability with protobuf**)
- Lay foundations for OpenGB

## Use cases

- Billing cron jobs with batch
- Creating servers
- Email loops
- Creating dynamic servers
    - What about dynamic server lifecycle? Is this more of an actor? This is blending between state and other stuff.
- Deploying CF workers

## Questions

- Concurrency
- Nondeterministic patches: https://docs.temporal.io/dev-guide/typescript/versioning#patching
- Do we plan to support side effects?

## Relation to existing Chirp primitives

### Messages

Workflows replace the usecase of messages for durable execution, which is almost all uses of messages.

Messages should still be used, but much less frequently. They're helpful for:

**Real-time Data Processing**

- When you have a continuous flow of data that needs to be processed in real-time or near-real-time.
- Examples include processing sensor data, social media feeds, financial market data, or clickstream data.
- Stream processing frameworks like Apache Kafka, Apache Flink, or Apache Spark Streaming are well-suited for handling high-volume, real-time data streams.

**Complex Event Processing (CEP)**

- When you need to detect and respond to patterns, correlations, or anomalies in real-time data streams.
- CEP involves analyzing and combining multiple event streams to identify meaningful patterns or trigger actions.
- Stream processing frameworks provide capabilities for defining and matching complex event patterns in real-time.

**Data Transformation and Enrichment**

- When you need to transform, enrich, or aggregate data as it arrives in real-time.
- This can involve tasks like data cleansing, normalization, joining with other data sources, or applying machine learning models.
- Stream processing allows you to process and transform data on-the-fly, enabling real-time analytics and insights.

**Continuous Data Integration**

- When you need to continuously integrate and process data from multiple sources in real-time.
- This can involve merging data streams, performing data synchronization, or updating downstream systems.
- Stream processing frameworks provide connectors and integrations with various data sources and sinks.

**Real-time Monitoring and Alerting**

- When you need to monitor data streams in real-time and trigger alerts or notifications based on predefined conditions.
- Stream processing allows you to define rules and thresholds to detect anomalies, errors, or critical events and send real-time alerts.

**High-throughput, Low-latency Processing**

- When you have a high volume of data that needs to be processed with low latency.
- Stream processing frameworks are designed to handle high-throughput data streams and provide low-latency processing capabilities.
- This is particularly useful in scenarios like fraud detection, real-time recommendations, or real-time bidding in advertising systems.

### Cross-package hooks

We currently use messages for hooking in to events from other workflows so we don't have to bake in support directly.

This is potentially error prone since it makes control flow more opaque.

TBD on if we keed this pattern.

### Workflows & operations across packages

**Child workflows**

TODO

**Operations**

TODO

## Temporal docs

https://docs.temporal.io/encyclopedia/

