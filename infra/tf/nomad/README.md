# Nomad Terraform Plan

These are the core services that run on top of Nomad.

Only stateless services that allow for changing IP addresses should be ran on Nomad. For all other services, make a seperate pool and provision with SaltStack.

