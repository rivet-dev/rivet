# Gotchas

## Provider Billing Intervals

Server providers bill for usage based on different billing cycles. It is important to know when scaling
because if the cycle is too long, autoscaling quickly will end up wasting money due to unused billing time.

### Linode

Linode bills hourly. Servers running for 1 minute, 59 minutes, or 60 minutes all cost the user the same
amount. Internally, we do not immediately delete servers when scaling down and instead start draining them.
This means if we have to scale up again within an hour we can undrain an existing server instead of
provisioning a new one, saving the user money.

When using Linode, you should choose your drain timeout to be close to (but not over) intervals of an hour
(3_600_000 ms).
