#!/bin/sh
echo "127.0.0.1 rivet-server" >> /etc/hosts
echo "127.0.0.1 rivet-client" >> /etc/hosts
echo "127.0.0.1 cockroachdb" >> /etc/hosts
echo "127.0.0.1 redis" >> /etc/hosts
echo "127.0.0.1 clickhouse" >> /etc/hosts
echo "127.0.0.1 nats" >> /etc/hosts
echo "127.0.0.1 seaweedfs" >> /etc/hosts
echo "127.0.0.1 vector-client" >> /etc/hosts
echo "127.0.0.1 vector-server" >> /etc/hosts
