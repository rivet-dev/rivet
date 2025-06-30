# Download cadvisor binary
#
# v0.38
echo 'Installing cadvisor'
apt install -y cadvisor

# Disable the default cadvisor service
systemctl stop cadvisor || true
systemctl disable cadvisor || true

# Create new service file with updated flags
#
# See all metrics: https://github.com/google/cadvisor/blob/95fd3aff63650e37c3825c58fbf4db85f68b8c26/container/factory.go#L45
#
# See also packages/edge/infra/client/config/src/utils.rs for cgroup prefix
cat > /lib/systemd/system/rivet-cadvisor.service << 'EOF'
[Unit]
Description=cAdvisor
Documentation=man:cadvisor
Documentation=https://github.com/google/cadvisor
After=docker.service containerd.service

[Service]
EnvironmentFile=/etc/default/cadvisor
ExecStart=/usr/bin/cadvisor \
    --port=7780 \
    --listen_ip=0.0.0.0 \
    --prometheus_endpoint="/metrics" \
    --disable_metrics=memory_numa,disk,advtcp,accelerator,hugetlb,referenced_memory,resctrl \
    --docker_only=false \
    --raw_cgroup_prefix_whitelist=/system.slice/pegboard-actor-

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload

# Enable and start the new rivet-cadvisor service
systemctl enable rivet-cadvisor
systemctl start rivet-cadvisor

