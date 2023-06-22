# Feature Flagging

It's common to leave features disabled behind a feature flag. This is common when either:

-   Merging code that isn't ready to deploy ([see pull request goals](/docs/processes/PULL_REQUESTS.md))
-   Enabling niche features
-   Enabling multiple providers (e.g. swappable email provider of Resend vs SendGrid)

## Adding config flags

Add a namespace config flag [here](/lib/bolt/config/src/ns.rs)

Validate the following:

-   Make this does not break the `bolt config gen` command for a fresh config
-   Make sure the flag defaults to disabled (usually)
-   Make sure the Rust config code is documented

## Related resources

-   [Terraform configs & secrets](/docs/infrastructure/terraform/CONFIGS_AND_SECRETS.md)
-   [SaltStack configs & secrets](/docs/infrastructure/saltstack/CONFIGS_AND_SECRETS.md)
