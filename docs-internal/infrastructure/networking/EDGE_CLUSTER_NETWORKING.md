# Edge Cluster Networking

## `gg`

| Port        | Type   | Protocol  | Description                                      |
| ----------- | ------ | --------- | ------------------------------------------------ |
| 80          | Public | TCP       | HTTP & WebSocket traffic to game serves.         |
| 443         | Public | TCP       | HTTPs & secure WebSocket traffic to game serves. |
| 20000-31999 | Public | TCP & UDP | Ingress traffic to game servers.                 |

## `job`

| Port        | Type    | Protocol  | Description                                                                                                                                                                                    |
| ----------- | ------- | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 20000-25999 | Private | TCP & UDP | Protected ports that game server ports are assigned to to receive traffic from GG. Nomad automatically assigns ports on this port range.                                                       |
| 26000-31999 | Public  | TCP & UDP | Unprotected port ranges that job servers using host networking can listen on. Traffic does not go through GG and is vulnerable to attacks. Ports in this range are not automatically assigned. |

## `ats`

| Port | Type    | Protocol | Description |
| ---- | ------- | -------- | ----------- |
| 8080 | Private | TCP      | ATS Server. |
