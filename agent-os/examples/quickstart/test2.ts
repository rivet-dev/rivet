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
    var result = require('signal-exit');
    console.log('type:', typeof result);
    console.log('keys:', Object.keys(result).join(', '));
    console.log('hasDefault:', 'default' in result);
    if (result.default) {
      console.log('default type:', typeof result.default);
      console.log('default value:', String(result.default).slice(0, 100));
    }
    
    // Check if this is an ESM namespace object
    console.log('Symbol.toStringTag:', result[Symbol.toStringTag]);
    console.log('__esModule:', result.__esModule);
  `);

  const r1 = await vm.exec("node /tmp/test.cjs");
  console.log("Exit:", r1.exitCode);
  console.log("Out:", r1.stdout);
  console.log("Err:", r1.stderr);

  await vm.dispose();
}

main().catch(e => { console.error(e); process.exit(1); });
