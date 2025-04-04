// This script is used to fetch the actor-core package from the monorepo
// and extract it to the temp directory. It is used by the actor-core
// package to avoid having to install the entire monorepo in the temp
// directory. This is useful for testing and development purposes.
// @ts-check
/// <reference types="node" />
/// <reference types="@yarnpkg/plugin-exec" />

const { join } = require('node:path');
const childProcess = require("node:child_process");
const fs = require("node:fs");
/**
 * @typedef {import('@yarnpkg/plugin-exec').ExecEnv} ExecEnv
 */
const { execEnv } = /** @type {{execEnv: ExecEnv} & typeof globalThis} */ (globalThis);

resolve("actor-core");

/**
 * 
 * @param {string} module - The path to the module (i.e "actor-core" or "platforms/nodejs")
 */
function resolve(module) {
    const pathToRepo = join(execEnv.tempDir, 'repo');
    const pathToArchive = join(execEnv.tempDir, 'archive.tgz');
    const pathToSubpackage = join(pathToRepo, 'packages', module);
    const pathToLocalRepository = process.env.ACTOR_CORE_REPO || join(execEnv.tempDir, 'actor-core');
    const pathToLocalSubpackage = join(pathToLocalRepository, 'packages', module);

    // Check if the local repository exists
    if (fs.existsSync(pathToLocalRepository) && fs.existsSync(pathToLocalSubpackage)) {
        console.log(`Using local repository: ${pathToLocalRepository}`);
        // If it exists, use it instead of cloning the repository
        childProcess.execFileSync(`yarn`, [`pack`, `--out`, pathToArchive], {cwd: pathToLocalSubpackage});

        // Send the package content into the build directory
        childProcess.execFileSync(`tar`, [`-x`, `-z`, `--strip-components=1`, `-f`, pathToArchive, `-C`, execEnv.buildDir]);
    } else {
        console.log(`Cloning repository: ${pathToRepo}`);
        // Clone the repository
        childProcess.execFileSync(`git`, [`clone`,'--single-branch', '--branch', '03-26-feat_add_inspector_to_manager', `git@github.com:rivet-gg/actor-core.git`, pathToRepo]);

        // Install the dependencies
        childProcess.execFileSync(`yarn`, [`install`], {cwd: pathToRepo});

        // Build the package
        childProcess.execFileSync(`yarn`, [`build`], {cwd: pathToSubpackage});

        // Pack a specific workspace
        childProcess.execFileSync(`yarn`, [`pack`, `--out`, pathToArchive], {cwd: pathToSubpackage});

        // Send the package content into the build directory
        childProcess.execFileSync(`tar`, [`-x`, `-z`, `--strip-components=1`, `-f`, pathToArchive, `-C`, execEnv.buildDir]);
    }
}