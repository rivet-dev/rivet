# Sharing and Joining State

Actors store data separately by design, so combining data from multiple actors works differently than traditional databases. This pattern is similar to patterns you'll see in distributed like Cassandra, DynamoDB, and ClickHouse.

Here are your options based on your needs:

## Few actors, need latest data

- Call actions on each actor to get their data
- Actions are fast, so this works well for small numbers of actors
- Trade-off: More work when reading data, but writing is simple

## Many actors, need latest data

- Have actors automatically send updates to other actors that need them
- This keeps everyone in sync without extra calls
- Trade-off: More work when data changes, but reading is fast

## Many actors, okay with slightly old data

- Write data to a separate OLAP database like ClickHouse when updating state
- Query the analytics database for fast joins
- Trade-off: Extra database to maintain, but great performance

## Many actors, need latest data (not recommended)

- Write data to a transactional OLTP database like Postgres or MySQL when updating state
- This works but adds complexity you usually don't need
- Trade-off: Heavy database overhead for minimal benefit