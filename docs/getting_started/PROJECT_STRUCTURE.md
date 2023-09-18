## Project Structure

**🗄️ Configuration**

```
namespaces/                      Configuration for your Rivet cluster
secrets/                         Credentials & secrets
```

**⚙️ Services**

```
svc/                       Source code for services
  api/                     Public-facing API services
  pkg/                     Internal business logic
    my-pkg/
      db/                  Database migrations
      ops/                 Operations used within other packages
      worker/              Chirp worker
      standalone/          CRON, headless, and oneoff services
      types/               Package-specific Protobuf types
lib/                             Internal libraries
fern/                            API type definitions (buildwithfern.com)
proto/                           Protobuf types (protobuf.dev)
```

**💻 Development**

```
CHANGELOG.md                     keepachangelog.com
shell.nix                        Nix shell configuration (provides dev tools)
doc/                             Internal documentation
errors/                          Documentation of all error types used in Rivet
templates/                       Templates used for bootstrapping services
tests/                           Misc system tests
```

**🏗️ Infrastructure**

```
infra/
  tf/                            Terraform state files
  docker/                        Docker images required for Rivet's infra
  tests/                         Infra tests
  nix/                           Shared Nix configs
```
