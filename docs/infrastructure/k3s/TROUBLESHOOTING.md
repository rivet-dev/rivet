# Troubleshooting

## Check what ports are forwarded to my load balancer?

Check the ports forwarded with this command:

```
docker ps | grep k3d
```

## I'm getting `Empty reply from server`

This means Traefik hasn't started yet. Make sure the deployment works.
