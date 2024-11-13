# Infrastructure as Code (IaC)

Rivet strictly follows the IaC methodology.

## Contributor onboarding

Since everything is completely automated, it makes it incredibly easy to get a contributor or employee set up
with their own independent cluster on their own cloud credentials.

## Upgrading clusters

Infrastructure upgrades can be painful. IaC lets you build & automatically test infrastructure migrations.

For example, we write tests for infrastructure migrations that:

1. Setup the infrastructure at commit A
2. Run the migrations to commit B
3. Run tests on migrated cluster

## Consistency between development & production

We encourage development and production setups to be as similar as possible in order to minimize bugs –
specifically because we're an infrastructure-focused company.

## Isn't this a waste of time as a startup?

Normally, yes. However, Rivet is heavily infrastructure focused, so manually provisioning all the facets that
Rivet requires would be incredibly time consuming, in addition to all the benefits mentioned above.
