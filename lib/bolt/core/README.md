# Bolt

_An opinionated distributed backend management system._

## Installation

```
$ cargo install --path .
```

## Templates

Templates should be as minimal as possible with as much generation left up to Bolt as possible. There should never be a scenario where you have to "rewrite" a service based on an updated version of a template, just because you changed one small parameter.

## Naming

Names should use kebab case. Bolt and other tools will automatically convert the name to snake case in places where appropriatey.

## Protobuf Imports

The original plan for schemas was to have two modules: the schema module and the service module. The schema module could be published an other modules could import it in order to interface with the service. However, that presents a few problems. Instead we import the Protobuf files from other services directly in the .proto files. This is done for a few reasons:

1. Microservices should be designed to be as language-agnostic as possible.
    - By importing schemas for other services in the Protobuf layer, we can really easily interface between services no matter the language.
2. Simplifies dependency management & Docker image building.
    - Traditionally when building a Docker project in monorepos, you have to do a little dance to selectively import parts of the monorepo in order to retain build caching. Caching schemas is not a big concern, so dumping all of this in the local schema module is easier.
3. Some data is not "owned" by a service.
    - Common data, such as user profiles and events dispatched by other services, are not owned by any specific service and are better imported by the Protobuf file.

The main downside to doing this is that a lot of extraneous code gets generated in the schema module.

## Generated code

Generated code gets committed to Git so the developer is completely aware of any changes that will be committed to the cluster. These files will be validated on the CI server with `generate validate` in order to make sure that they are not accidentally modified by the dev before uploading to the cluster.
