job "runc-example" {
  datacenters = ["lnd-atl"]

  group "game" {
    count = 1

	restart {
		attempts = 0
		mode = "fail"
	}

    task "runc-task" {
      driver = "raw_exec"

      config {
        command = "/bin/bash"
        args = [
          "-euf", "-c",
          <<EOF
echo "$(pwd)"

# BEGIN custom loader
echo '=== Convert ==='
time skopeo copy "docker-archive:local/docker-image/image.tar" "oci:local/oci-image:default"

echo '=== Install ==='

echo '=== Unpack ==='
curl -Lf -o umoci 'https://github.com/opencontainers/umoci/releases/download/v0.4.7/umoci.amd64'
chmod +x umoci
time ./umoci unpack --image "local/oci-image:default" "local/oci-bundle/"
# END custom loader

# Copy the Docker-specific values from the OCI bundle config.json to the base config
#
# This way, we enforce our own capabilities on the container
override_config="local/oci-bundle-config.overrides.json"
mv "local/oci-bundle/config.json" "$override_config"
jq "
.process.args = $(jq '.process.args' $override_config) |
.process.env = $(jq '.process.env' $override_config) |
.process.user = $(jq '.process.user' $override_config) |
.process.cwd = $(jq '.process.cwd' $override_config)
" local/oci-bundle-config.base.json > local/oci-bundle/config.json

echo '=== Finished ==='
# TODO: Generate container name
(cd local/oci-bundle && /usr/bin/runc run rivet-job)

EOF
        ]
      }
      
      artifact {
        source = "http://10.0.0.65:8080/s3-cache/aws/test24-bucket-build/1b578337-afdd-4b09-88da-6fad732b5448/image.tar"
        destination = "local/docker-image"
        options {
          archive = false
        }
      }

      template {
		  destination = "local/oci-bundle-config.base.json"
		  data = <<EOF
{
        "ociVersion": "1.0.2-dev",
        "process": {
                "terminal": false,
                "user": {
                        "uid": 0,
                        "gid": 0
                },
                "args": [
                        "sh"
                ],
                "env": [
                        "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
                        "TERM=xterm"
                ],
                "cwd": "/",
                "capabilities": {
                        "bounding": [
                                "CAP_AUDIT_WRITE",
                                "CAP_KILL",
                                "CAP_NET_BIND_SERVICE"
                        ],
                        "effective": [
                                "CAP_AUDIT_WRITE",
                                "CAP_KILL",
                                "CAP_NET_BIND_SERVICE"
                        ],
                        "permitted": [
                                "CAP_AUDIT_WRITE",
                                "CAP_KILL",
                                "CAP_NET_BIND_SERVICE"
                        ],
                        "ambient": [
                                "CAP_AUDIT_WRITE",
                                "CAP_KILL",
                                "CAP_NET_BIND_SERVICE"
                        ]
                },
                "rlimits": [
                        {
                                "type": "RLIMIT_NOFILE",
                                "hard": 1024,
                                "soft": 1024
                        }
                ],
                "noNewPrivileges": true
        },
        "root": {
                "path": "rootfs",
                "readonly": true
        },
        "hostname": "runc",
        "mounts": [
                {
                        "destination": "/proc",
                        "type": "proc",
                        "source": "proc"
                },
                {
                        "destination": "/dev",
                        "type": "tmpfs",
                        "source": "tmpfs",
                        "options": [
                                "nosuid",
                                "strictatime",
                                "mode=755",
                                "size=65536k"
                        ]
                },
                {
                        "destination": "/dev/pts",
                        "type": "devpts",
                        "source": "devpts",
                        "options": [
                                "nosuid",
                                "noexec",
                                "newinstance",
                                "ptmxmode=0666",
                                "mode=0620",
                                "gid=5"
                        ]
                },
                {
                        "destination": "/dev/shm",
                        "type": "tmpfs",
                        "source": "shm",
                        "options": [
                                "nosuid",
                                "noexec",
                                "nodev",
                                "mode=1777",
                                "size=65536k"
                        ]
                },
                {
                        "destination": "/dev/mqueue",
                        "type": "mqueue",
                        "source": "mqueue",
                        "options": [
                                "nosuid",
                                "noexec",
                                "nodev"
                        ]
                },
                {
                        "destination": "/sys",
                        "type": "sysfs",
                        "source": "sysfs",
                        "options": [
                                "nosuid",
                                "noexec",
                                "nodev",
                                "ro"
                        ]
                },
                {
                        "destination": "/sys/fs/cgroup",
                        "type": "cgroup",
                        "source": "cgroup",
                        "options": [
                                "nosuid",
                                "noexec",
                                "nodev",
                                "relatime",
                                "ro"
                        ]
                }
        ],
        "linux": {
                "resources": {
                        "devices": [
                                {
                                        "allow": false,
                                        "access": "rwm"
                                }
                        ]
                },
                "namespaces": [
                        {
                                "type": "pid"
                        },
                        {
                                "type": "network"
                        },
                        {
                                "type": "ipc"
                        },
                        {
                                "type": "uts"
                        },
                        {
                                "type": "mount"
                        },
                        {
                                "type": "cgroup"
                        }
                ],
                "maskedPaths": [
                        "/proc/acpi",
                        "/proc/asound",
                        "/proc/kcore",
                        "/proc/keys",
                        "/proc/latency_stats",
                        "/proc/timer_list",
                        "/proc/timer_stats",
                        "/proc/sched_debug",
                        "/sys/firmware",
                        "/proc/scsi"
                ],
                "readonlyPaths": [
                        "/proc/bus",
                        "/proc/fs",
                        "/proc/irq",
                        "/proc/sys",
                        "/proc/sysrq-trigger"
                ]
        }
}
EOF
      }

      resources {
        cpu    = 1000
        memory = 1024
      }
    }
  }
}

