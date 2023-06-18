# Nebula

## What is Nebula?

Nebula is similar to a VPN. It gives each server a private IP & network interface that automatically encrypts traffic between other servers. It allows allows for fine-grained firewalls to be defined between servers internally to improve security.

Nebula is different from a VPN since Nebula does not have a central server that all traffic has to pass through and instead uses [NAT hole punching](https://docs.github.com/en/enterprise-server@3.4/authentication/keeping-your-account-and-data-secure/creating-a-personal-access-token).

## What do we use Nebula for?

All internal traffic running through Rivet uses Nebula.

Nebula is specifically helpful for securing traffic across multiple dataceners and clouds. Because Rivet is built to run across as many types of environments as possible, Nebula provides a consistent way of addressing our nodes.

We also use it to define firewall rules between our pools to mitigate the impact of a server with malicious code running on it.


