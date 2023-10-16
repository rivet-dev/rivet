job "runc-example" {
  datacenters = ["lnd-atl"]

  group "game" {
    count = 1

    task "runc-task" {
      driver = "raw_exec"

      config {
        command = "/bin/bash"
        args = [
          "-euf", "-c",
          <<EOF
echo "pwd $(pwd)"

# MARK: Begin custom loader
echo '=== Convert ==='
time skopeo copy "docker-archive:local/docker-image/image.tar" "oci:local/oci-image:default"

echo '=== Install ==='

echo '=== Unpack ==='
curl -Lf -o umoci 'https://github.com/opencontainers/umoci/releases/download/v0.4.7/umoci.amd64'
chmod +x umoci
time ./umoci unpack --image "local/oci-image:default" "local/oci-bundle/"
# MARK: End custom loader

# TODO: Write custom config.json borrowing properties from default config.json

# See default generator for containerd: https://github.com/containerd/containerd/blob/main/oci/spec.go
jq '.process.terminal = false' local/oci-bundle/config.json > local/config-tmp.json
rm local/oci-bundle/config.json
mv local/config-tmp.json local/oci-bundle/config.json

# sleep infinity

echo '=== Finished ==='
# TODO: Generate container name
(cd local/oci-bundle && /usr/bin/runc run rivet-job)
# /usr/bin/runc --root local/oci-bundle run my-container
o
EOF
        ]
      }
      
      artifact {
        source = "http://10.0.0.65:8080/s3-cache/aws/test24-bucket-build/1b578337-afdd-4b09-88da-6fad732b5448/image.tar"
        destination = "local/docker-image"
        options {
          # archive = "tar"
          archive = false
        }
      }

      resources {
        cpu    = 1000
        memory = 1024
      }
    }
  }
}

