# Features

Features are a way for us to configure which services run on what clusters. This
makes it very easy for us to run the the minimum amount of services required on
our `job-run` clusters with minimal hardware while running the full platform in
`platform` clusters.

Features are defined in `Service.toml` under `service.feature`. Only main
services need to define the features, since all of their dependencies will
automatically be included in the given deployment. This usually means this only
needs to be specified on api, periodic, and batch jobs. Worker jobs are almost
always included as a dependency of another job.

At the moment, we run everything under the `platform` feature since providing
region failover for all `job-run` regions is very expensive.
