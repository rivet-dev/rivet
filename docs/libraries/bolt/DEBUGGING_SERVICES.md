# Debugging Services

## Reading logs

Use the `bolt logs` command to read a service's logs. See `bolt logs --help` for more options.

Remember that you can only read the logs of an executable (i.e. API services, workers, and standalone services), not libraries (i.e. operations).

## Metrics

Rivet exposes Prometheus metrics for as many parameters as possible.

If you have Cloudflare Access configured, visit `https://prometheus-svc.MAIN_DOMAIN` where `MAIN_DOMAIN` is the value of `dns.domain.main`. From here, you can query metrics from all of the infrastructure & from Rivet services.
