use rivetkit_client::{Client, EncodingKind, GetOrCreateOptions, TransportKind};
use fs_extra;
use portpicker;
use serde_json::json;
use std::process::{Child, Command};
use std::time::Duration;
use tempfile;
use tokio::time::sleep;
use tracing::{error, info};

/// Manages a mock server process for testing
struct MockServer {
    child: Child,
    // Keep the tempdir alive until this struct is dropped
    _temp_dir: tempfile::TempDir,
}

impl MockServer {
    async fn start(port: u16) -> Self {
        // Get the repo root directory based on current file location
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let repo_root = current_dir
            .ancestors()
            .find(|p| p.join("package.json").exists())
            .expect("Failed to find repo root");

        // Run `pnpm build -F rivetkit` in the root of this repo
        let status = Command::new("yarn")
            .args(["build", "-F", "rivetkit"])
            .current_dir(&repo_root)
            .status()
            .expect("Failed to build rivetkit");

        if !status.success() {
            panic!("Failed to build rivetkit");
        }

        // Create a temporary directory for the test server
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();
        println!("Created temp directory at: {}", temp_path.display());

        // Create vendor directory in the temp dir
        let vendor_dir = temp_path.join("vendor");
        std::fs::create_dir_all(&vendor_dir).expect("Failed to create vendor directory");

        // Define packages to pack
        let packages = [
            ("rivetkit", repo_root.join("packages/rivetkit")),
            ("nodejs", repo_root.join("packages/platforms/nodejs")),
            ("memory", repo_root.join("packages/drivers/memory")),
            ("file-system", repo_root.join("packages/drivers/file-system")),
        ];

        // Pack each package to the vendor directory
        for (name, path) in packages.iter() {
            let output_path = vendor_dir.join(format!("rivetkit-{}.tgz", name));
            println!(
                "Packing {} from {} to {}",
                name,
                path.display(),
                output_path.display()
            );

            let status = Command::new("yarn")
                .args(["pack", "--out", output_path.to_str().unwrap()])
                .current_dir(path)
                .status()
                .expect(&format!("Failed to pack {}", name));

            if !status.success() {
                panic!("Failed to pack {}", name);
            }
        }

        // Copy examples/counter to the temp dir
        let counter_dir = repo_root.join("examples/counter");
        let options = fs_extra::dir::CopyOptions::new();
        fs_extra::dir::copy(&counter_dir, temp_path, &options)
            .expect("Failed to copy counter example");

        // Create the server directory structure
        let server_dir = temp_path.join("counter");
        let server_script_path = server_dir.join("run.ts");

        // Write the server script
        let server_script = r#"
import { app } from "./actors/app.ts";
import { serve } from "@rivetkit/nodejs";

serve(app, { port: PORT, mode: "memory" });
"#
        .replace("PORT", &port.to_string());

        std::fs::write(&server_script_path, server_script).expect("Failed to write server script");

        // Write a new package.json with tarball dependencies
        let package_json_path = server_dir.join("package.json");
        let package_json = format!(
            r#"{{
    "name": "rivetkit-rust-test",
    "packageManager": "yarn@4.2.2",
    "private": true,
    "type": "module",
    "dependencies": {{
        "rivetkit": "file:{}",
        "@rivetkit/nodejs": "file:{}",
        "@rivetkit/memory": "file:{}",
        "@rivetkit/file-system": "file:{}"
    }},
    "devDependencies": {{
        "tsx": "^3.12.7"
    }}
}}"#,
            vendor_dir.join("rivetkit-rivetkit.tgz").display(),
            vendor_dir.join("rivetkit-nodejs.tgz").display(),
            vendor_dir.join("rivetkit-memory.tgz").display(),
            vendor_dir.join("rivetkit-file-system.tgz").display()
        );

        std::fs::write(&package_json_path, package_json).expect("Failed to write package.json");

        // Write a .yarnrc.yml file to use node-modules linker
        let yarnrc_path = server_dir.join(".yarnrc.yml");
        let yarnrc_content = "nodeLinker: node-modules\n";
        std::fs::write(&yarnrc_path, yarnrc_content).expect("Failed to write .yarnrc.yml");
        
        // Install dependencies
        let status = Command::new("yarn")
            .current_dir(&server_dir)
            .status()
            .expect("Failed to install dependencies");

        if !status.success() {
            panic!("Failed to install dependencies");
        }

        // Spawn the server process
        let child = Command::new("npx")
            .args(["tsx", "run.ts"])
            .current_dir(&server_dir)
            .spawn()
            .expect("Failed to spawn server process");

        Self {
            child,
            _temp_dir: temp_dir,
        }
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        // Kill the server process
        if let Err(e) = self.child.kill() {
            error!("Failed to kill server: {}", e);
        }

        // Note: The temporary directory is automatically cleaned up when the tempfile::TempDir
        // value is dropped, which happens when the test finishes

        info!("Mock server terminated");
    }
}

#[tokio::test]
async fn e2e() {
    // Configure logging
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // .with_env_filter(EnvFilter::new("rivetkit_client=trace,hyper=error"))
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    // Pick an available port
    let port = portpicker::pick_unused_port().expect("Failed to pick an unused port");
    info!("Using port {}", port);
    let endpoint = format!("http://localhost:{}", port);

    // Start the mock server
    let _server = MockServer::start(port).await;
    // Wait for server to start
    info!("Waiting for server to start...");
    sleep(Duration::from_secs(2)).await;
    
    // Create the client
    info!("Creating client to endpoint: {}", endpoint);
    let client = Client::new(&endpoint, TransportKind::WebSocket, EncodingKind::Cbor);
    let counter = client.get_or_create("counter", [].into(), GetOrCreateOptions::default())
        .unwrap();
    let conn = counter.connect();

    conn.on_event("newCount", |x| {
        info!("Received newCount event: {:?}", x);
    }).await;
    
    let out = counter.action("increment", vec![json!(1)]).await.unwrap();
    info!("Action 1: {:?}", out);
    let out = conn.action("increment", vec![json!(1)]).await.unwrap();
    info!("Action 2: {:?}", out);
    
    // Clean up
    client.disconnect();
}
