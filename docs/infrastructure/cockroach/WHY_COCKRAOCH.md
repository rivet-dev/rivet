# Why CockroachDB?

## Motivation

Rivet needs a OLTP to provide the core of our operations.

**Why SQL**

KV & document databases like MongoDB are not an option. Databases that use good old SQL usually have straightforward transaction processing, joins, constraints, schemas, and row locking mechanics that are often well handled in other types of databases.

Adopting an SQL-based database also means that developers don't have to learn the semantics of a new query language and all the pitfalls that comes with it.

SQL is an oldie but a godie.

## Requirements

-   Easy to operate
-   ACID-compliant-ish (distributed SQL databases often aren't truly ACID compliant)
-   Schemas & constraints
-   Secondary indexes

## Alternatives

### Postgres

**Maturity & Governance**

In hindsight, it would have been a good idea to stick with Postgres instead of building on top of CockroachDB, which is a newer and for-profit database.

**Exentions**

While CockroachDB supports common features like JSON, full-text search, and spatial indexes, there are numerous Postgres extensions that will never exists within CockroachDB. For example, we could have used the TimescaleDB extension instead of ClickHouse for our time series workloads. If we chose to support ML workloads, pgvector would be a click away, but instead we would have to adopt a new database.

**Scaling\***

"It doesn't scale" is a common refrain for Postgres critics. Because CockroachDB is a distributed database, it will scale much better that Postgres does. However, a startup that has to scale to that size will also have the means to hire a team of engineers to handle that migration. Velocity is much more important at an early stage, so choosing immature technologies tends to not be a good idea. Luckily, CockroachDB has been preditable and reliable thus far.

**Ease of operation**

Postgres is not a difficult database to operate _but_ CockroachDB has a really powerful web interface that provides monitoring, index insights, and much more. Additionally, adding and removing nodes form CockroachDB is very straightfowrard. The claims of "you can't kill CockroachDB" have held up so far for us.

**GDPR compliance**

CockroachDB supports handling data locality natively while Postgres requires running a separate database for European users.

### FoundationDB

> TODO

### Vitess

> TODO

### YugaByte

> TODO

### Key-value database (e.g. Cassandra)

> TODO

## Shortcomings

-   Managed by a for-profit corporation which could go under or change the license (e.g. ElasticCache)
-   Lack of Postgres-like extensions
-   Lack of UDF
