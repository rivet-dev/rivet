# Router Priorities

| Priority | Router                    | Notes                                                                    |
| -------- | ------------------------- | ------------------------------------------------------------------------ |
| 50       | api-monolith              | Lives at the root, so anything that wants a path needs a higher priority |
| 51       | _Other Bolt API services_ |                                                                          |
| 60       | Media fallback (imagor)   | Anything without a query will route here                                 |
| 61       | Media with config         | Anything with a query will route here                                    |
