import * as fs from 'node:fs';
import * as path from 'node:path';
import { spawn } from 'node:child_process';

/**
 * Determines the preferred package manager based on lockfiles and config files.
 * Checks for yarn.lock, package-lock.json, pnpm-lock.yaml, and .npmrc
 * @returns The detected package manager ('yarn', 'npm', 'pnpm') or undefined if none found
 */
export function getPreferredPackageManager(projectRoot: string): 'yarn' | 'npm' | 'pnpm' | undefined {
    // Check environment variable override first
    const envPackageManager = process.env._RIVET_PACKAGE_MANAGER?.toLowerCase();
    if (envPackageManager && ['yarn', 'npm', 'pnpm'].includes(envPackageManager)) {
        console.log(`Using package manager from environment: ${envPackageManager}`);
        return envPackageManager as 'yarn' | 'npm' | 'pnpm';
    }

    console.log('Detecting preferred package manager...');

    // Check for lockfiles in the current directory
    const hasYarnLock = fs.existsSync(path.join(projectRoot, 'yarn.lock'));
    const hasNpmLock = fs.existsSync(path.join(projectRoot, 'package-lock.json'));
    const hasPnpmLock = fs.existsSync(path.join(projectRoot, 'pnpm-lock.yaml'));

    // Check .npmrc for pnpm configuration
    const npmrcPath = path.join(projectRoot, '.npmrc');
    let hasPnpmConfig = false;
    if (fs.existsSync(npmrcPath)) {
        try {
            const npmrcContent = fs.readFileSync(npmrcPath, 'utf8');
            hasPnpmConfig = npmrcContent.includes('package-manager=pnpm');
        } catch (error) {
            // Ignore read errors
        }
    }

    // Add logging before return statements
    if (hasYarnLock) {
        console.log('Found yarn.lock - using yarn');
        return 'yarn';
    }
    if (hasPnpmLock || hasPnpmConfig) {
        console.log('Found pnpm configuration - using pnpm');
        return 'pnpm';
    }
    if (hasNpmLock) {
        console.log('Found package-lock.json - using npm');
        return 'npm';
    }

    console.log('No package manager configuration found');
    return undefined;
}

/**
 * Checks if a package manager is installed by attempting to run its version command
 * @param packageManager The package manager to check
 * @returns Promise<boolean> indicating if the package manager is installed
 */
async function isPackageManagerInstalled(packageManager: string): Promise<boolean> {
    console.log(`Checking if ${packageManager} is installed...`);
    
    return new Promise((resolve) => {
        const process = spawn(packageManager, ['--version'], {
            stdio: 'ignore',
            shell: true
        });

        process.on('close', (code: number) => {
            resolve(code === 0);
        });

        process.on('error', () => {
            resolve(false);
        });
    });
}

/**
 * Runs a command using the preferred package manager
 * @param command The command to run (e.g. 'install', 'add', etc.)
 * @param args Additional arguments for the command
 * @returns A promise that resolves when the command completes
 */
export async function runPackageManagerCommand(projectRoot: string, command: string, ...args: string[]): Promise<void> {
    const packageManager = getPreferredPackageManager(projectRoot) || 'yarn';
    console.log(`Selected package manager: ${packageManager}`);
    
    // Check environment variable override for Deno usage
    const forceUseDeno = process.env._RIVET_DENO_RUN_PACKAGE_MANGER?.toLowerCase() === 'true';
    const isUsingDeno = forceUseDeno || !(await isPackageManagerInstalled(packageManager));
    console.log(`Using Deno: ${isUsingDeno}`);

    const commandArray = isUsingDeno 
        ? [Deno.execPath(), 'run', '-A', '--node-modules-dir=auto', `npm:${packageManager}`, command, ...args]
        : [packageManager, command, ...args];
    
    console.log(`Executing command: ${commandArray.join(' ')}`);

    return new Promise((resolve, reject) => {
        const process = spawn(commandArray[0], commandArray.slice(1), {
            stdio: 'inherit',
            shell: true,
            cwd: projectRoot,
        });

        process.on('close', (code: number) => {
            if (code === 0) {
                console.log('Command completed successfully');
                resolve();
            } else {
                console.error(`Command failed with exit code ${code}`);
                reject(new Error(`Command failed with exit code ${code}`));
            }
        });

        process.on('error', (err: Error) => {
            console.error('Command execution error:', err);
            reject(err);
        });
    });
}
