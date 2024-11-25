const { build } = require("esbuild");

void main();

async function main() {
    await bundle({
        platform: "node",
        target: "node14",
        format: "cjs",
        outdir: "node",
    });
    await bundle({
        platform: "browser",
        format: "esm",
        outdir: "browser/esm",
    });
    await bundle({
        platform: "browser",
        format: "cjs",
        outdir: "browser/cjs",
    });
}

async function bundle({ platform, target, format, outdir }) {
    await runEsbuild({
        platform,
        target,
        format,
        entryPoint: "./src/index.ts",
        outfile: `./dist/${outdir}/index.js`,
    });
    await runEsbuild({
        platform,
        target,
        format,
        entryPoint: "./src/core/index.ts",
        outfile: `./dist/${outdir}/core.js`,
    });
    await runEsbuild({
        platform,
        target,
        format,
        entryPoint: "./src/serialization/index.ts",
        outfile: `./dist/${outdir}/serialization.js`,
    });
}

async function runEsbuild({ platform, target, format, entryPoint, outfile }) {
    await build({
        platform,
        target,
        format,
        entryPoints: [entryPoint],
        outfile,
        bundle: true,
        alias: {
            // matches up with tsconfig paths
            "@rivet-gg/api": "./src",
        },
		external: ['node-fetch'],
    }).catch(() => process.exit(1));
}
