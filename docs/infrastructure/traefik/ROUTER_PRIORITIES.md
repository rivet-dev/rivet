# Router Priorities

| Priority | Router                  | Notes                                    |
| -------- | ----------------------- | ---------------------------------------- |
| 50       | _Bolt API services_     |                                          |
| 60       | Media fallback (imagor) | Anything without a query will route here |
| 61       | Media with config       | Anything with a query will route here    |
