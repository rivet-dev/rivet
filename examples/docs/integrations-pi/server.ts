import { agentOS, setup } from "@rivet-dev/agentos";
import { pi } from "@rivet-dev/pi";
import { agentOSSandbox } from "@rivet-dev/sandbox-adapter";

const mySandbox = agentOS();

const myAgent = pi({
	sandbox: agentOSSandbox({ actor: "mySandbox" }),
	actions: {
		ping: () => "pong",
	},
});

export const registry = setup({
	use: { myAgent, mySandbox },
});

registry.start();
