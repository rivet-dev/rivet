import { AgentOs } from "@rivet-dev/agent-os-core";
import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";
import { resolve } from "path";

async function main() {
  const vm = await AgentOs.create({ 
    software: [common, pi],
    moduleAccessCwd: resolve(import.meta.dirname, "node_modules/@mariozechner/pi-coding-agent"),
  });

  await vm.writeFile("/tmp/test.cjs", `
    // Check what require.resolve returns
    try {
      var resolved = require.resolve('signal-exit');
      console.log('resolved path:', resolved);
    } catch(e) {
      console.error('RESOLVE FAILED:', e.message);
    }
    
    // Check module.paths
    console.log('module.paths:', JSON.stringify(module.paths));
  `);

  const r1 = await vm.exec("node /tmp/test.cjs");
  console.log("Exit:", r1.exitCode);
  console.log("Out:", r1.stdout);
  console.log("Err:", r1.stderr);

  await vm.dispose();
}

main().catch(e => { console.error(e); process.exit(1); });
