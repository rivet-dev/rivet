# Billing

Deploy your application using the model that best fits your needs. All are billed based on usage.

| Product   | Description                                                      | Ideal Use Cases                                         |
|-------------|------------------------------------------------------------------|---------------------------------------------------------|
| Functions   | Stateless, event-driven code that responds to HTTP requests.     | APIs, webhooks, microservices.                          |
| Containers  | Full control over your runtime environment with Docker containers.| Long-running services, custom environments.             |
| Actors      | Resilient, stateful services that maintain state between requests.| WebSocket servers, game backends, real-time collaboration. |

## Usage Pricing

### Network

| Resource        | Pricing                              | Notes                                                      |
|-----------------|--------------------------------------|------------------------------------------------------------|
| Ingress         | Free                                 | No charges for incoming data.                              |
| Egress          | $0.15 per GB                         | Charges apply to outgoing data.                            |

### Storage

Applies to Actors and Containers.

| Resource        | Pricing                              |
|-----------------|--------------------------------------|
| Save State Reads   | 1M included, then $0.20 per million  |
| Save State Writes  | 1M included, then $1.00 per million  |
| Stored Data     | $0.40 per GB-month                   |

### Actions

Applies to Actor actions and workflow executions.

| Resource        | Pricing                              |
|-----------------|--------------------------------------|
| Actions        | 1M included, then $0.15 per million |

## Sample Scenarios

| Use Case                              | Setup                        | Usage | Monthly Cost |
|----------------------------------------|------------------------------|-------|--------------|
| Storage-heavy app                      | 10GB storage, 5M reads, 1M writes | 10GB × $0.40 = $4.00(5M–1M) reads = $0.80(1M–1M) writes = $0.00 | $4.80       |
| Workflow with Actions                  | 1M actions/month             | 1M actions included = $0.00 | $0.00     |
| High-volume Actions                    | 2M actions/month            | (2M–1M) actions × $0.15/1M = $0.15 | $0.15     |
| High-bandwidth app                     | 1TB egress/month            | 1TB × $0.15 = $150 | $150.00     |

## Contact Sales

For detailed pricing information or to discuss custom requirements, please [contact our sales team](/sales).

## FAQ

### How is usage calculated?
Usage is calculated based on actual resource consumption, including network bandwidth, save state operations, and actions.

### What are Save State Reads and Writes?
Save State operations occur when Actor state is persisted:
- **Save State Reads**: When an Actor wakes up and loads its state from storage
- **Save State Writes**: When Actor state is saved (defaults to every 10 seconds if there are changes, or on `c.saveState()`)

### What are Actions?
Actions are method calls on your Actors (e.g., `await actor.myAction()`). The first 1 million actions per month are included, then $0.15 per million actions.

### Can I upgrade or downgrade my plan?
Yes, you can change your plan at any time. Changes take effect at the start of your next billing cycle.

### Do you offer volume discounts?
Yes, we offer volume discounts for larger deployments. Contact our sales team for more information.

### What is the minimum memory allocation for containers?
You can allocate as little as 128MB of memory for containers.