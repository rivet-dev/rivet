# Append config
# cat << 'EOF' >> /etc/foundationdb/foundationdb.conf
# [fdbserver]
# EOF

# TODO: add -t flag for TLS (https://apple.github.io/foundationdb/tls.html#enable-tls)
# Make fdb accessible on VLAN
python3 /usr/lib/foundationdb/make_public.py -a ___VLAN_IP___
