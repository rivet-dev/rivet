# Troubleshooting

## `Missing columns: 'xxxx' while processing query` in JSON

In a query like this, where `properties` is `JSON`:

```
> SELECT properties.abc FROM events
Code: 47. DB::Exception: Received from clickhouse.clickhouse.svc.cluster.local:9440. DB::Exception: Missing columns: 'properties.abc' while processing query: 'SELECT properties.abc FROM events', required columns: 'properties.abc', maybe you meant: 'properties'. (UNKNOWN_IDENTIFIER)
```

This is because the `JSON` type is semi-structured and stored as a tuple under the hood. If there is not a row with the given JSON property, then the tuple element will not exist.

In order to optionally read a tuple element, you need to use `tupleElement`, like this (where `''` is the fallback value):

```
SELECT tupleElement(properties, 'abc', '') FROM events
```
