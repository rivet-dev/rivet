# [rivet.run](http://rivet.run) DNS & TLS Configuration

## Moving parts

#### TLS Cert

-   Can only have 1 wildcard
    -   i.e. `*.lobby.{dc_id}.rivet.run`
-   Takes a long time to issue
-   Prone to Lets Encrypt downtime and [rate limits](https://letsencrypt.org/docs/rate-limits/)
    -   Nathan requested a rate limit increase for when this is needed

#### DNS record

-   Must point to the IP of the datacenter we need
    -   i.e. `*.lobby.{dc_id}.rivet.run` goes to the GG Node for the given datacenter
    -   `*.rivet.run` will not work as a static DNS record because you can’t point it at a single datacenter

#### GG host resolution

-   When a request hits the GG server for HTTP(S) or TCP+TLS requests, we need to be able to resolve the lobby to send it to
-   This is why the lobby ID Needs to be in the DNS name

#### GG autoscaling

-   The IPs that the DNS records point to change frequently as GG nodes scale up and down

## Design

#### DNS records

Dynamically create a DNS record for each GG node formatted like `*.lobby.{dc_id}.rivet.run`. Example:

```bash
A *.lobby.51f3d45e-693f-4470-b86d-66980edd87ec.rivet.run 1.2.3.4	# DC foo, GG node 1
A *.lobby.51f3d45e-693f-4470-b86d-66980edd87ec.rivet.run 5.6.7.8	# DC foo, GG node 2
A *.lobby.51f3d45e-693f-4470-b86d-66980edd87ec.rivet.run 9.10.11.12	# DC bar, GG node 1
```

These the IPs of these records change as the GG nodes scale up and down, but the origin stays the same.

#### TLS certs

Each datacenter needs a TLS cert. For the example above, we need a TLS cert for `*.lobby.51f3d45e-693f-4470-b86d-66980edd87ec.rivet.run` and `*.lobby.51f3d45e-693f-4470-b86d-66980edd87ec.rivet.run`.

## TLS

#### TLS cert provider

Currently we use Lets Encrypt as our TLS certificate provider.

Alternatives:

-   ZeroSSL

#### TLS cert refreshing

Right now, the TLS certs are issued in the Terraform plan. Eventually, TLS certs should renew on their own automatically.

## TLS Alternatives

#### Use `*.rivet.run` TLS cert with custom DNS server

Create a `NS` record for `*.rivet.run` pointed at our custom DNS server

We can use a single static TLS cert
