# region-probe

This service is responsible for auto-registering all available regions with the database.

Upon startup, this service will read the `NOMAD_DC` env var to derive the provider and provider region. Then, it will attempt to insert in to the database if it hasn't been inserted already.

Data such as the display name and universal region is defined in `region-get`.

This service does nothing at runtime.
