# Docker Configurations

- [server](./server/) is the production-ready Rivet server.
- [client](./client/) is the production-ready Rivet client.
- [monolith](./dev-monolith/) is for running a Rivet server & client to develop your application with in a single container. It can also be used in an external Docker Compose.
- [dev-full](./dev-full/) (Docker Compose) is for testing & developing Rivet itself.

## netrc & GitHub

Rivet depends on cloning a lot of repos. GitHub rate limits these pulls, so we
have to authenticate with a GitHub Token in order to successfully build these
images in GitHub Actions.

To do this, we generate a `.netrc` file with `secrets.GITHUB_TOKEN` (see
`.github/actions/docker-setup`). This file gets mounted as a Docker secret in
the build stage.

It's important that we don't use an `ARG` or `COPY` for the secert nor write it
to the file system, since this might expose the token in the released image. In
theory, all Docker images use a separate runner build stage from the builder
stage so we _could_ write the token to the builder filesystem to simplify
things, but this risk should be avoided in case the wrong image gets pushed or
the wrong artifact ends up in the final image.

