import { actor, setup } from "rivetkit";

interface User {
  id: string;
  name: string;
}

interface Order {
  id: string;
  amount: number;
}

interface AuditLog {
  event: string;
  data: User | Order;
  timestamp: number;
}

const userActor = actor({
  state: {},
  actions: {
    createUser: (c, name: string) => {
      const user = { id: crypto.randomUUID(), name };
      c.broadcast("userCreated", user);
      return user;
    }
  }
});

const orderActor = actor({
  state: {},
  actions: {
    completeOrder: (c, amount: number) => {
      const order = { id: crypto.randomUUID(), amount };
      c.broadcast("orderCompleted", order);
      return order;
    }
  }
});

const auditLogActor = actor({
  state: { logs: [] as AuditLog[] },

  actions: {
    startAuditing: async (c) => {
      const client = c.client<typeof registry>();

      // Connect to multiple actors to listen for events
      const userActorConn = client.userActor.getOrCreate(["main"]).connect();
      const orderActorConn = client.orderActor.getOrCreate(["main"]).connect();

      // Listen for user events
      userActorConn.on("userCreated", (user: User) => {
        c.state.logs.push({
          event: "userCreated",
          data: user,
          timestamp: Date.now()
        });
      });

      // Listen for order events
      orderActorConn.on("orderCompleted", (order: Order) => {
        c.state.logs.push({
          event: "orderCompleted",
          data: order,
          timestamp: Date.now()
        });
      });

      return { status: "auditing started" };
    }
  }
});

const registry = setup({ use: { userActor, orderActor, auditLogActor } });
