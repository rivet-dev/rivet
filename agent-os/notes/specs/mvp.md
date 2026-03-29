```
import { AgentOs } from "@rivet-dev/agent-os";

const agent = new AgentOs(...)

// Sessions
await myvm.createSession("pi");
await myvm.sendPrompt("foobar");
myvm.on("message", message => {
	console.log("message");
});

// Commands
await myvm.execute("ls", ["/"]);
// await myvm.execute("appstore", ["install", "pi"]);
// Networkign
await myvm.fetch(port, request);

// File system
// TODO:

// TODO: re-export everythign from the kernel

// Sessions
await myvm.createSession("pi");
await myvm.sendPrompt("foobar");
myvm.onSessionEvent(message => {
	console.log("message");
});

```

Above I attach a code snippet. We're setting up a new project from scratch. I'm going to give you a bunch of descriptions of what it is and then I want you to update the README with anything in the claude.md with specific constraints that we talked about.
This is a project called agentOS. This project is focused on taking a package called Secure-Exec. You can find that in my home folder; it's a package called secure-exec-1. That's my home folder that includes something called an operating system, which includes a full kernel. This operating system is a JavaScript kernel that can operate as a full operating system using WebAssembly as binaries and then also plugs in with a sandbox version of Node.js with our custom runtime for Node.js-based operations.
Here's the deal: we're going to be using that. We're going to be wrapping that with a cleaner API in agentOS. The idea is that we are going to basically have two huge phases to this project:
1. The first phase is just implementing the basic functionality, like XU command, fetch, the networking, the file system, and those super basic things that I discussed. The idea is that we should do this first phase where we can test end-to-end with integration tests that we can create agentOS and then execute with those commands and everything works as intended. In terms of testing the network, you're going to need, in your task, to actually use the `node` command to spawn a network server in the background and then execute against that.
2. The second phase is going to be related to agents. The PI coding agent works inside of the Secure-Exec OS and inside of agentOS. What we're going to be doing is going to be creating a nice API to manage PI sessions. It will actually be any type of coding agent; we're going to start with just PI. What's going to happen is we're using a protocol called ACP, the Agent Communication Protocol, to create a universal wrapper over PI. There's another project that has already implemented the ACP API, another project called Sandbox Agent. That project is in my home folder, called sandbox-agent. We're not going to be using any code from sandbox-agent but you need to reference it for how ACP works under the hood and make sure that you put this in the claude.md that we should use this for reference for ACP's integration.
Okay and then now that we have, in terms of architecture, when you call `create session`, it's going to spawn the ACP adapter, which is a separate npm package from PI itself that then, under the hood, spawns PI. To be clear, `create session` spawns an ACP adapter session, which then spawns PI. You can see how we do this in the sandbox-agent project. Today we're just going to be reproducing that pattern but inside of the agentOS. This needs to be tested at each layer:
1. You need to test that it can actually spawn PI in headless mode and you can send commands to it. That's part one.
2. You need to test that you can manually spawn the ACP adapter for PI and that you can send commands to and from it.
3. You need to test that the whole `create session` high-level API works.
Each one of those needs to be tested sequentially.
I want to go do some research on Secure-Exec and sandbox-agent as I discussed and then come back with any questions. Once the questions are done we're going to rate a spec for this but let's start with just the questions.

