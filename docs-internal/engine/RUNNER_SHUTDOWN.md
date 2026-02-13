# Runner Shutdown

## 1. Self initiated shutdown

1. Runner sends ToServerStopping to runner WS
2. Runner WS proxies ToServerStopping to runner WF
3. Runner WF sets itself as "draining", preventing future actor allocations to it
4. Runner WF sends GoingAway signal to all actor WFs
5. Once the runner lost threshold is passed, runner WF sends ToClientClose to runner WS
6. Runner WS closes connection to runner, informing it not to attempt reconnection

## 2. Rivet initiated shutdown

1. Runner WF receives Stop signal
2. Runner WF sends GoingAway signal to all actor WFs
3. Once the runner lost threshold is passed, runner WF sends ToClientClose to runner WS
4. Runner WS closes connection to runner, informing it not to attempt reconnection
