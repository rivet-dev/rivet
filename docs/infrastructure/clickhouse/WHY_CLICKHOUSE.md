# Why ClickHouse?

## Motivation

Rivet has a series of OLAP use cases, such as analytics, log aggregation, and more.

## Requirements

-   Self hostable
-   Easy to operate
-   Realtime aggregates
-   Cost effective
-   Turnkey aggregation to avoid the TCO of a stream/batch processing framework

### Nice to haves

-   Tiered storage
-   Maintain a similar workflow to CockroachDB

## Alternatives

### CockroachDB

We already run CockroachDB, so it makes sense to push it to its limit until it can't scale any more.

**Vectorized execution**

CockroachDB supports [vectorized execution](https://www.cockroachlabs.com/docs/stable/vectorized-execution.html) in order to optimize for some of the aggregating workloads that we use today in ClickHouse. This takes advantage of similar mechanics you find in [column-oriented databases](https://clickhouse.com/docs/en/faq/general/columnar-database) within an OLTP database. However, Cokroach will never be architected as a column-oriented database to achieve the same performance as ClickHouse.

**Ingestion**

We need to ingest large amounts of logs and analytics events in Rivet. While there has been a lot of work [invested in ingestion performance](https://www.cockroachlabs.com/blog/bulk-data-import/), the hardware required to achieve similar ingestion rates to ClickHouse is much more expensive.

ClickHouse ingestion rates are 4.4x faster and take 5.3x less storage space that Postgres.

**Benchmarks**

While comparing benchmarks to Postgres is not quite a fair comparision, it's a _close enough_ comparision to Cockroach's performance given they have similar design constraints. More work should be done to compare the two head to head.

[ClickHouse <-> Postgres benchmarks](https://benchmark.clickhouse.com/#eyJzeXN0ZW0iOnsiQXRoZW5hIChwYXJ0aXRpb25lZCkiOmZhbHNlLCJBdGhlbmEgKHNpbmdsZSkiOmZhbHNlLCJBdXJvcmEgZm9yIE15U1FMIjpmYWxzZSwiQXVyb3JhIGZvciBQb3N0Z3JlU1FMIjpmYWxzZSwiQnl0ZUhvdXNlIjpmYWxzZSwiY2hEQiI6ZmFsc2UsIkNpdHVzIjpmYWxzZSwiQ2xpY2tIb3VzZSAoZGF0YSBsYWtlLCBwYXJ0aXRpb25lZCkiOmZhbHNlLCJDbGlja0hvdXNlIChQYXJxdWV0LCBwYXJ0aXRpb25lZCkiOmZhbHNlLCJDbGlja0hvdXNlIChQYXJxdWV0LCBzaW5nbGUpIjpmYWxzZSwiQ2xpY2tIb3VzZSI6dHJ1ZSwiQ2xpY2tIb3VzZSAodHVuZWQpIjp0cnVlLCJDbGlja0hvdXNlICh6c3RkKSI6ZmFsc2UsIkNsaWNrSG91c2UgQ2xvdWQiOmZhbHNlLCJDbGlja0hvdXNlIENsb3VkIChBV1MpIjpmYWxzZSwiQ2xpY2tIb3VzZSBDbG91ZCAoR0NQKSI6ZmFsc2UsIkNsaWNrSG91c2UgKHdlYikiOmZhbHNlLCJDcmF0ZURCIjpmYWxzZSwiRGF0YWJlbmQiOmZhbHNlLCJEYXRhRnVzaW9uIChzaW5nbGUgcGFycXVldCkiOmZhbHNlLCJBcGFjaGUgRG9yaXMiOmZhbHNlLCJEcnVpZCI6ZmFsc2UsIkR1Y2tEQiAoUGFycXVldCwgcGFydGl0aW9uZWQpIjpmYWxzZSwiRHVja0RCIjpmYWxzZSwiRWxhc3RpY3NlYXJjaCI6ZmFsc2UsIkVsYXN0aWNzZWFyY2ggKHR1bmVkKSI6ZmFsc2UsIkdyZWVucGx1bSI6ZmFsc2UsIkhlYXZ5QUkiOmZhbHNlLCJIeWRyYSI6ZmFsc2UsIkluZm9icmlnaHQiOmZhbHNlLCJLaW5ldGljYSI6ZmFsc2UsIk1hcmlhREIgQ29sdW1uU3RvcmUiOmZhbHNlLCJNYXJpYURCIjpmYWxzZSwiTW9uZXREQiI6ZmFsc2UsIk1vbmdvREIiOmZhbHNlLCJNeVNRTCAoTXlJU0FNKSI6ZmFsc2UsIk15U1FMIjpmYWxzZSwiUGlub3QiOmZhbHNlLCJQb3N0Z3JlU1FMIjp0cnVlLCJQb3N0Z3JlU1FMICh0dW5lZCkiOnRydWUsIlF1ZXN0REIgKHBhcnRpdGlvbmVkKSI6ZmFsc2UsIlF1ZXN0REIiOmZhbHNlLCJSZWRzaGlmdCI6ZmFsc2UsIlNlbGVjdERCIjpmYWxzZSwiU2luZ2xlU3RvcmUiOmZhbHNlLCJTbm93Zmxha2UiOmZhbHNlLCJTUUxpdGUiOmZhbHNlLCJTdGFyUm9ja3MiOmZhbHNlLCJUaW1lc2NhbGVEQiAoY29tcHJlc3Npb24pIjpmYWxzZSwiVGltZXNjYWxlREIiOmZhbHNlfSwidHlwZSI6eyJzdGF0ZWxlc3MiOnRydWUsIm1hbmFnZWQiOnRydWUsIkphdmEiOnRydWUsImNvbHVtbi1vcmllbnRlZCI6dHJ1ZSwiQysrIjp0cnVlLCJNeVNRTCBjb21wYXRpYmxlIjp0cnVlLCJyb3ctb3JpZW50ZWQiOnRydWUsIkMiOnRydWUsIlBvc3RncmVTUUwgY29tcGF0aWJsZSI6dHJ1ZSwiQ2xpY2tIb3VzZSBkZXJpdmF0aXZlIjp0cnVlLCJlbWJlZGRlZCI6dHJ1ZSwiYXdzIjp0cnVlLCJnY3AiOnRydWUsInNlcnZlcmxlc3MiOnRydWUsIlJ1c3QiOnRydWUsInNlYXJjaCI6dHJ1ZSwiZG9jdW1lbnQiOnRydWUsInRpbWUtc2VyaWVzIjp0cnVlfSwibWFjaGluZSI6eyJzZXJ2ZXJsZXNzIjpmYWxzZSwiMTZhY3UiOmZhbHNlLCJMIjpmYWxzZSwiTSI6ZmFsc2UsIlMiOmZhbHNlLCJYUyI6ZmFsc2UsImM2YS40eGxhcmdlLCA1MDBnYiBncDIiOnRydWUsImM2YS5tZXRhbCwgNTAwZ2IgZ3AyIjpmYWxzZSwiYzUuNHhsYXJnZSwgNTAwZ2IgZ3AyIjpmYWxzZSwiNjAgdGhyZWFkcyAoaWRlYWwpIjpmYWxzZSwiNjAgdGhyZWFkcyAobG9jYWwpIjpmYWxzZSwiMTkyR0IiOmZhbHNlLCIyNEdCIjpmYWxzZSwiMzYwR0IiOmZhbHNlLCI0OEdCIjpmYWxzZSwiNzIwR0IiOmZhbHNlLCI5NkdCIjpmYWxzZSwiNzA4R0IiOmZhbHNlLCJtNWQuMjR4bGFyZ2UiOmZhbHNlLCJtNmkuMzJ4bGFyZ2UiOmZhbHNlLCJjNW4uNHhsYXJnZSwgNTAwZ2IgZ3AyIjpmYWxzZSwiYzZhLjR4bGFyZ2UsIDE1MDBnYiBncDIiOmZhbHNlLCJkYzIuOHhsYXJnZSI6ZmFsc2UsInJhMy4xNnhsYXJnZSI6ZmFsc2UsInJhMy40eGxhcmdlIjpmYWxzZSwicmEzLnhscGx1cyI6ZmFsc2UsIlMyNCI6ZmFsc2UsIlMyIjpmYWxzZSwiMlhMIjpmYWxzZSwiM1hMIjpmYWxzZSwiNFhMIjpmYWxzZSwiWEwiOmZhbHNlfSwiY2x1c3Rlcl9zaXplIjp7IjEiOnRydWUsIjIiOnRydWUsIjQiOnRydWUsIjgiOnRydWUsIjE2Ijp0cnVlLCIzMiI6dHJ1ZSwiNjQiOnRydWUsIjEyOCI6dHJ1ZSwic2VydmVybGVzcyI6dHJ1ZSwidW5kZWZpbmVkIjp0cnVlfSwibWV0cmljIjoiaG90IiwicXVlcmllcyI6W3RydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWUsdHJ1ZSx0cnVlLHRydWVdfQ==)

**Tiered storage**

CockroachDB does not support tiered storage.

### Time series database (e.g. InfluxDB, TimescaleDB)

While a specialized TSDB would handle a lot of the use cases that we're using ClickHouse for, ClickHouse is a much more flexible database. At this stage of a startup, taking on the TCO of multiple databases can be taxing on management and complexity.

### Batch processing (e.g. Spark)

This does not provide realtime anayltics.

### Stream processing (e.g. Flink, Spark Realtime)

While stream processing frameworks achieve many of the goals that we need, we're striving for simplicity. These tools are notoriously difficult to operate. However, providers like Aiven provude managed versions of this software at scale.

ClickHouse also allows us to use the same migration tools (`go-migrate`) for both Cockroach and ClickHouse. Ingesting data in to both are done in similar methods, so our architecutre is kept simple and consistent.

### OpenSearch

> TODO

### Apache Druid

> TODO

### Apache Pinot

> TODO

### Firebolt

### Google BigQuery & Snowflake & Amazon Redshift

These options are managed solutions that can't be self hosted. These are also incredibly expensive compared to using raw hardware to run ClickHouse.

## Shortcomings

> TODO
