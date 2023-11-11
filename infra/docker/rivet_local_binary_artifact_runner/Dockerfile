FROM debian:12.1-slim

# Update ca-certificates.
RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && apt-get install -y --no-install-recommends ca-certificates

# Copy supporting scripts
COPY ./infra/misc/svc_scripts/health_check.sh ./infra/misc/svc_scripts/install_ca.sh /usr/bin/
RUN chmod +x /usr/bin/health_check.sh /usr/bin/install_ca.sh

