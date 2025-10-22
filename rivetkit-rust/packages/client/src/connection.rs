use anyhow::Result;
use futures_util::FutureExt;
use serde_json::Value;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, oneshot, watch, Mutex};

use crate::{
    backoff::Backoff,
    protocol::{query::ActorQuery, *},
    drivers::*,
    remote_manager::RemoteManager,
    EncodingKind,
    TransportKind
};
use tracing::debug;


type RpcResponse = Result<to_client::ActionResponse, to_client::Error>;
type EventCallback = dyn Fn(&Vec<Value>) + Send + Sync;

struct SendMsgOpts {
    ephemeral: bool,
}

impl Default for SendMsgOpts {
    fn default() -> Self {
        Self { ephemeral: false }
    }
}

// struct WatchPair {
//     tx: watch::Sender<bool>,
//     rx: watch::Receiver<bool>,
// }
type WatchPair = (watch::Sender<bool>, watch::Receiver<bool>);

pub type ActorConnection = Arc<ActorConnectionInner>;

struct ConnectionAttempt {
    did_open: bool,
    _task_end_reason: DriverStopReason,
}

pub struct ActorConnectionInner {
    remote_manager: RemoteManager,
    transport_kind: TransportKind,
    encoding_kind: EncodingKind,
    query: ActorQuery,
    parameters: Option<Value>,

    driver: Mutex<Option<DriverHandle>>,
    msg_queue: Mutex<Vec<Arc<to_server::ToServer>>>,

    rpc_counter: AtomicU64,
    in_flight_rpcs: Mutex<HashMap<u64, oneshot::Sender<RpcResponse>>>,

    event_subscriptions: Mutex<HashMap<String, Vec<Box<EventCallback>>>>,

    // Connection info for reconnection
    actor_id: Mutex<Option<String>>,
    connection_id: Mutex<Option<String>>,
    connection_token: Mutex<Option<String>>,

    dc_watch: WatchPair,
    disconnection_rx: Mutex<Option<oneshot::Receiver<()>>>,
}

impl ActorConnectionInner {
    pub(crate) fn new(
        remote_manager: RemoteManager,
        query: ActorQuery,
        transport_kind: TransportKind,
        encoding_kind: EncodingKind,
        parameters: Option<Value>,
    ) -> ActorConnection {
        Arc::new(Self {
            remote_manager,
            transport_kind,
            encoding_kind,
            query,
            parameters,
            driver: Mutex::new(None),
            msg_queue: Mutex::new(Vec::new()),
            rpc_counter: AtomicU64::new(0),
            in_flight_rpcs: Mutex::new(HashMap::new()),
            event_subscriptions: Mutex::new(HashMap::new()),
            actor_id: Mutex::new(None),
            connection_id: Mutex::new(None),
            connection_token: Mutex::new(None),
            dc_watch: watch::channel(false),
            disconnection_rx: Mutex::new(None),
        })
    }

    fn is_disconnecting(self: &Arc<Self>) -> bool {
        *self.dc_watch.1.borrow() == true
    }

    async fn try_connect(self: &Arc<Self>) -> ConnectionAttempt {
        // Get connection info for reconnection
        let conn_id = self.connection_id.lock().await.clone();
        let conn_token = self.connection_token.lock().await.clone();

        let Ok((driver, mut recver, task)) = connect_driver(
            self.transport_kind,
            DriverConnectArgs {
                remote_manager: self.remote_manager.clone(),
                query: self.query.clone(),
                encoding_kind: self.encoding_kind,
                parameters: self.parameters.clone(),
                conn_id,
                conn_token,
            }
        ).await else {
            // Either from immediate disconnect (local device connection refused)
            // or from error like invalid URL
            return ConnectionAttempt {
                did_open: false,
                _task_end_reason: DriverStopReason::TaskError,
            };
        };

        {
            let mut my_driver = self.driver.lock().await;
            *my_driver = Some(driver);
        }

        let mut task_end_reason = task.map(|res| match res {
            Ok(a) => a,
            Err(task_err) => {
                if task_err.is_cancelled() {
                    debug!("Connection task was cancelled");
                    DriverStopReason::UserAborted
                } else {
                    DriverStopReason::TaskError
                }
            }
        });

        let mut did_connection_open = false;

        // spawn listener for rpcs
        let task_end_reason = loop {
            tokio::select! {
                reason = &mut task_end_reason => {
                    debug!("Connection closed: {:?}", reason);

                    break reason;
                },
                msg = recver.recv() => {
                    // If the sender is dropped, break the loop
                    let Some(msg) = msg else {
                        // break DriverStopReason::ServerDisconnect;
                        continue;
                    };

                    if let to_client::ToClientBody::Init(_) = &msg.body {
                        did_connection_open = true;
                    }

                    self.on_message(msg).await;
                }
            }
        };

        'destroy_driver: {
            debug!("Destroying driver");
            let mut d_guard = self.driver.lock().await;
            let Some(d) = d_guard.take() else {
                // We destroyed the driver already,
                // e.g. .disconnect() was called
                break 'destroy_driver;
            };

            d.disconnect();
        }

        ConnectionAttempt {
            did_open: did_connection_open,
            _task_end_reason: task_end_reason,
        }
    }

    async fn on_open(self: &Arc<Self>, init: &to_client::Init) {
        debug!("Connected to server: {:?}", init);

        // Store connection info for reconnection
        *self.actor_id.lock().await = Some(init.actor_id.clone());
        *self.connection_id.lock().await = Some(init.connection_id.clone());
        *self.connection_token.lock().await = Some(init.connection_token.clone());

        for (event_name, _) in self.event_subscriptions.lock().await.iter() {
            self.send_subscription(event_name.clone(), true).await;
        }

        // Flush message queue
        for msg in self.msg_queue.lock().await.drain(..) {
            // If its in the queue, it isn't ephemeral, so we pass
            // default SendMsgOpts
            self.send_msg(msg, SendMsgOpts::default()).await;
        }
    }

    async fn on_message(self: &Arc<Self>, msg: Arc<to_client::ToClient>) {
        let body = &msg.body;

        match body {
            to_client::ToClientBody::Init(init) => {
                self.on_open(init).await;
            }
            to_client::ToClientBody::ActionResponse(ar) => {
                let id = ar.id;
                let mut in_flight_rpcs = self.in_flight_rpcs.lock().await;
                let Some(tx) = in_flight_rpcs.remove(&id) else {
                    debug!("Unexpected response: rpc id not found");
                    return;
                };
                if let Err(e) = tx.send(Ok(ar.clone())) {
                    debug!("{:?}", e);
                    return;
                }
            }
            to_client::ToClientBody::Event(ev) => {
                // Decode CBOR args
                let args: Vec<Value> = match serde_cbor::from_slice(&ev.args) {
                    Ok(a) => a,
                    Err(e) => {
                        debug!("Failed to decode event args: {:?}", e);
                        return;
                    }
                };

                let listeners = self.event_subscriptions.lock().await;
                if let Some(callbacks) = listeners.get(&ev.name) {
                    for cb in callbacks {
                        cb(&args);
                    }
                }
            }
            to_client::ToClientBody::Error(e) => {
                if let Some(action_id) = e.action_id {
                    let mut in_flight_rpcs = self.in_flight_rpcs.lock().await;
                    let Some(tx) = in_flight_rpcs.remove(&action_id) else {
                        debug!("Unexpected response: rpc id not found");
                        return;
                    };
                    if let Err(e) = tx.send(Err(e.clone())) {
                        debug!("{:?}", e);
                        return;
                    }

                    return;
                }

                debug!("Connection error: {} - {}", e.code, e.message);
            }
        }
    }

    async fn send_msg(self: &Arc<Self>, msg: Arc<to_server::ToServer>, opts: SendMsgOpts) {
        let guard = self.driver.lock().await;

        'send_immediately: {
            let Some(driver) = guard.deref() else {
                break 'send_immediately;
            };

            let Ok(_) = driver.send(msg.clone()).await else {
                break 'send_immediately;
            };

            return;
        }

        // Otherwise queue
        if opts.ephemeral == false {
            self.msg_queue.lock().await.push(msg.clone());
        }

        return;
    }

    pub async fn action(self: &Arc<Self>, method: &str, params: Vec<Value>) -> Result<Value> {
        let id: u64 = self.rpc_counter.fetch_add(1, Ordering::SeqCst);

        let (tx, rx) = oneshot::channel();
        self.in_flight_rpcs.lock().await.insert(id, tx);

        // Encode params as CBOR
        let args_cbor = serde_cbor::to_vec(&params)?;

        self.send_msg(
            Arc::new(to_server::ToServer {
                body: to_server::ToServerBody::ActionRequest(
                    to_server::ActionRequest {
                        id,
                        name: method.to_string(),
                        args: args_cbor,
                    },
                ),
            }),
            SendMsgOpts::default(),
        )
        .await;

        let Ok(res) = rx.await else {
            return Err(anyhow::anyhow!("Socket closed during rpc"));
        };

        match res {
            Ok(ok) => {
                // Decode CBOR output
                let output: Value = serde_cbor::from_slice(&ok.output)?;
                Ok(output)
            }
            Err(err) => {
                let metadata = if let Some(md) = &err.metadata {
                    match serde_cbor::from_slice::<Value>(md) {
                        Ok(v) => v,
                        Err(_) => Value::Null,
                    }
                } else {
                    Value::Null
                };

                Err(anyhow::anyhow!(
                    "RPC Error({}/{}): {}, {:#}",
                    err.group,
                    err.code,
                    err.message,
                    metadata
                ))
            }
        }
    }

    async fn send_subscription(self: &Arc<Self>, event_name: String, subscribe: bool) {
        self.send_msg(
            Arc::new(to_server::ToServer {
                body: to_server::ToServerBody::SubscriptionRequest(
                    to_server::SubscriptionRequest {
                        event_name,
                        subscribe,
                    },
                ),
            }),
            SendMsgOpts { ephemeral: true },
        )
        .await;
    }

    async fn add_event_subscription(
        self: &Arc<Self>,
        event_name: String,
        callback: Box<EventCallback>,
    ) {
        // TODO: Support for once
        let mut listeners = self.event_subscriptions.lock().await;

        let is_new_subscription = listeners.contains_key(&event_name) == false;

        listeners
            .entry(event_name.clone())
            .or_insert(Vec::new())
            .push(callback);

        if is_new_subscription {
            self.send_subscription(event_name, true).await;
        }
    }

    pub async fn on_event<F>(self: &Arc<Self>, event_name: &str, callback: F)
    where
        F: Fn(&Vec<Value>) + Send + Sync + 'static,
    {
        self.add_event_subscription(event_name.to_string(), Box::new(callback))
            .await
    }

    pub async fn disconnect(self: &Arc<Self>) {
        if self.is_disconnecting() {
            // We are already disconnecting
            return;
        }

        debug!("Disconnecting from actor conn");

        self.dc_watch.0.send(true).ok();

        if let Some(d) = self.driver.lock().await.deref() {
            d.disconnect();
        }
        self.in_flight_rpcs.lock().await.clear();
        self.event_subscriptions.lock().await.clear();
        let Some(rx) = self.disconnection_rx.lock().await.take() else {
            return;
        };

        rx.await.ok();
    }
}


pub fn start_connection(
    conn: &Arc<ActorConnectionInner>,
    mut shutdown_rx: broadcast::Receiver<()>
) {
    let (tx, rx) = oneshot::channel();

    let conn = conn.clone();

    tokio::spawn(async move {
        {
            let mut stop_rx = conn.disconnection_rx.lock().await;
            if stop_rx.is_some() {
                // Already doing connection_with_retry
                // - this drops the oneshot
                return;
            }

            *stop_rx = Some(rx);
        }

        'keepalive: loop {
            debug!("Attempting to reconnect");
            let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
            let mut retry_attempt = 0;
            'retry: loop {
                retry_attempt += 1;
                debug!(
                    "Establish conn: attempt={}, timeout={:?}",
                    retry_attempt,
                    backoff.delay()
                );
                let attempt = conn.try_connect().await;

                if conn.is_disconnecting() {
                    break 'keepalive;
                }

                if attempt.did_open {
                    break 'retry;
                }

                let mut dc_rx = conn.dc_watch.0.subscribe();

                tokio::select! {
                    _ = backoff.tick() => {},
                    _ = dc_rx.wait_for(|x| *x == true) => {
                        break 'keepalive;
                    }
                    _ = shutdown_rx.recv() => {
                        debug!("Received shutdown signal, stopping connection attempts");
                        break 'keepalive;
                    }
                }
            }
        }

        tx.send(()).ok();
        conn.disconnection_rx.lock().await.take();
    });
}

impl Debug for ActorConnectionInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorConnection")
            .field("transport_kind", &self.transport_kind)
            .field("encoding_kind", &self.encoding_kind)
            .finish()
    }
}