# API Timeouts

Many load balancers have 60s configured as default timeout. Our API timeouts are designed to work within these bounds.

## Long polling

We use long polling (i.e. `watch_index`) to implement real time functionality. This means we need to be cautious about existing timeouts.

## Timeouts

Current timeouts:

-   Cloudflare: 100s ([source](https://developers.cloudflare.com/support/troubleshooting/cloudflare-errors/troubleshooting-cloudflare-5xx-errors/#error-524-a-timeout-occurred))
    -   **Behavior** Returns a 524
    -   Cannot be configured unless paying for Cloudflare Enterprise
-   Traefik: 60s ([source](https://github.com/rivet-gg/rivet/blob/c63067ce6e81f97b435e424e576fbd922b14f748/infra/tf/k8s_infra/traefik.tf#L65))
    -   **Motivation** `api-helper` should always handle this error if everything is functioning correctly. This is meant to be less than Cloudflare to be able to show a Traefik-specific response.
-   `api-helper`: 50s ([source](https://github.com/rivet-gg/rivet/blob/9811ae11656d63e26b4814fe15f7f852f5479a48/lib/api-helper/macros/src/lib.rs#L975))
    -   **Behavior** Returns `API_REQUEST_TIMEOUT`
    -   **Motivation** This gives a 10s budget for any other 60s timeout
-   `select_with_timeout!`: 40s ([source](https://github.com/rivet-gg/rivet/blob/9811ae11656d63e26b4814fe15f7f852f5479a48/lib/util/macros/src/lib.rs#L12))
    -   **Behavior** Timeout handled by API endpoint, usually 200
    -   **Motivation** This gives a 10s budget for any requests before/after the select statement
