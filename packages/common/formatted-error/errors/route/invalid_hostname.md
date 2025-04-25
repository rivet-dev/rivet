---
name = "ROUTE_INVALID_HOSTNAME"
description = "The hostname provided for the route is invalid."
description_basic = "The route hostname format is invalid."
http_status = 400
---

# invalid_hostname

The hostname provided for the route is invalid. Route hostnames must follow a specific format and adhere to DNS standards.

### Details

Route hostnames must meet the following requirements:
- Follow the format `{subdomain}.{domain_job}`
- The subdomain must be at least 4 characters long
- The subdomain must be at most 63 characters long (DNS limitation)
- The entire hostname must be at most 253 characters (DNS limitation)
- The subdomain must contain only lowercase letters, numbers, and hyphens
- The subdomain must not start or end with a hyphen
- The subdomain must not contain consecutive hyphens
- The domain must match the configured domain_job in your environment

### Examples

If your domain_job is `job.example.com`:

Invalid:
- `abc.job.example.com` (subdomain too short)
- `my_app.job.example.com` (invalid character)
- `myapp.other-domain.com` (wrong domain)
- `-myapp.job.example.com` (starts with hyphen)
- `myapp-.job.example.com` (ends with hyphen)
- `my--app.job.example.com` (consecutive hyphens)

Valid:
- `myapp.job.example.com`
- `api-v1.job.example.com`
- `my-service-123.job.example.com`