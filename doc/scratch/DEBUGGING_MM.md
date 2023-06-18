# Debugging Matchmaker
## Monitor
```
bolt monitor
```

## Subscribe to NATS messages

```
bolt deploy forward staging +nats -p 8085
export NATS_URL=127.0.0.1:8085
export NATS_USER=admin
// or
export NATS_USER=bolt
export NATS_PASSWORD=password
```

```
# TODO: This doesn't work for some reason
nats sub 'chirp.msg.>' | grep -a 'Received on' | grep -a 'msg-mm-'
```

```
nats sub 'JOB_RUN.>' | grep -a 'Received on'
```

## Look at `RVT-RAY-ID` response header
Copy the ray ID from the find response and past it in to https://rivet.gg/admin.

