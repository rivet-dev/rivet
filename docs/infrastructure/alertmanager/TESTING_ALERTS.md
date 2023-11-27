# Testing Alerts

## Manually

Create a test alert by running in terminal 1:

```
PORT=9093 ./scripts/forward/alertmanager.sh
```

And terminal 2:

```
curl -X POST -H "Content-Type: application/json" -d '[{"labels": {"alertname":"TestAlert2","severity":"critical"}}]' http://localhost:9093/api/v1/alerts
```

