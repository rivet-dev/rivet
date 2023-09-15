apt-get update -y
apt-get install -y build-essential pkg-config libtool gcc make openssl libpcre3-dev libcap2-bin flex hwloc libncurses5-dev curl


# Download source
cd /tmp
wget https://archive.apache.org/dist/trafficserver/trafficserver-9.0.1.tar.bz2
tar -xvf trafficserver-9.0.1.tar.bz2
cd trafficserver-9.0.1

# Build and install
./configure --prefix=/usr/local
make
make install

cat << 'EOL' > /etc/systemd/system/trafficserver.service
[Unit]
Description=Apache Traffic Server
After=network-online.target

[Service]
Type=simple
User=trafficserver
Group=trafficserver
Environment="TM_START=yes"
PIDFile=/run/trafficserver/manager.lock
ExecStart=/usr/local/bin/traffic_manager $TM_DAEMON_ARGS
ExecReload=/usr/local/bin/traffic_manager config reload

[Install]
WantedBy=multi-user.target
EOL

# Reload systemd and enable the service
systemctl daemon-reload
systemctl enable trafficserver
systemctl start trafficserver

