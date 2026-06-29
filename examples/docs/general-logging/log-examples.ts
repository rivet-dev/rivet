import { actor } from "rivetkit";

const myActor = actor({
  state: {},
  actions: {
    logExamples: (c) => {
      // Just a message
      c.log.info('server started');
      // Prints: level=INFO actor=myActor key=foo actorId=44096d46632fd087 msg="server started"

      // Object with message and additional fields
      c.log.info({ msg: 'user connected', userId: 123, ip: '192.168.1.1' });
      // Prints: level=INFO actor=myActor key=foo actorId=44096d46632fd087 msg="user connected" userId=123 ip="192.168.1.1"

      // Just an object (no message)
      c.log.info({ action: 'purchase', amount: 99.99, currency: 'USD' });
      // Prints: level=INFO actor=myActor key=foo actorId=44096d46632fd087 action="purchase" amount=99.99 currency="USD"
    }
  }
});
