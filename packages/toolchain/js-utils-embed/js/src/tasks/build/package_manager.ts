import * as fs from 'node:fs';
import * as path from 'node:path';
import { spawn } from 'node:child_process';

/**
 * Recursively searches for a file in the current directory and parent directories
 * @param startDir The directory to start searching from
 * @param fileName The file to search for
 * @returns The path to the file if found, or null if not found
 */
function findFileRecursively(startDir: string, fileName: string): string | null {
    let currentDir = path.resolve(startDir);
    const rootDir = path.parse(currentDir).root;

    while (currentDir !== rootDir) {
        const filePath = path.join(currentDir, fileName);
        if (fs.existsSync(filePath)) {
            return filePath;
        }
        currentDir = path.dirname(currentDir);
    }

    // Check root directory
    const filePath = path.join(rootDir, fileName);
    if (fs.existsSync(filePath)) {
        return filePath;
    }

    return null;
}

/**
 * Determines the preferred package manager based on lockfiles and config files.
 * Recursively searches parent directories for yarn.lock, package-lock.json, pnpm-lock.yaml, bun.lockb, bun.lock, and .npmrc
 * @returns The detected package manager ('yarn', 'npm', 'pnpm', 'bun') or undefined if none found
 */
export function getPreferredPackageManager(projectRoot: string): 'yarn' | 'npm' | 'pnpm' | 'bun' | undefined {
    // Check environment variable override first
    const envPackageManager = process.env._RIVET_PACKAGE_MANAGER?.toLowerCase();
    if (envPackageManager && ['yarn', 'npm', 'pnpm', 'bun'].includes(envPackageManager)) {
        console.log(`Using package manager from environment: ${envPackageManager}`);
        return envPackageManager as 'yarn' | 'npm' | 'pnpm' | 'bun';
    }

    console.log('Detecting preferred package manager...');

    // Recursively search for lockfiles
    const yarnLockPath = findFileRecursively(projectRoot, 'yarn.lock');
    const npmLockPath = findFileRecursively(projectRoot, 'package-lock.json');
    const pnpmLockPath = findFileRecursively(projectRoot, 'pnpm-lock.yaml');
    const bunLockbPath = findFileRecursively(projectRoot, 'bun.lockb');
    const bunLockPath = findFileRecursively(projectRoot, 'bun.lock');

    // Check .npmrc for pnpm configuration (recursively)
    const npmrcPath = findFileRecursively(projectRoot, '.npmrc');
    let hasPnpmConfig = false;
    if (npmrcPath) {
        try {
            const npmrcContent = fs.readFileSync(npmrcPath, 'utf8');
            hasPnpmConfig = npmrcContent.includes('package-manager=pnpm');
        } catch (error) {
            // Ignore read errors
        }
    }

    // Add logging before return statements
    if (yarnLockPath) {
        console.log(`Found yarn.lock at ${yarnLockPath} - using yarn`);
        return 'yarn';
    }
    if (bunLockbPath || bunLockPath) {
        if (bunLockbPath) {
            console.log(`Found bun.lockb at ${bunLockbPath} - using bun`);
        } else {
            console.log(`Found bun.lock at ${bunLockPath} - using bun`);
        }
        return 'bun';
    }
    if (pnpmLockPath || hasPnpmConfig) {
        if (pnpmLockPath) {
            console.log(`Found pnpm-lock.yaml at ${pnpmLockPath} - using pnpm`);
        } else {
            console.log(`Found pnpm configuration in .npmrc at ${npmrcPath} - using pnpm`);
        }
        return 'pnpm';
    }
    if (npmLockPath) {
        console.log(`Found package-lock.json at ${npmLockPath} - using npm`);
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
    const packageManager = getPreferredPackageManager(projectRoot) || 'npm';
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
