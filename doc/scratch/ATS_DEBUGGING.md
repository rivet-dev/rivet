# ATS Debugging

## Read requests log

```
nomad exec -task server -job ats:local-lcl tail -f /usr/local/var/log/trafficserver/requests.log
```

This file will not exist immediately after startup.

See the [cache results reference](https://docs.trafficserver.apache.org/en/9.1.x/admin-guide/logging/cache-results.en.html).

