// Local leadership/lifecycle modifications; see LEADERSHIP.md in this package.
//! SlateDB makes use of [async_channel] to offload work to background tasks when
//! possible. Examples of these tasks include [crate::mem_table_flush],
//! [crate::checkpoint], and [crate::compactor]. Each task's lifecycle is fairly
//! similar; they:
//!
//! 1. Receive messages and perform some work
//! 2. Perform work based on a fixed schedule
//! 3. Manage error states when failures occur
//! 4. Drain messages on shutdown
//! 5. Clean up resources on shutdown
//!
//! [MessageDispatcher], [MessageHandlerExecutor], and [MessageHandler] standardize
//! this pattern in a single set of event loops.
//!
//! - [MessageHandlerExecutor] spawns background tasks, monitors them for completion, and
//!   updates [crate::db_state::DbState::closed_result] accordingly.
//! - [MessageDispatcher] runs an event loop for a single background task. It receives
//!   messages and ticks, and passes them to the [MessageHandler] for processing.
//! - [MessageHandler] receives callbacks from the dispatcher based on lifetime events
//!   (messages, ticks, etc.) and performs work based on those events.
//!
//! SlateDB instantiates a [MessageHandlerExecutor] and calls
//! [MessageHandlerExecutor::add_handler] for each [MessageHandler]. The DB then calls
//! [MessageHandlerExecutor::monitor_on] to start the event loop and monitor tasks until
//! shutdown.
//!
//! Handlers can also be grouped for parallel processing using
//! [MessageHandlerExecutor::add_handlers]. All handlers in a group share a single
//! [async_channel::Receiver], enabling multi-consumer message distribution. Each handler
//! runs its own [MessageDispatcher] event loop with independent tickers, but the group
//! shares a single [tokio_util::sync::CancellationToken] and result cell. This is useful
//! for tasks like parallel L0 uploading where multiple handlers can process messages from
//! the same channel concurrently.
//!
//! ## Example
//!
//! ```ignore
//! # use slatedb::dispatcher::{MessageHandlerExecutor, MessageHandler, MessageFactory, MessageTickerDef};
//! # use slatedb::error::SlateDBError;
//! # use slatedb_common::clock::DefaultSystemClock;
//! # use slatedb::utils::WatchableOnceCell;
//! # use std::sync::Arc;
//! # use std::time::Duration;
//! # use async_channel;
//! # use tokio::runtime::Handle;
//! # use futures::stream::BoxStream;
//! # use futures::StreamExt;
//! # #[tokio::main]
//! # async fn main() {
//! enum Message {
//!     Say(String),
//! }
//!
//! struct PrintMessageHandler;
//!
//! #[async_trait::async_trait]
//! impl MessageHandler<Message> for PrintMessageHandler {
//!     fn tickers(&mut self) -> Vec<MessageTickerDef<Message>> {
//!         let mut tickers = Vec::new();
//!         tickers.push(MessageTickerDef::new(
//!             Duration::from_secs(1),
//!             Box::new(|| Message::Say("tick".to_string())),
//!         ));
//!         tickers
//!     }
//!
//!     async fn handle(
//!         &mut self,
//!         message: Message,
//!     ) -> Result<(), SlateDBError> {
//!         match message {
//!             Message::Say(msg) => println!("{}", msg),
//!         }
//!         Ok(())
//!     }
//!
//!     async fn cleanup(
//!         &mut self,
//!         mut messages: BoxStream<'async_trait, Message>,
//!         result: Result<(), SlateDBError>,
//!     ) -> Result<(), SlateDBError> {
//!         match result {
//!             Ok(_) | Err(SlateDBError::Closed) => {
//!                 while let Some(m) = messages.next().await {
//!                     let _ = self.handle(m).await;
//!                 }
//!             },
//!             // skipping drain messages on unclean shutdown
//!             _ => {},
//!         }
//!         Ok(())
//!     }
//! }
//!
//! let clock = Arc::new(DefaultSystemClock::default());
//! let (tx, rx) = async_channel::unbounded();
//! let closed_result = WatchableOnceCell::new();
//! let handle = Handle::current();
//! let task_executor = MessageHandlerExecutor::new(
//!     closed_result,
//!     clock,
//! );
//! task_executor.add_handler(
//!     "print_message_handler".to_string(),
//!     Box::new(PrintMessageHandler),
//!     rx,
//!     &handle,
//! ).expect("failed to add handler");
//! let join_handle = task_executor.monitor_on(&handle)
//!     .expect("failed to start monitoring");
//! tx.try_send(Message::Say("hello".to_string())).unwrap();
//! task_executor.shutdown_task("print_message_handler").await.ok();
//! join_handle.await.ok();
//! # }
//! ```

use std::{
    collections::HashMap, future::Future, panic::AssertUnwindSafe, pin::Pin, sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use crossbeam_skiplist::SkipMap;
use fail_parallel::{fail_point, FailPointRegistry};
use futures::{
    future::BoxFuture,
    stream::{BoxStream, FuturesUnordered},
    FutureExt, StreamExt,
};
use log::error;
use parking_lot::Mutex;
use tokio::{runtime::Handle, task::JoinHandle};
use tokio_util::{sync::CancellationToken, task::JoinMap};

use crate::{
    db_status::ClosedResultWriter,
    error::SlateDBError,
    utils::{panic_string, split_join_result, split_unwind_result, WatchableOnceCell},
};
use slatedb_common::{
    clock::{SystemClock, SystemClockTicker},
    DbRand,
};

/// A factory for creating messages when a [MessageDispatcherTicker] ticks.
pub(crate) type MessageFactory<T> = dyn Fn() -> T + Send;

/// A single ticker definition returned by [`MessageHandler::tickers`]: a base
/// interval, a [`MessageFactory`] to produce a message on each tick, and an
/// optional per-ticker jitter spread.
pub(crate) struct MessageTickerDef<T: Send> {
    /// Produces a fresh message on each tick.
    pub(crate) factory: Box<MessageFactory<T>>,
    /// The base tick interval. When jitter is enabled, the randomized wait is
    /// centered on this interval.
    pub(crate) interval: Duration,
    /// The jitter range, as a fraction of the interval.
    /// Each tick waits a uniform random duration in `[interval * (1 - jitter.0), interval * (1 + jitter.0)]`.
    pub(crate) jitter: Option<(f64, Arc<DbRand>)>,
}

impl<T: Send> MessageTickerDef<T> {
    /// Creates an unjittered ticker that fires every `duration`.
    pub(crate) fn new(interval: Duration, factory: Box<MessageFactory<T>>) -> Self {
        Self {
            interval,
            factory,
            jitter: None,
        }
    }

    /// Jitters each tick over `[interval * (1 - spread), interval * (1 + spread)]`.
    /// For example, `spread = 0.5` waits the interval plus or minus half. See
    /// [`MessageTickerDef`] for details.
    pub(crate) fn with_jitter(mut self, spread: f64, rand: Arc<DbRand>) -> Self {
        self.jitter = Some((spread, rand));
        self
    }
}

/// An async event source that produces messages for the dispatcher.
/// Each call to [`notify`](Notifier::notify) waits for the next event and
/// returns the corresponding message. Pending notifications are dropped on
/// shutdown; handlers should check notification sources explicitly in
/// [`cleanup`](MessageHandler::cleanup) if needed.
#[async_trait]
pub(crate) trait Notifier<T: Send>: Send {
    async fn notify(&mut self) -> T;
}

/// Wraps a [Notifier] so its futures can be collected into a
/// [FuturesUnordered], returning `&mut self` alongside the message so the
/// caller can push the next future back in.
struct DispatcherNotifier<T: Send> {
    inner: Box<dyn Notifier<T>>,
}

impl<T: Send> DispatcherNotifier<T> {
    fn next(&mut self) -> Pin<Box<dyn Future<Output = (T, &mut Self)> + Send + '_>> {
        Box::pin(async move {
            let msg = self.inner.notify().await;
            (msg, self)
        })
    }
}

/// A dispatcher that invokes [MessageHandler] callbacks when events occur.
///
/// [MessageDispatcher] receives a [MessageHandler] and [async_channel::Receiver<T>].
/// Messages sent to the receiver are passed to the [MessageHandler] for processing.
///
/// [MessageDispatcher::run] is the primary entry point for running the dispatcher;
/// it is responsible for running the main event loop. The function receives messages
/// from the [async_channel::Receiver<T>] and from any [MessageDispatcherTicker]s, and
/// passes them to the [MessageHandler] for processing.
struct MessageDispatcher<T: Send + std::fmt::Debug> {
    handler: Box<dyn MessageHandler<T>>,
    rx: async_channel::Receiver<T>,
    clock: Arc<dyn SystemClock>,
    cancellation_token: CancellationToken,
    #[allow(dead_code)]
    fp_registry: Arc<FailPointRegistry>,
}

impl<T: Send + std::fmt::Debug> MessageDispatcher<T> {
    /// Creates a new [MessageDispatcher]. Messages sent to the channel are passed to
    /// the [MessageHandler] for processing.
    ///
    /// ## Arguments
    ///
    /// * `handler`: The [MessageHandler] to use for processing messages.
    /// * `rx`: The [async_channel::Receiver<T>] to use for receiving messages.
    /// * `clock`: The [SystemClock] to use for time.
    /// * `cancellation_token`: The [CancellationToken] to use for shutdown.
    #[allow(dead_code)]
    fn new(
        handler: Box<dyn MessageHandler<T>>,
        rx: async_channel::Receiver<T>,
        clock: Arc<dyn SystemClock>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            handler,
            rx,
            clock,
            cancellation_token,
            fp_registry: Arc::new(FailPointRegistry::new()),
        }
    }

    /// Creates a new [MessageDispatcher]. Messages sent to the channel are passed to
    /// the [MessageHandler] for processing. A fail point registry is provided for
    /// fail points.
    ///
    /// ## Arguments
    ///
    /// * `handler`: The [MessageHandler] to use for processing messages.
    /// * `rx`: The [async_channel::Receiver<T>] to use for receiving messages.
    /// * `clock`: The [SystemClock] to use for time.
    /// * `cancellation_token`: The [CancellationToken] to use for shutdown.
    /// * `fp_registry`: The [FailPointRegistry] to use for fail points.
    #[allow(dead_code)]
    fn with_fp_registry(mut self, fp_registry: Arc<FailPointRegistry>) -> Self {
        self.fp_registry = fp_registry;
        self
    }

    /// Runs the main event loop for the dispatcher. This is where messages and ticker
    /// events are processed.
    ///
    /// [MessageDispatcher::run] contains a message loop with the following control
    /// flow:
    ///
    /// 1. Break immediately if the task is cancelled and returns `Ok(())`.
    /// 2. Else, if there is a message, read it and invoke [MessageHandler::handle].
    /// 3. Else, if there is a ticker event, read it and invoke [MessageHandler::handle].
    ///
    /// ## Returns
    ///
    /// A [Result] containing `Ok(())` on clean shutdown, or an error if the handler
    /// fails for any reason.
    async fn run(&mut self) -> Result<(), SlateDBError> {
        let mut tickers = self
            .handler
            .tickers()
            .into_iter()
            .map(|def| {
                let mut ticker = self.clock.ticker(def.interval);
                if let Some(ref jitter) = def.jitter {
                    ticker.with_jitter(jitter.0, jitter.1.clone());
                }
                MessageDispatcherTicker::new(ticker, def)
            })
            .collect::<Vec<_>>();
        let mut ticker_futures: FuturesUnordered<_> =
            tickers.iter_mut().map(|t| t.tick()).collect();
        let mut notifiers = self
            .handler
            .notifiers()
            .into_iter()
            .map(|n| DispatcherNotifier { inner: n })
            .collect::<Vec<_>>();
        let mut notifier_futures: FuturesUnordered<_> =
            notifiers.iter_mut().map(|n| n.next()).collect();
        loop {
            fail_point!(Arc::clone(&self.fp_registry), "dispatcher-run-loop", |_| {
                Err(SlateDBError::Fenced)
            });
            tokio::select! {
                biased;
                // stop the loop if we're in an error state or cancelled
                _ = self.cancellation_token.cancelled() => {
                    break;
                }
                // if no errors, prioritize messages
                Ok(message) = self.rx.recv() => {
                    self.handler.handle(message).await?;
                },
                // if no messages, check tickers
                Some((message, ticker)) = ticker_futures.next() => {
                    self.handler.handle(message).await?;
                    ticker_futures.push(ticker.tick());
                },
                // if no messages or tickers, check notifiers
                Some((message, notifier)) = notifier_futures.next() => {
                    self.handler.handle(message).await?;
                    notifier_futures.push(notifier.next());
                },
            }
        }
        Ok(())
    }

    /// Runs the full dispatcher lifecycle: [run] with panic catching, write to
    /// `closed_result`, then [cleanup]. Returns the run result.
    #[allow(clippy::panic)] // for failpoint
    async fn run_lifecycle(
        mut self,
        name: String,
        closed_result: Arc<dyn ClosedResultWriter>,
        #[allow(unused_variables)] fp_registry: Arc<FailPointRegistry>,
    ) -> Result<(), SlateDBError> {
        // catch dispatcher panics using catch_unwind
        let run_unwind_result = AssertUnwindSafe(self.run()).catch_unwind().await;
        let (run_result, run_maybe_panic) = split_unwind_result(name.clone(), run_unwind_result);
        if let Err(ref err) = run_result {
            error!(
                "background task exited unexpectedly. [task_name={}, error={:?}, panic={:?}]",
                name,
                err,
                run_maybe_panic.map(|p| panic_string(&p))
            );
        }
        fail_point!(fp_registry.clone(), "executor-wrapper-before-write", |_| {
            panic!("failpoint: executor-wrapper-before-write");
        });
        // set result in db state (first writer wins)
        closed_result.write_result(run_result.clone());
        // re-read the result since it might have already been set by another task
        let final_result = closed_result
            .result_reader()
            .read()
            .expect("error state was unexpectedly empty");
        let cleanup_unwind_result = AssertUnwindSafe(self.cleanup(final_result))
            .catch_unwind()
            .await;
        let (cleanup_result, cleanup_maybe_panic) =
            split_unwind_result(name.clone(), cleanup_unwind_result);
        if let Err(err) = cleanup_result {
            error!(
                "background task failed to clean up on shutdown [name={}, error={:?}, panic={:?}]",
                name,
                err,
                cleanup_maybe_panic.map(|p| panic_string(&p))
            );
        }
        run_result
    }

    /// Tells the handler to clean up any resources.
    ///
    /// If cleanup fails, the error is returned.
    ///
    /// ## Arguments
    ///
    /// * `result`: The value of [crate::db_state::DbState::closed_result] when
    ///   [MessageDispatcher::run] returns.
    ///
    /// ## Returns
    ///
    /// The [Result] after cleaning up resources.
    async fn cleanup(&mut self, result: Result<(), SlateDBError>) -> Result<(), SlateDBError> {
        fail_point!(Arc::clone(&self.fp_registry), "dispatcher-cleanup", |_| {
            Err(SlateDBError::Fenced)
        });
        self.rx.close();
        let messages = futures::stream::unfold(&mut self.rx, |rx| async move {
            rx.recv().await.ok().map(|message| (message, rx))
        });
        self.handler.cleanup(Box::pin(messages), result).await
    }
}

/// A ticker that generates messages at regular intervals.
///
/// On each [MessageDispatcherTicker::tick], the message factory is called to generate a
/// message, and the message is returned as a [Future].
struct MessageDispatcherTicker<'a, T: Send> {
    inner: SystemClockTicker<'a>,
    def: MessageTickerDef<T>,
}

impl<'a, T: Send> MessageDispatcherTicker<'a, T> {
    /// Creates a new [MessageDispatcherTicker].
    ///
    /// ## Arguments
    ///
    /// * `inner`: The [SystemClockTicker] that drives ticking. Jitter, when
    ///   enabled, is applied inside this ticker.
    /// * `def`: The [MessageTickerDef] describing the interval and message
    ///   factory.
    ///
    /// ## Returns
    ///
    /// The new [MessageDispatcherTicker].
    fn new(inner: SystemClockTicker<'a>, def: MessageTickerDef<T>) -> Self {
        Self { inner, def }
    }

    /// Returns a [Future] that resolves when the ticker ticks, and returns the message
    /// generated by the message factory.
    ///
    /// The wait is driven by the `inner` [SystemClockTicker], which applies
    /// jitter when it is enabled. The wait lives inside this future, so the
    /// dispatcher loop keeps servicing messages and other tickers while it is
    /// pending.
    ///
    /// ## Returns
    ///
    /// A [Future] that resolves when the ticker ticks.
    fn tick(&mut self) -> Pin<Box<dyn Future<Output = (T, &mut Self)> + Send + '_>> {
        let message = (self.def.factory)();
        Box::pin(async move {
            self.inner.tick().await;
            (message, self)
        })
    }
}

/// [MessageDispatcher] event loop callbacks are implemented in a [MessageHandler].
/// Handlers are responsible for:
///
/// 1. Processing messages ([MessageHandler::handle])
/// 2. Defining ticker schedules (when to tick a certain message) ([MessageHandler::tickers])
/// 3. Cleaning up resources ([MessageHandler::cleanup])
///
/// It is safe to return errors on failure or panic; the [MessageDispatcher] and
/// [MessageHandlerExecutor] will handle them appropriately.
#[async_trait]
pub(crate) trait MessageHandler<T: Send>: Send {
    /// Defines message ticker schedules. [MessageDispatcher::run] instantiates a
    /// [MessageDispatcherTicker] for each ticker defined here. Whenever each ticker
    /// ticks, the message factory generates a message, and [MessageDispatcher] sends the
    /// message to this [MessageHandler].
    ///
    /// ## Returns
    ///
    /// A vector of [MessageTickerDef]s, each defining a tick interval, a message
    /// factory to generate a new message on each tick, and optional per-ticker
    /// jitter. Handlers whose tickers would otherwise synchronize across
    /// independent processes (for example, the compaction worker polling
    /// `.compactions`) can attach a jitter spread via
    /// [MessageTickerDef::with_jitter] to spread their tick timing.
    fn tickers(&mut self) -> Vec<MessageTickerDef<T>> {
        vec![]
    }

    /// Returns async event sources that produce messages for the dispatcher.
    /// Each [Notifier] is polled alongside messages and tickers. When a
    /// notifier's [`notify`](Notifier::notify) resolves, the produced message
    /// is passed to [`handle`](Self::handle). Called once before the dispatcher
    /// loop starts.
    fn notifiers(&mut self) -> Vec<Box<dyn Notifier<T>>> {
        vec![]
    }

    /// Handles a message. Messages can come from either a channel or a ticker. See
    /// [crate::dispatcher] for details.
    ///
    /// ## Arguments
    ///
    /// * `message`: The message to handle.
    /// * `error`: An optional error to pass to the handler. If set, this argument
    ///   signals to the [MessageHandler] that the database is in an error state during
    ///   shutdown.
    ///
    /// ## Returns
    ///
    /// The [Result] after handling the message.
    async fn handle(&mut self, message: T) -> Result<(), SlateDBError>;

    /// Cleans up resources. This method should not be used to continue processing
    /// messages or writing to the database.
    ///
    /// ## Arguments
    ///
    /// * `messages`: An iterator of messages still in the channel after
    ///   [MessageDispatcher::run] returns. If a handler fails, this iterator will not
    ///   include the message that triggered the failure.
    /// * `result`: The value of [crate::db_state::DbState::closed_result] when
    ///   [MessageDispatcher::run] returns.
    ///
    /// ## Returns
    ///
    /// The [Result] after cleaning up resources.
    async fn cleanup(
        &mut self,
        messages: BoxStream<'async_trait, T>,
        result: Result<(), SlateDBError>,
    ) -> Result<(), SlateDBError>;
}

/// A builder data structure for [MessageHandlerExecutor]. The executor creates a
/// [MessageHandlerFuture] for each handler it receives in
/// [MessageHandlerExecutor::add_handler]. The future is used to run the handler's
/// dispatcher ([MessageDispatcher::run]) and handle cleanup
/// ([MessageDispatcher::cleanup]).
struct MessageHandlerFuture {
    /// The name of the task group.
    name: String,
    /// The index of this handler within its group.
    group_index: usize,
    /// A future that runs the handler's dispatcher and handles cleanup.
    future: BoxFuture<'static, Result<(), SlateDBError>>,
    /// A cancellation token used to cancel the handler's dispatcher.
    token: CancellationToken,
    /// A runtime handle used to spawn the future.
    handle: Handle,
}

/// Tracks per-group state in the monitor loop: how many members remain
/// and the aggregate result (first error wins).
struct TaskGroup {
    remaining: usize,
    result: Result<(), SlateDBError>,
}

impl TaskGroup {
    fn new(count: usize) -> Self {
        Self {
            remaining: count,
            result: Ok(()),
        }
    }

    /// Merge a member result. First error wins. Returns `true` when this
    /// was the last member (i.e. the group is now complete).
    fn member_completed(&mut self, result: Result<(), SlateDBError>) -> bool {
        if self.result.is_ok() {
            self.result = result;
        }
        self.remaining -= 1;
        self.remaining == 0
    }
}

/// Owns the state needed by the monitor loop and provides a single
/// `run` method that drives it.
struct TaskMonitor {
    tasks: JoinMap<(String, usize), Result<(), SlateDBError>>,
    groups: HashMap<String, TaskGroup>,
    closed_result: Arc<dyn ClosedResultWriter>,
    results: Arc<SkipMap<String, WatchableOnceCell<Result<(), SlateDBError>>>>,
    tokens: Vec<CancellationToken>,
}

impl TaskMonitor {
    /// Run the monitor loop until all tasks complete.
    async fn run(mut self) {
        while !self.tasks.is_empty() {
            if let Some(((group_name, _), join_result)) = self.tasks.join_next().await {
                let (task_result, task_maybe_panic) =
                    split_join_result(group_name.clone(), join_result);

                if let Err(ref err) = task_result {
                    error!(
                        "background task failed [name={}, error={:?}, panic={:?}]",
                        group_name,
                        err,
                        task_maybe_panic.map(|p| panic_string(&p))
                    );
                    self.closed_result.write_result(task_result.clone());
                    self.tokens.iter().for_each(|t| t.cancel());
                }

                let group = self
                    .groups
                    .get_mut(&group_name)
                    .expect("group tracking entry missing");

                if group.member_completed(task_result) {
                    let group_result = self
                        .groups
                        .remove(&group_name)
                        .expect("group tracking entry missing on final member")
                        .result;
                    self.results
                        .get(&group_name)
                        .expect("result cell isn't set when expected")
                        .value()
                        .write(group_result);
                }
            }
        }
    }
}

impl std::fmt::Debug for MessageHandlerFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageHandlerFuture")
            .field("name", &self.name)
            .finish()
    }
}

/// The [MessageHandlerExecutor] is responsible for spawning, monitoring, and shutting
/// down [MessageDispatcher]s. Think of it as a dispatcher pool.
///
/// Each dispatcher is associated with a name, which is used to identify the dispatcher
/// in the [MessageHandlerExecutor]. As dispatchers complete, their results are stored
/// in the [MessageHandlerExecutor]. The first completed dispatcher's results are also
/// used for [crate::db_state::DbState::closed_result].
///
/// The executor also catches panics, and will convert them to an appropriate
/// [SlateDBError].
#[derive(Debug)]
pub(crate) struct MessageHandlerExecutor {
    /// A vector of futures for each dispatcher. Set to `None` when executor starts.
    futures: Mutex<Option<Vec<MessageHandlerFuture>>>,
    /// A map of cancellation tokens for each dispatcher.
    tokens: SkipMap<String, CancellationToken>,
    /// A map of (task name, results) for dispatchers that have returned.
    results: Arc<SkipMap<String, WatchableOnceCell<Result<(), SlateDBError>>>>,
    /// A wrapper that stores the final result of the database lifecycle and
    /// auto-reports the close reason. Ok(()) indicates a clean shutdown; Err(e)
    /// indicates an error.
    closed_result: Arc<dyn ClosedResultWriter>,
    /// A system clock for time keeping.
    clock: Arc<dyn SystemClock>,
    /// A fail point registry for fail points.
    #[allow(dead_code)]
    fp_registry: Arc<FailPointRegistry>,
}

impl MessageHandlerExecutor {
    /// Creates a new [MessageHandlerExecutor].
    ///
    /// ## Arguments
    ///
    /// * `closed_result`: A [ClosedResultWriter] that stores the database close result.
    /// * `clock`: A [SystemClock] to use for tickers and timekeeping.
    ///
    /// ## Returns
    ///
    /// The new [MessageHandlerExecutor].
    pub(crate) fn new(
        closed_result: Arc<dyn ClosedResultWriter>,
        clock: Arc<dyn SystemClock>,
    ) -> Self {
        Self {
            futures: Mutex::new(Some(vec![])),
            closed_result,
            clock,
            tokens: SkipMap::new(),
            results: Arc::new(SkipMap::new()),
            fp_registry: Arc::new(FailPointRegistry::new()),
        }
    }

    /// Sets a custom [FailPointRegistry] for the executor.
    ///
    /// ## Arguments
    ///
    /// * `fp_registry`: A [FailPointRegistry] to use for fail points.
    ///
    /// ## Returns
    ///
    /// The [MessageHandlerExecutor] with the custom [FailPointRegistry].
    #[allow(dead_code)]
    pub(crate) fn with_fp_registry(mut self, fp_registry: Arc<FailPointRegistry>) -> Self {
        self.fp_registry = fp_registry;
        self
    }

    /// Adds a single [MessageHandler] as a group of one. See [MessageHandlerExecutor::add_handlers].
    pub(crate) fn add_handler<T: Send + std::fmt::Debug + 'static>(
        &self,
        name: String,
        handler: Box<dyn MessageHandler<T>>,
        rx: async_channel::Receiver<T>,
        handle: &Handle,
    ) -> Result<(), SlateDBError> {
        self.add_handlers(name, vec![handler], rx, handle)
    }

    /// Adds a group of [MessageHandler]s that share a single [async_channel::Receiver].
    /// Each handler runs its own [MessageDispatcher] event loop independently, enabling
    /// parallel message processing across the group.
    ///
    /// All handlers in the group share one [CancellationToken] and one result cell.
    /// A sequential handler is simply a group of size 1 (see [MessageHandlerExecutor::add_handler]).
    ///
    /// ## Arguments
    ///
    /// * `name`: The name of the handler group.
    /// * `handlers`: The [MessageHandler]s to run in parallel.
    /// * `rx`: The shared [async_channel::Receiver] for receiving messages.
    /// * `handle`: The [Handle] to use for spawning the dispatchers.
    ///
    /// ## Returns
    ///
    /// - `Ok(())` if the group was successfully added.
    /// - Err([SlateDBError::BackgroundTaskExists]) if a group with the same name already exists.
    /// - Err([SlateDBError::BackgroundTaskExecutorStarted]) if the executor is already started.
    pub(crate) fn add_handlers<T: Send + std::fmt::Debug + 'static>(
        &self,
        name: String,
        handlers: Vec<Box<dyn MessageHandler<T>>>,
        rx: async_channel::Receiver<T>,
        handle: &Handle,
    ) -> Result<(), SlateDBError> {
        assert!(!handlers.is_empty(), "handlers must not be empty");
        let token = CancellationToken::new();
        let mut futures = Vec::with_capacity(handlers.len());

        for (group_index, handler) in handlers.into_iter().enumerate() {
            let dispatcher =
                MessageDispatcher::new(handler, rx.clone(), self.clock.clone(), token.clone())
                    .with_fp_registry(self.fp_registry.clone());
            let future = dispatcher.run_lifecycle(
                name.clone(),
                self.closed_result.clone(),
                self.fp_registry.clone(),
            );
            futures.push(MessageHandlerFuture {
                name: name.clone(),
                group_index,
                future: Box::pin(future),
                token: token.clone(),
                handle: handle.clone(),
            });
        }

        let mut guard = self.futures.lock();
        if let Some(task_definitions) = guard.as_mut() {
            if task_definitions.iter().any(|t| t.name == name) {
                return Err(SlateDBError::BackgroundTaskExists(name));
            }
            task_definitions.extend(futures);
            Ok(())
        } else {
            Err(SlateDBError::BackgroundTaskExecutorStarted)
        }
    }

    /// Starts all [MessageHandlerFuture]s and spawns a task that monitors for completed
    /// dispatchers. As dispatchers complete, their results are stored in the executor's
    /// `results` map.
    ///
    /// ## Arguments
    ///
    /// * `handle`: The [Handle] to spawn the monitor on.
    ///
    /// ## Returns
    ///
    /// - Ok([JoinHandle]) if the monitor was successfully spawned.
    /// - Err([SlateDBError::BackgroundTaskExecutorStarted]) if the executor is already
    ///   started.
    pub(crate) fn monitor_on(&self, handle: &Handle) -> Result<JoinHandle<()>, SlateDBError> {
        let mut task_definitions = {
            let mut guard = self.futures.lock();
            if let Some(task_definitions) = guard.take() {
                task_definitions
            } else {
                return Err(SlateDBError::BackgroundTaskExecutorStarted);
            }
        };
        let mut tasks = JoinMap::new();
        let mut groups: HashMap<String, TaskGroup> = HashMap::new();
        for task_definition in task_definitions.drain(..) {
            // Only insert token and result cell once per group name
            if !self.tokens.contains_key(&task_definition.name) {
                self.tokens
                    .insert(task_definition.name.clone(), task_definition.token.clone());
                self.results
                    .insert(task_definition.name.clone(), WatchableOnceCell::new());
            }
            groups
                .entry(task_definition.name.clone())
                .or_insert_with(|| TaskGroup::new(0))
                .remaining += 1;
            tasks.spawn_on(
                (task_definition.name.clone(), task_definition.group_index),
                task_definition.future,
                &task_definition.handle,
            );
        }
        let monitor = TaskMonitor {
            tasks,
            groups,
            closed_result: self.closed_result.clone(),
            results: self.results.clone(),
            tokens: self.tokens.iter().map(|e| e.value().clone()).collect(),
        };
        Ok(handle.spawn(monitor.run()))
    }

    /// Drop registered but unstarted handlers, including their references back
    /// to this executor. Drop outside the mutex to avoid reentrant destruction.
    pub(crate) fn discard_pending_tasks(&self) {
        let pending = self.futures.lock().take();
        drop(pending);
    }

    /// Signal every running task before joining any one of them during failed open.
    pub(crate) fn cancel_all_tasks(&self) {
        for token in self.tokens.iter() {
            token.value().cancel();
        }
    }

    /// Cancels a task by name.
    ///
    /// ## Arguments
    ///
    /// * `name`: The name of the task to cancel.
    pub(crate) fn cancel_task(&self, name: &str) {
        if let Some(entry) = self.tokens.get(name) {
            entry.value().cancel();
        }
    }

    /// Waits for a task to complete.
    ///
    /// ## Arguments
    ///
    /// * `name`: The name of the task to wait for.
    ///
    /// ## Returns
    ///
    /// The result of the task.
    pub(crate) async fn join_task(&self, name: &str) -> Result<(), SlateDBError> {
        if let Some(entry) = self.results.get(name) {
            return entry.value().reader().await_value().await;
        }
        Ok(())
    }

    /// Cancels a running task and waits for it to complete.
    ///
    /// ## Arguments
    ///
    /// * `name`: The name of the task to cancel and wait for.
    ///
    /// ## Returns
    ///
    /// The result of the task.
    pub(crate) async fn shutdown_task(&self, name: &str) -> Result<(), SlateDBError> {
        self.cancel_task(name);
        self.join_task(name).await
    }

    /// Removes a task that may not have started yet, or cancels a running task and waits for it to
    /// complete.
    ///
    /// ## Arguments
    ///
    /// * `name`: The name of the task to cancel and wait for.
    ///
    /// ## Returns
    ///
    /// [`Some`] with the result of the task if it was started, [`None`] otherwise
    pub(crate) async fn shutdown_or_deregister_task(
        &self,
        name: &str,
    ) -> Option<Result<(), SlateDBError>> {
        {
            let mut guard = self.futures.lock();
            if let Some(task_definitions) = guard.as_mut() {
                if task_definitions.iter().any(|task| task.name == name) {
                    task_definitions.retain(|task| task.name != name);
                    return None;
                }
            }
        }
        Some(self.shutdown_task(name).await)
    }
}

#[cfg(all(test, feature = "test-util"))]
mod test {
    use super::{MessageDispatcher, MessageHandler};
    use crate::db_status::ClosedResultWriter;
    use crate::dispatcher::{MessageFactory, MessageHandlerExecutor, MessageTickerDef};
    use crate::error::SlateDBError;
    use crate::utils::WatchableOnceCell;
    use fail_parallel::FailPointRegistry;
    use futures::stream::BoxStream;
    use futures::StreamExt;
    use slatedb_common::clock::{DefaultSystemClock, MockSystemClock, SystemClock};
    use slatedb_common::DbRand;
    use std::collections::{HashSet, VecDeque};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::runtime::Handle;
    use tokio::task::yield_now;
    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum TestMessage {
        Channel(i32),
        Tick(i32),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Phase {
        Pre,
        Cleanup,
    }

    #[derive(Clone)]
    struct TestHandler {
        log: Arc<Mutex<Vec<(Phase, TestMessage)>>>,
        cleanup_called: WatchableOnceCell<Result<(), SlateDBError>>,
        tickers: Vec<(Duration, u8)>,
        clock: Arc<dyn SystemClock>,
        clock_schedule: VecDeque<Duration>,
        jitter: Option<(f64, Arc<DbRand>)>,
    }

    impl TestHandler {
        fn new(
            log: Arc<Mutex<Vec<(Phase, TestMessage)>>>,
            cleanup_called: WatchableOnceCell<Result<(), SlateDBError>>,
            clock: Arc<dyn SystemClock>,
        ) -> Self {
            Self {
                log,
                cleanup_called,
                tickers: vec![],
                clock,
                clock_schedule: VecDeque::new(),
                jitter: None,
            }
        }

        fn add_ticker(mut self, d: Duration, id: u8) -> Self {
            self.tickers.push((d, id));
            self
        }

        /// Add a clock schedule to the handler. The clock will pop the first duration from the
        /// schedule and advance the clock by that duration after each message is processed.
        fn add_clock_schedule(mut self, ts: u64) -> Self {
            self.clock_schedule.push_back(Duration::from_millis(ts));
            self
        }

        fn with_jitter(mut self, spread: f64, rand: Arc<DbRand>) -> Self {
            self.jitter = Some((spread, rand));
            self
        }
    }

    #[async_trait::async_trait]
    impl MessageHandler<TestMessage> for TestHandler {
        fn tickers(&mut self) -> Vec<MessageTickerDef<TestMessage>> {
            let mut tickers: Vec<MessageTickerDef<TestMessage>> = vec![];
            for (interval, id) in self.tickers.iter() {
                let id = *id as i32;
                let mut def =
                    MessageTickerDef::new(*interval, Box::new(move || TestMessage::Tick(id)));
                def.jitter = self.jitter.clone();
                tickers.push(def);
            }
            tickers
        }

        async fn handle(&mut self, message: TestMessage) -> Result<(), SlateDBError> {
            self.log.lock().unwrap().push((Phase::Pre, message));
            if let Some(advance_duration) = self.clock_schedule.pop_front() {
                self.clock.advance(advance_duration).await;
            }
            Ok(())
        }

        async fn cleanup(
            &mut self,
            mut messages: BoxStream<'async_trait, TestMessage>,
            result: Result<(), SlateDBError>,
        ) -> Result<(), SlateDBError> {
            self.cleanup_called.write(result);
            while let Some(m) = messages.next().await {
                self.log.lock().unwrap().push((Phase::Cleanup, m));
            }
            Ok(())
        }
    }

    async fn wait_for_message_count(log: Arc<Mutex<Vec<(Phase, TestMessage)>>>, count: usize) {
        timeout(Duration::from_secs(30), async move {
            while log.lock().unwrap().len() < count {
                yield_now().await;
            }
        })
        .await
        .expect("timeout waiting for message count");
    }

    // A jittered tick sleeps its randomized wait inside the ticker future, so it
    // must not block the dispatcher loop: channel messages keep flowing while the
    // tick is parked, and the tick itself only fires once the (mock) clock
    // advances past the wait. This is the property that motivated moving jitter
    // out of the handler (where the sleep blocked the channel) and into the
    // ticker.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_dispatcher_ticker_jitter_does_not_block_messages() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let (tx, rx) = async_channel::unbounded();
        let clock = Arc::new(MockSystemClock::new());
        // With spread 0.5 on a 100ms interval, every tick *after the first* waits
        // a random duration in [50ms, 150ms]. The first tick is always immediate
        // (it mirrors Tokio's `Interval`), so it fires right away; the dispatcher's
        // next tick future then waits a jittered [50ms, 150ms] that can't elapse
        // until the clock is advanced in the second phase.
        let handler = TestHandler::new(log.clone(), WatchableOnceCell::new(), clock.clone())
            .add_ticker(Duration::from_millis(100), 1)
            .with_jitter(0.5, Arc::new(DbRand::new(0)));
        let cancellation_token = CancellationToken::new();
        let mut dispatcher = MessageDispatcher::new(
            Box::new(handler),
            rx,
            clock.clone(),
            cancellation_token.clone(),
        );
        let join = tokio::spawn(async move { dispatcher.run().await });
        wait_for_message_count(log.clone(), 1).await;
        assert_eq!(
            log.lock().unwrap()[0],
            (Phase::Pre, TestMessage::Tick(1)),
            "the first tick should fire immediately"
        );
        tx.try_send(TestMessage::Channel(7)).unwrap();
        tx.try_send(TestMessage::Channel(8)).unwrap();
        tx.try_send(TestMessage::Channel(9)).unwrap();
        wait_for_message_count(log.clone(), 4).await;

        // All three channel messages are processed even though the first tick is
        // parked on its randomized wait the entire time (the clock never
        // advanced, so the jittered tick cannot fire). This is the non-blocking
        // property.
        assert_eq!(
            log.lock().unwrap().clone(),
            vec![
                (Phase::Pre, TestMessage::Tick(1)),
                (Phase::Pre, TestMessage::Channel(7)),
                (Phase::Pre, TestMessage::Channel(8)),
                (Phase::Pre, TestMessage::Channel(9)),
            ],
            "no tick but the first immediate one should fire until the clock advances past the jittered wait"
        );

        // Releasing the clock lets the jittered tick fire. The mock clock fixes
        // the wait's deadline (relative to the current time) when the ticker is
        // first polled, which races with this advance, so step the clock forward
        // repeatedly until the tick lands rather than advancing exactly once.
        timeout(Duration::from_secs(30), async {
            while log.lock().unwrap().len() < 5 {
                clock.advance(Duration::from_secs(1)).await;
                yield_now().await;
            }
        })
        .await
        .expect("jittered tick did not fire after advancing the clock");
        assert_eq!(
            log.lock().unwrap()[4],
            (Phase::Pre, TestMessage::Tick(1)),
            "the jittered tick should fire once the clock passes the jitter delay"
        );

        cancellation_token.cancel();
        let result = timeout(Duration::from_secs(30), join)
            .await
            .expect("dispatcher did not stop in time")
            .expect("join failed");
        assert!(matches!(result, Ok(())));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_dispatcher_run_happy_path() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let (tx, rx) = async_channel::unbounded();
        let clock = Arc::new(MockSystemClock::new());
        let handler = TestHandler::new(log.clone(), WatchableOnceCell::new(), clock.clone())
            .add_ticker(Duration::from_millis(5), 1)
            .add_clock_schedule(5); // Advance clock by 5ms after first message
        let cancellation_token = CancellationToken::new();
        let fp = Arc::new(FailPointRegistry::default());
        let mut dispatcher = MessageDispatcher::new(
            Box::new(handler),
            rx,
            clock.clone(),
            cancellation_token.clone(),
        )
        .with_fp_registry(fp.clone());

        // Pause the dispatcher run loop to prevent first ticker from firing immediately
        fail_parallel::cfg(fp.clone(), "dispatcher-run-loop", "pause").unwrap();

        // Start the dispatcher.
        let join = tokio::spawn(async move { dispatcher.run().await });

        // Send a message successfully, then trigger a tick before processing the next message
        tx.try_send(TestMessage::Channel(10)).unwrap();

        // Enable run loop to process first message, then the tick
        fail_parallel::cfg(fp.clone(), "dispatcher-run-loop", "off").unwrap();

        // Wait for both to be processed
        wait_for_message_count(log.clone(), 2).await;

        // Send another message successfully
        tx.try_send(TestMessage::Channel(20)).unwrap();
        wait_for_message_count(log.clone(), 3).await;

        // Cancel and wait for cleanup to start
        cancellation_token.cancel();

        // Ensure run() completes and returns clean shutdown
        let result = timeout(Duration::from_secs(30), join)
            .await
            .expect("dispatcher did not stop in time")
            .expect("join failed");

        // Verify final state
        assert!(matches!(result, Ok(())));
        let messages = log.lock().unwrap().clone();
        assert_eq!(
            messages,
            vec![
                (Phase::Pre, TestMessage::Channel(10)),
                (Phase::Pre, TestMessage::Tick(1)),
                (Phase::Pre, TestMessage::Channel(20))
            ]
        );
    }

    #[cfg(feature = "test-util")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_executor_propagates_handler_error_drains_messages() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let cleanup_called = WatchableOnceCell::new();
        let mut cleanup_reader = cleanup_called.reader();
        let (tx, rx) = async_channel::unbounded();
        let clock = Arc::new(MockSystemClock::new());
        let handler = TestHandler::new(log.clone(), cleanup_called.clone(), clock.clone());
        let closed_result = Arc::new(WatchableOnceCell::new());
        let fp_registry = Arc::new(FailPointRegistry::default());
        let task_executor = MessageHandlerExecutor::new(closed_result.clone(), clock.clone())
            .with_fp_registry(fp_registry.clone());
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "1*off->return").unwrap();
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-cleanup", "pause").unwrap();
        task_executor
            .add_handler(
                "test".to_string(),
                Box::new(handler),
                rx,
                &Handle::current(),
            )
            .expect("spawn failed");
        task_executor
            .monitor_on(&Handle::current())
            .expect("failed to monitor executor");

        // Send a message successfully.
        tx.try_send(TestMessage::Channel(42)).unwrap();
        // Wait for the first message to be processed.
        wait_for_message_count(log.clone(), 1).await;
        // Send another message. The panic is guaranteed to be triggered before this
        // message is processed since the `dispatcher-run-loop` failpoint is set to
        // `1*off->return`, one message has been processed, and the point is hit before
        // the message selector.
        tx.try_send(TestMessage::Channel(77)).unwrap();
        // Now allow cleanup to proceed.
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-cleanup", "off").unwrap();

        // Wait for cleanup to start (unclean shutdown)
        let _ = cleanup_reader.await_value().await;

        // Wait for the dispatcher to stop
        let result = timeout(Duration::from_secs(30), task_executor.join_task("test"))
            .await
            .expect("dispatcher did not stop in time");

        // Verify final state
        assert!(matches!(result, Err(SlateDBError::Fenced)));
        assert!(matches!(
            closed_result.result_reader().read(),
            Some(Err(SlateDBError::Fenced))
        ));
        assert!(matches!(
            cleanup_reader.read(),
            Some(Err(SlateDBError::Fenced))
        ));
        let messages = log.lock().unwrap().clone();
        assert_eq!(
            messages,
            vec![
                (Phase::Pre, TestMessage::Channel(42)),
                (Phase::Cleanup, TestMessage::Channel(77)),
            ]
        );
    }

    #[tokio::test]
    async fn test_shutdown_task_removes_handler_before_monitor_starts() {
        let clock = Arc::new(DefaultSystemClock::new());
        let closed_result = Arc::new(WatchableOnceCell::new());
        let task_executor = MessageHandlerExecutor::new(closed_result.clone(), clock.clone());
        let removed_cleanup = WatchableOnceCell::new();
        let retained_cleanup = WatchableOnceCell::new();
        let (removed_tx, removed_rx) = async_channel::unbounded();
        let (_retained_tx, retained_rx) = async_channel::unbounded();

        task_executor
            .add_handler(
                "removed".to_string(),
                Box::new(TestHandler::new(
                    Arc::new(Mutex::new(Vec::new())),
                    removed_cleanup.clone(),
                    clock.clone(),
                )),
                removed_rx,
                &Handle::current(),
            )
            .unwrap();
        task_executor
            .add_handler(
                "retained".to_string(),
                Box::new(TestHandler::new(
                    Arc::new(Mutex::new(Vec::new())),
                    retained_cleanup.clone(),
                    clock,
                )),
                retained_rx,
                &Handle::current(),
            )
            .unwrap();

        assert!(task_executor
            .shutdown_or_deregister_task("removed")
            .await
            .is_none());
        assert!(removed_tx.is_closed());
        assert!(removed_cleanup.result_reader().read().is_none());

        let monitor = task_executor.monitor_on(&Handle::current()).unwrap();
        assert!(!task_executor.tokens.contains_key("removed"));
        assert!(task_executor.tokens.contains_key("retained"));

        task_executor
            .shutdown_or_deregister_task("retained")
            .await
            .unwrap()
            .unwrap();
        monitor.await.unwrap();
        assert!(retained_cleanup.result_reader().read().is_some());
        assert!(closed_result.result_reader().read().is_some());
    }

    #[tokio::test]
    async fn test_shutdown_task_cancels_when_pending_task_not_found() {
        let task_executor = MessageHandlerExecutor::new(
            Arc::new(WatchableOnceCell::new()),
            Arc::new(DefaultSystemClock::new()),
        );
        let running_token = CancellationToken::new();
        let running_result = WatchableOnceCell::new();
        running_result.write(Ok(()));
        task_executor
            .tokens
            .insert("running".to_string(), running_token.clone());
        task_executor
            .results
            .insert("running".to_string(), running_result);

        task_executor.shutdown_task("running").await.unwrap();

        assert!(running_token.is_cancelled());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_dispatcher_prioritizes_messages_over_tickers() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let (tx, rx) = async_channel::unbounded();
        let clock = Arc::new(MockSystemClock::new());
        let handler = TestHandler::new(log.clone(), WatchableOnceCell::new(), clock.clone())
            .add_ticker(Duration::from_millis(5), 1);
        let cancellation_token = CancellationToken::new();
        let fp_registry = Arc::new(FailPointRegistry::default());
        let mut dispatcher = MessageDispatcher::new(
            Box::new(handler),
            rx,
            clock.clone(),
            cancellation_token.clone(),
        )
        .with_fp_registry(fp_registry.clone());
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "pause").unwrap();
        let join = tokio::spawn(async move { dispatcher.run().await });

        // Trigger a tick and a message
        clock.advance(Duration::from_millis(5)).await;
        tx.try_send(TestMessage::Channel(99)).unwrap();
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "off").unwrap();
        wait_for_message_count(log.clone(), 2).await;

        // Shutdown cleanly
        cancellation_token.cancel();
        let result = timeout(Duration::from_secs(30), join)
            .await
            .expect("dispatcher did not stop in time")
            .expect("join failed");

        // Verify final state
        assert!(matches!(result, Ok(())));
        let messages = log.lock().unwrap().clone();
        assert_eq!(
            messages,
            vec![
                (Phase::Pre, TestMessage::Channel(99)),
                (Phase::Pre, TestMessage::Tick(1)),
            ]
        );
    }

    // This test simulates the following timeline:
    // immediate tick(3) (tickers always return first ticks immediately)
    // immediate tick(5)
    // advance clock to 5ms
    // tick(5)
    // advance clock to 7ms
    // tick(7)
    // advance clock to 10ms
    // tick(5)
    // advance clock to 14ms
    // tick(7)
    // advance clock to 15ms
    // tick(5)
    // advance clock to 20ms
    // tick(5)
    // advance clock to 21ms
    // tick(7)
    #[cfg(feature = "test-util")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_dispatcher_supports_multiple_tickers() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let (_tx, rx) = async_channel::unbounded::<TestMessage>();
        let clock = Arc::new(MockSystemClock::new());
        let handler = TestHandler::new(log.clone(), WatchableOnceCell::new(), clock.clone())
            .add_ticker(Duration::from_millis(5), 1)
            .add_ticker(Duration::from_millis(7), 2)
            .add_clock_schedule(0) // 0ms (initial two ticks are immediate, so don't advance clock until second tick)
            .add_clock_schedule(5) // 5ms
            .add_clock_schedule(2) // 7ms
            .add_clock_schedule(3) // 10ms
            .add_clock_schedule(4) // 14ms
            .add_clock_schedule(1) // 15ms
            .add_clock_schedule(5) // 20ms
            .add_clock_schedule(1); // 21ms
        let cancellation_token = CancellationToken::new();
        let fp_registry = Arc::new(FailPointRegistry::default());
        let mut dispatcher = MessageDispatcher::new(
            Box::new(handler),
            rx,
            clock.clone(),
            cancellation_token.clone(),
        )
        .with_fp_registry(fp_registry.clone());
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "pause").unwrap();
        let join = tokio::spawn(async move { dispatcher.run().await });
        assert_eq!(log.lock().unwrap().len(), 0);
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "off").unwrap();

        wait_for_message_count(log.clone(), 9).await;
        assert_eq!(
            log.lock().unwrap().clone(),
            vec![
                (Phase::Pre, TestMessage::Tick(1)), // immediate tick
                (Phase::Pre, TestMessage::Tick(2)), // immediate tick
                (Phase::Pre, TestMessage::Tick(1)), // 5
                (Phase::Pre, TestMessage::Tick(2)), // 7
                (Phase::Pre, TestMessage::Tick(1)), // 10
                (Phase::Pre, TestMessage::Tick(2)), // 14
                (Phase::Pre, TestMessage::Tick(1)), // 15
                (Phase::Pre, TestMessage::Tick(1)), // 20
                (Phase::Pre, TestMessage::Tick(2)), // 21
            ]
        );

        // Shutdown and wait for cleanup to start
        cancellation_token.cancel();

        let result = timeout(Duration::from_secs(30), join)
            .await
            .expect("dispatcher did not stop in time")
            .expect("join failed");

        // Verify final state
        assert!(matches!(result, Ok(())));
        assert_eq!(log.lock().unwrap().len(), 9);
    }

    // This test simulates the following timeline:
    // immediate tick(3) (tickers always return first ticks immediately)
    // immediate tick(5)
    // advance clock to 3ms
    // tick(3)
    // advance clock to 5ms
    // tick(5)
    // advance clock to 6ms
    // tick(3)
    // advance clock to 9ms
    // tick(3)
    // advance clock to 10ms
    // tick(5)
    // advance clock to 12ms
    // tick(3)
    // clock = 15ms: tick(3), tick(5)
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_dispatcher_supports_overlapping_tickers() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let (_tx, rx) = async_channel::unbounded::<TestMessage>();
        let clock = Arc::new(MockSystemClock::new());
        let handler = TestHandler::new(log.clone(), WatchableOnceCell::new(), clock.clone())
            .add_ticker(Duration::from_millis(3), 3)
            .add_ticker(Duration::from_millis(5), 5)
            .add_clock_schedule(0) // 0 (initial two ticks are immediate, so don't advance clock until second tick)
            .add_clock_schedule(3) // 3
            .add_clock_schedule(2) // 5
            .add_clock_schedule(1) // 6
            .add_clock_schedule(3) // 9
            .add_clock_schedule(1) // 10
            .add_clock_schedule(2) // 12
            .add_clock_schedule(3); // 15
        let cancellation_token = CancellationToken::new();
        let fp_registry = Arc::new(FailPointRegistry::default());
        let mut dispatcher = MessageDispatcher::new(
            Box::new(handler),
            rx,
            clock.clone(),
            cancellation_token.clone(),
        )
        .with_fp_registry(fp_registry.clone());
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "pause").unwrap();
        let join = tokio::spawn(async move { dispatcher.run().await });
        assert_eq!(log.lock().unwrap().len(), 0);
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "off").unwrap();

        wait_for_message_count(log.clone(), 10).await;
        assert_eq!(
            log.lock().unwrap().clone()[..8],
            vec![
                (Phase::Pre, TestMessage::Tick(3)), // immediate tick
                (Phase::Pre, TestMessage::Tick(5)), // immediate tick
                (Phase::Pre, TestMessage::Tick(3)), // 3
                (Phase::Pre, TestMessage::Tick(5)), // 5
                (Phase::Pre, TestMessage::Tick(3)), // 6
                (Phase::Pre, TestMessage::Tick(3)), // 9
                (Phase::Pre, TestMessage::Tick(5)), // 10
                (Phase::Pre, TestMessage::Tick(3)), // 12
            ]
        );
        // expect two back-to-back ticks at 15
        let mut last_two_ticks = log.lock().unwrap().clone()[8..].to_vec();
        last_two_ticks.sort_by(|a, b| match (a.1.clone(), b.1.clone()) {
            (TestMessage::Tick(a), TestMessage::Tick(b)) => a.cmp(&b),
            _ => panic!("expected ticks"),
        });
        assert_eq!(
            last_two_ticks,
            vec![
                (Phase::Pre, TestMessage::Tick(3)),
                (Phase::Pre, TestMessage::Tick(5))
            ]
        );

        // Shutdown cleanly
        cancellation_token.cancel();
        let result = timeout(Duration::from_secs(30), join)
            .await
            .expect("dispatcher did not stop in time")
            .expect("join failed");
        assert!(matches!(result, Ok(())));
        assert_eq!(log.lock().unwrap().len(), 10);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_dispatcher_supports_identical_tickers() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let (_tx, rx) = async_channel::unbounded::<TestMessage>();
        let clock = Arc::new(MockSystemClock::new());
        let handler = TestHandler::new(log.clone(), WatchableOnceCell::new(), clock.clone())
            .add_ticker(Duration::from_millis(3), 1)
            .add_ticker(Duration::from_millis(3), 2)
            .add_clock_schedule(0) // 0 (initial two ticks are immediate, so don't advance clock until second tick)
            .add_clock_schedule(3) // 3
            .add_clock_schedule(0) // 3 (process second)
            .add_clock_schedule(3) // 6
            .add_clock_schedule(0) // 6 (process second)
            .add_clock_schedule(3) // 9
            .add_clock_schedule(0) // 9 (process second)
            .add_clock_schedule(3) // 12
            .add_clock_schedule(0) // 12 (process second)
            .add_clock_schedule(3) // 15
            .add_clock_schedule(0) // 15 (process second)
            .add_clock_schedule(3) // 18
            .add_clock_schedule(0) // 18 (process second)
            .add_clock_schedule(3) // 21
            .add_clock_schedule(0); // 21 (process second)
        let cancellation_token = CancellationToken::new();
        let fp_registry = Arc::new(FailPointRegistry::default());
        let mut dispatcher = MessageDispatcher::new(
            Box::new(handler),
            rx,
            clock.clone(),
            cancellation_token.clone(),
        )
        .with_fp_registry(fp_registry.clone());
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "pause").unwrap();
        let join = tokio::spawn(async move { dispatcher.run().await });
        assert_eq!(log.lock().unwrap().len(), 0);
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "off").unwrap();

        wait_for_message_count(log.clone(), 16).await;
        assert_eq!(
            log.lock().unwrap().clone(),
            vec![
                (Phase::Pre, TestMessage::Tick(1)), // immediate tick
                (Phase::Pre, TestMessage::Tick(2)), // immediate tick
                (Phase::Pre, TestMessage::Tick(1)), // 3
                (Phase::Pre, TestMessage::Tick(2)), // 3
                (Phase::Pre, TestMessage::Tick(1)), // 6
                (Phase::Pre, TestMessage::Tick(2)), // 6
                (Phase::Pre, TestMessage::Tick(1)), // 9
                (Phase::Pre, TestMessage::Tick(2)), // 9
                (Phase::Pre, TestMessage::Tick(1)), // 12
                (Phase::Pre, TestMessage::Tick(2)), // 12
                (Phase::Pre, TestMessage::Tick(1)), // 15
                (Phase::Pre, TestMessage::Tick(2)), // 15
                (Phase::Pre, TestMessage::Tick(1)), // 18
                (Phase::Pre, TestMessage::Tick(2)), // 18
                (Phase::Pre, TestMessage::Tick(1)), // 21
                (Phase::Pre, TestMessage::Tick(2)), // 21
            ]
        );

        // Shutdown cleanly
        cancellation_token.cancel();
        let result = timeout(Duration::from_secs(30), join)
            .await
            .expect("dispatcher did not stop in time")
            .expect("join failed");
        assert!(matches!(result, Ok(())));
        assert_eq!(log.lock().unwrap().len(), 16);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_executor_catches_panic_from_run_loop() {
        #[derive(Clone)]
        struct PanicHandler {
            cleanup_called: WatchableOnceCell<Result<(), SlateDBError>>,
        }

        #[async_trait::async_trait]
        impl MessageHandler<u8> for PanicHandler {
            fn tickers(&mut self) -> Vec<MessageTickerDef<u8>> {
                vec![]
            }

            async fn handle(&mut self, _message: u8) -> Result<(), SlateDBError> {
                panic!("intentional panic in handler");
            }

            async fn cleanup(
                &mut self,
                mut messages: BoxStream<'async_trait, u8>,
                result: Result<(), SlateDBError>,
            ) -> Result<(), SlateDBError> {
                // Record the shutdown result and drain any pending messages
                self.cleanup_called.write(result);
                while messages.next().await.is_some() {}
                Ok(())
            }
        }

        let cleanup_called = WatchableOnceCell::new();
        let mut cleanup_reader = cleanup_called.reader();
        let (tx, rx) = async_channel::unbounded::<u8>();
        let clock = Arc::new(DefaultSystemClock::new());
        let handler = PanicHandler { cleanup_called };
        let closed_result = Arc::new(WatchableOnceCell::new());
        let task_executor = MessageHandlerExecutor::new(closed_result.clone(), clock.clone());

        // Spawn the panic task
        task_executor
            .add_handler(
                "test".to_string(),
                Box::new(handler),
                rx,
                &Handle::current(),
            )
            .expect("failed to spawn task");

        // Monitor the executor
        task_executor
            .monitor_on(&Handle::current())
            .expect("failed to monitor executor");

        // Trigger panic inside the run loop via handle()
        tx.try_send(1u8).unwrap();

        // Wait for cleanup to observe the panic result
        let _ = timeout(Duration::from_secs(30), cleanup_reader.await_value())
            .await
            .expect("timeout waiting for cleanup result");

        // Join dispatcher and verify panic was converted and propagated
        let result = timeout(Duration::from_secs(30), task_executor.join_task("test"))
            .await
            .expect("dispatcher did not stop in time");

        // Check final state
        assert!(matches!(result, Err(SlateDBError::BackgroundTaskPanic(_))));
        assert!(matches!(
            closed_result.result_reader().read(),
            Some(Err(SlateDBError::BackgroundTaskPanic(_)))
        ));
        assert!(matches!(
            cleanup_reader.read(),
            Some(Err(SlateDBError::BackgroundTaskPanic(_)))
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_executor_panic_in_wrapper() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let cleanup_called = WatchableOnceCell::new();
        let cleanup_reader = cleanup_called.reader();
        let (tx, rx) = async_channel::unbounded::<TestMessage>();
        drop(tx); // no messages needed
        let clock = Arc::new(DefaultSystemClock::new());
        let handler = TestHandler::new(log.clone(), cleanup_called.clone(), clock.clone());
        let closed_result = Arc::new(WatchableOnceCell::new());
        let fp_registry = Arc::new(FailPointRegistry::default());
        let task_executor = MessageHandlerExecutor::new(closed_result.clone(), clock.clone())
            .with_fp_registry(fp_registry.clone());

        fail_parallel::cfg(
            fp_registry.clone(),
            "executor-wrapper-before-write",
            "panic",
        )
        .unwrap();

        // Spawn the task and monitor
        task_executor
            .add_handler(
                "test".to_string(),
                Box::new(handler),
                rx,
                &Handle::current(),
            )
            .expect("failed to spawn task");
        task_executor
            .monitor_on(&Handle::current())
            .expect("failed to monitor executor");

        // Cancel to exit run() cleanly, then `async move`` wrapper panics at the fail point
        task_executor.cancel_task("test");

        // Wait for task result
        let result = timeout(Duration::from_secs(30), task_executor.join_task("test"))
            .await
            .expect("dispatcher did not stop in time");

        // Assertions: panic result, closed_result set to panic, cleanup not called
        assert!(matches!(result, Err(SlateDBError::BackgroundTaskPanic(_))));
        assert!(matches!(
            closed_result.result_reader().read(),
            Some(Err(SlateDBError::BackgroundTaskPanic(_)))
        ));
        assert!(cleanup_reader.read().is_none());
    }

    // -- Parallel handler test infrastructure --

    /// A handler that records which handler_id processed each message.
    /// Optionally blocks in handle() until a Notify is signaled, which lets
    /// tests force messages to go to the *other* handler.
    struct ParallelTestHandler {
        handler_id: u8,
        /// (handler_id, message) log shared across all handlers in the group.
        log: Arc<Mutex<Vec<(u8, TestMessage)>>>,
        cleanup_called: WatchableOnceCell<Result<(), SlateDBError>>,
        /// If set, the handler waits on this Notify for every message.
        block_on: Option<Arc<tokio::sync::Notify>>,
        /// If set, the handler returns an error after handling this many messages.
        error_after: Option<usize>,
        handled_count: usize,
        tickers: Vec<(Duration, u8)>,
    }

    impl ParallelTestHandler {
        fn new(
            handler_id: u8,
            log: Arc<Mutex<Vec<(u8, TestMessage)>>>,
            cleanup_called: WatchableOnceCell<Result<(), SlateDBError>>,
        ) -> Self {
            Self {
                handler_id,
                log,
                cleanup_called,
                block_on: None,
                error_after: None,
                handled_count: 0,
                tickers: vec![],
            }
        }

        fn with_block_on(mut self, notify: Arc<tokio::sync::Notify>) -> Self {
            self.block_on = Some(notify);
            self
        }

        fn with_error_after(mut self, n: usize) -> Self {
            self.error_after = Some(n);
            self
        }

        fn add_ticker(mut self, d: Duration, id: u8) -> Self {
            self.tickers.push((d, id));
            self
        }
    }

    #[async_trait::async_trait]
    impl MessageHandler<TestMessage> for ParallelTestHandler {
        fn tickers(&mut self) -> Vec<MessageTickerDef<TestMessage>> {
            self.tickers
                .iter()
                .map(|(interval, id)| {
                    let id = *id as i32;
                    MessageTickerDef::new(
                        *interval,
                        Box::new(move || TestMessage::Tick(id)) as Box<MessageFactory<TestMessage>>,
                    )
                })
                .collect()
        }

        async fn handle(&mut self, message: TestMessage) -> Result<(), SlateDBError> {
            self.log.lock().unwrap().push((self.handler_id, message));
            if let Some(notify) = &self.block_on {
                notify.notified().await;
            }
            self.handled_count += 1;
            if let Some(n) = self.error_after {
                if self.handled_count > n {
                    return Err(SlateDBError::Fenced);
                }
            }
            Ok(())
        }

        async fn cleanup(
            &mut self,
            mut messages: BoxStream<'async_trait, TestMessage>,
            result: Result<(), SlateDBError>,
        ) -> Result<(), SlateDBError> {
            self.cleanup_called.write(result);
            while let Some(m) = messages.next().await {
                self.log.lock().unwrap().push((self.handler_id, m));
            }
            Ok(())
        }
    }

    /// Wait until the parallel log has at least `count` entries.
    async fn wait_for_parallel_log_count(log: Arc<Mutex<Vec<(u8, TestMessage)>>>, count: usize) {
        timeout(Duration::from_secs(30), async move {
            while log.lock().unwrap().len() < count {
                yield_now().await;
            }
        })
        .await
        .expect("timeout waiting for parallel log count");
    }

    // -- Parallel handler tests --

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_parallel_handlers_both_members_process_messages() {
        // Strategy: both handlers block after receiving a message. Send two
        // messages and wait for both to appear in the log. Since each handler
        // blocks after its first message, both must be occupied — proving the
        // channel delivered to two distinct handlers.
        let log = Arc::new(Mutex::new(Vec::<(u8, TestMessage)>::new()));
        let (tx, rx) = async_channel::unbounded();
        let clock = Arc::new(DefaultSystemClock::new());
        let block1 = Arc::new(tokio::sync::Notify::new());
        let block2 = Arc::new(tokio::sync::Notify::new());

        let handler1 = ParallelTestHandler::new(1, log.clone(), WatchableOnceCell::new())
            .with_block_on(block1.clone());
        let handler2 = ParallelTestHandler::new(2, log.clone(), WatchableOnceCell::new())
            .with_block_on(block2.clone());

        let closed_result = Arc::new(WatchableOnceCell::new());
        let task_executor = MessageHandlerExecutor::new(closed_result, clock);
        task_executor
            .add_handlers(
                "parallel".to_string(),
                vec![Box::new(handler1), Box::new(handler2)],
                rx,
                &Handle::current(),
            )
            .expect("failed to add handlers");
        task_executor.monitor_on(&Handle::current()).unwrap();

        // Send two messages. Both handlers will each grab one and block.
        tx.try_send(TestMessage::Channel(1)).unwrap();
        tx.try_send(TestMessage::Channel(2)).unwrap();
        wait_for_parallel_log_count(log.clone(), 2).await;

        // Verify both handlers processed a message
        let entries = log.lock().unwrap().clone();
        let handler_ids: HashSet<u8> = entries.iter().map(|(id, _)| *id).collect();
        assert!(
            handler_ids.contains(&1) && handler_ids.contains(&2),
            "expected both handlers to process messages, got: {:?}",
            entries,
        );

        // Unblock both and shut down. Use notify_one so a permit is retained
        // if a handler has logged but has not yet reached notified().await.
        block1.notify_one();
        block2.notify_one();
        let result = timeout(
            Duration::from_secs(5),
            task_executor.shutdown_task("parallel"),
        )
        .await
        .expect("timeout waiting for shutdown after unblock");
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_parallel_handlers_independent_tickers() {
        // Each handler has its own ticker. Verify both fire.
        let log = Arc::new(Mutex::new(Vec::<(u8, TestMessage)>::new()));
        let (_tx, rx) = async_channel::unbounded::<TestMessage>();
        let clock = Arc::new(DefaultSystemClock::new());

        // Handler 1 ticks with id=10, handler 2 ticks with id=20
        let handler1 = ParallelTestHandler::new(1, log.clone(), WatchableOnceCell::new())
            .add_ticker(Duration::from_millis(1), 10);
        let handler2 = ParallelTestHandler::new(2, log.clone(), WatchableOnceCell::new())
            .add_ticker(Duration::from_millis(1), 20);

        let closed_result = Arc::new(WatchableOnceCell::new());
        let task_executor = MessageHandlerExecutor::new(closed_result, clock);
        task_executor
            .add_handlers(
                "parallel".to_string(),
                vec![Box::new(handler1), Box::new(handler2)],
                rx,
                &Handle::current(),
            )
            .expect("failed to add handlers");
        task_executor.monitor_on(&Handle::current()).unwrap();

        // Wait for at least 4 ticks (2 initial immediate ticks + 2 more)
        wait_for_parallel_log_count(log.clone(), 4).await;

        let entries = log.lock().unwrap().clone();
        let h1_ticks: Vec<_> = entries
            .iter()
            .filter(|(id, _)| *id == 1)
            .map(|(_, m)| m.clone())
            .collect();
        let h2_ticks: Vec<_> = entries
            .iter()
            .filter(|(id, _)| *id == 2)
            .map(|(_, m)| m.clone())
            .collect();
        assert!(
            h1_ticks.iter().all(|m| *m == TestMessage::Tick(10)),
            "handler 1 should only see tick(10), got: {:?}",
            h1_ticks,
        );
        assert!(
            h2_ticks.iter().all(|m| *m == TestMessage::Tick(20)),
            "handler 2 should only see tick(20), got: {:?}",
            h2_ticks,
        );
        assert!(!h1_ticks.is_empty(), "handler 1 should have ticked");
        assert!(!h2_ticks.is_empty(), "handler 2 should have ticked");

        task_executor.shutdown_task("parallel").await.ok();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_parallel_handlers_error_cancels_sibling_and_runs_cleanup() {
        let log = Arc::new(Mutex::new(Vec::<(u8, TestMessage)>::new()));
        let (tx, rx) = async_channel::unbounded::<TestMessage>();
        let clock = Arc::new(DefaultSystemClock::new());

        let cleanup1 = WatchableOnceCell::new();
        let cleanup2 = WatchableOnceCell::new();
        let mut cleanup1_reader = cleanup1.reader();
        let mut cleanup2_reader = cleanup2.reader();

        let handler1 = ParallelTestHandler::new(1, log.clone(), cleanup1).with_error_after(1);
        let handler2 = ParallelTestHandler::new(2, log.clone(), cleanup2).with_error_after(1);

        let closed_result = Arc::new(WatchableOnceCell::new());
        let task_executor = MessageHandlerExecutor::new(closed_result.clone(), clock);
        task_executor
            .add_handlers(
                "parallel".to_string(),
                vec![Box::new(handler1), Box::new(handler2)],
                rx,
                &Handle::current(),
            )
            .expect("failed to add handlers");
        task_executor.monitor_on(&Handle::current()).unwrap();

        // Send 3 messages so that one of the handlers will see 2 messages and
        // trigger the expected error.
        tx.try_send(TestMessage::Channel(1)).unwrap();
        tx.try_send(TestMessage::Channel(2)).unwrap();
        // The third send is needed to guarantee that at least one channel sees
        // two messages, but it may fail if the same channel handled both messages above.
        let _ = tx.try_send(TestMessage::Channel(3));

        // Wait for both cleanups
        let _ = timeout(Duration::from_secs(5), cleanup1_reader.await_value())
            .await
            .expect("timeout waiting for handler 1 cleanup");
        let _ = timeout(Duration::from_secs(5), cleanup2_reader.await_value())
            .await
            .expect("timeout waiting for handler 2 cleanup");

        let result = timeout(Duration::from_secs(5), task_executor.join_task("parallel"))
            .await
            .expect("timeout waiting for join");

        assert!(matches!(result, Err(SlateDBError::Fenced)));
        assert!(matches!(
            closed_result.result_reader().read(),
            Some(Err(SlateDBError::Fenced))
        ));
        // Both handlers ran cleanup
        assert!(cleanup1_reader.read().is_some());
        assert!(cleanup2_reader.read().is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_parallel_handlers_join_waits_for_all_members() {
        // Both handlers block. Send two messages so each grabs one. Cancel the
        // group. Verify join_task does NOT resolve while handlers are blocked.
        // Then unblock and verify it resolves.
        let log = Arc::new(Mutex::new(Vec::<(u8, TestMessage)>::new()));
        let (tx, rx) = async_channel::unbounded();
        let clock = Arc::new(DefaultSystemClock::new());
        let block1 = Arc::new(tokio::sync::Notify::new());
        let block2 = Arc::new(tokio::sync::Notify::new());

        let handler1 = ParallelTestHandler::new(1, log.clone(), WatchableOnceCell::new())
            .with_block_on(block1.clone());
        let handler2 = ParallelTestHandler::new(2, log.clone(), WatchableOnceCell::new())
            .with_block_on(block2.clone());

        let closed_result = Arc::new(WatchableOnceCell::new());
        let task_executor = Arc::new(MessageHandlerExecutor::new(closed_result, clock));
        task_executor
            .add_handlers(
                "parallel".to_string(),
                vec![Box::new(handler1), Box::new(handler2)],
                rx,
                &Handle::current(),
            )
            .expect("failed to add handlers");
        task_executor.monitor_on(&Handle::current()).unwrap();

        // Send two messages so both handlers each grab one and block
        tx.try_send(TestMessage::Channel(1)).unwrap();
        tx.try_send(TestMessage::Channel(2)).unwrap();
        wait_for_parallel_log_count(log.clone(), 2).await;

        // Cancel the group
        task_executor.cancel_task("parallel");

        // join_task should NOT resolve yet because both handlers are blocked
        let join_result = timeout(
            Duration::from_millis(100),
            task_executor.join_task("parallel"),
        )
        .await;
        assert!(
            join_result.is_err(),
            "join_task should not resolve while members are still blocked"
        );

        // Unblock both. notify_one is safe even if a handler has logged but
        // has not yet reached notified().await.
        block1.notify_one();
        block2.notify_one();
        let result = timeout(Duration::from_secs(5), task_executor.join_task("parallel"))
            .await
            .expect("timeout waiting for join after unblock");
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_parallel_handlers_cleanup_drains_messages() {
        // Pause both handlers so messages queue up, then cancel and verify
        // all messages are drained during cleanup across the two handlers.
        let log = Arc::new(Mutex::new(Vec::<(u8, TestMessage)>::new()));
        let (tx, rx) = async_channel::unbounded();
        let clock = Arc::new(DefaultSystemClock::new());
        let fp_registry = Arc::new(FailPointRegistry::default());

        let handler1 = ParallelTestHandler::new(1, log.clone(), WatchableOnceCell::new());
        let handler2 = ParallelTestHandler::new(2, log.clone(), WatchableOnceCell::new());

        let closed_result = Arc::new(WatchableOnceCell::new());
        let task_executor =
            MessageHandlerExecutor::new(closed_result, clock).with_fp_registry(fp_registry.clone());
        // Pause the run loop so messages queue up
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "pause").unwrap();
        task_executor
            .add_handlers(
                "parallel".to_string(),
                vec![Box::new(handler1), Box::new(handler2)],
                rx,
                &Handle::current(),
            )
            .expect("failed to add handlers");
        task_executor.monitor_on(&Handle::current()).unwrap();

        // Queue up messages while paused
        for i in 1..=5 {
            tx.try_send(TestMessage::Channel(i)).unwrap();
        }

        // Unpause and cancel — messages should be drained during cleanup
        fail_parallel::cfg(fp_registry.clone(), "dispatcher-run-loop", "off").unwrap();
        task_executor.cancel_task("parallel");

        let result = timeout(Duration::from_secs(5), task_executor.join_task("parallel"))
            .await
            .expect("timeout waiting for join");
        assert!(result.is_ok());

        // All 5 messages should have been processed (some in Pre, some in Cleanup)
        let entries = log.lock().unwrap().clone();
        let mut values: Vec<i32> = entries
            .iter()
            .map(|(_, m)| match m {
                TestMessage::Channel(v) => *v,
                _ => panic!("unexpected tick"),
            })
            .collect();
        values.sort();
        assert_eq!(values, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_parallel_group_coexists_with_sequential_task() {
        // A parallel group ("parallel") and a sequential task ("sequential")
        // in the same executor. Both should function independently.
        let parallel_log = Arc::new(Mutex::new(Vec::<(u8, TestMessage)>::new()));
        let sequential_log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let (par_tx, par_rx) = async_channel::unbounded();
        let (seq_tx, seq_rx) = async_channel::unbounded();
        let clock = Arc::new(DefaultSystemClock::new());

        let handler1 = ParallelTestHandler::new(1, parallel_log.clone(), WatchableOnceCell::new());
        let handler2 = ParallelTestHandler::new(2, parallel_log.clone(), WatchableOnceCell::new());
        let seq_handler = TestHandler::new(
            sequential_log.clone(),
            WatchableOnceCell::new(),
            clock.clone(),
        );

        let closed_result = Arc::new(WatchableOnceCell::new());
        let task_executor = MessageHandlerExecutor::new(closed_result, clock);
        task_executor
            .add_handlers(
                "parallel".to_string(),
                vec![Box::new(handler1), Box::new(handler2)],
                par_rx,
                &Handle::current(),
            )
            .expect("failed to add parallel handlers");
        task_executor
            .add_handler(
                "sequential".to_string(),
                Box::new(seq_handler),
                seq_rx,
                &Handle::current(),
            )
            .expect("failed to add sequential handler");
        task_executor.monitor_on(&Handle::current()).unwrap();

        // Send messages to both task types concurrently
        par_tx.try_send(TestMessage::Channel(1)).unwrap();
        par_tx.try_send(TestMessage::Channel(2)).unwrap();
        seq_tx.try_send(TestMessage::Channel(100)).unwrap();

        wait_for_parallel_log_count(parallel_log.clone(), 2).await;
        wait_for_message_count(sequential_log.clone(), 1).await;

        // Verify parallel group processed both messages (order and handler don't matter)
        let par_entries = parallel_log.lock().unwrap().clone();
        let mut par_values: Vec<i32> = par_entries
            .iter()
            .map(|(_, m)| match m {
                TestMessage::Channel(v) => *v,
                _ => panic!("unexpected tick"),
            })
            .collect();
        par_values.sort();
        assert_eq!(par_values, vec![1, 2]);

        // Verify sequential task processed its message
        let seq_entries = sequential_log.lock().unwrap().clone();
        assert_eq!(seq_entries.len(), 1);
        assert_eq!(seq_entries[0], (Phase::Pre, TestMessage::Channel(100)));

        task_executor.shutdown_task("parallel").await.ok();
        task_executor.shutdown_task("sequential").await.ok();
    }

    /// Adapts a `watch::Receiver<u64>` into a [Notifier] that produces
    /// `WatchMessage::WatermarkAdvanced` on each change.
    struct WatermarkNotifier {
        rx: tokio::sync::watch::Receiver<u64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum WatchMessage {
        Work(u64),
        WatermarkAdvanced,
    }

    #[async_trait::async_trait]
    impl super::Notifier<WatchMessage> for WatermarkNotifier {
        async fn notify(&mut self) -> WatchMessage {
            if self.rx.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
            WatchMessage::WatermarkAdvanced
        }
    }

    /// A handler that uses a notifier to react to watermark changes,
    /// similar to how the manifest writer waits for the WAL durable sequence.
    struct WatchHandler {
        log: Arc<Mutex<Vec<(Phase, TestMessage)>>>,
        cleanup_called: WatchableOnceCell<Result<(), SlateDBError>>,
        watermark_rx: tokio::sync::watch::Receiver<u64>,
        /// Pending work waiting for the watermark to catch up.
        blocked_seq: Option<u64>,
    }

    #[async_trait::async_trait]
    impl MessageHandler<WatchMessage> for WatchHandler {
        fn notifiers(&mut self) -> Vec<Box<dyn super::Notifier<WatchMessage>>> {
            vec![Box::new(WatermarkNotifier {
                rx: self.watermark_rx.clone(),
            })]
        }

        async fn handle(&mut self, message: WatchMessage) -> Result<(), SlateDBError> {
            match message {
                WatchMessage::Work(seq) => {
                    self.blocked_seq = Some(seq);
                }
                WatchMessage::WatermarkAdvanced => {}
            }
            // After any event, try to process blocked work.
            if let Some(seq) = self.blocked_seq {
                if *self.watermark_rx.borrow() >= seq {
                    self.blocked_seq = None;
                    self.log
                        .lock()
                        .unwrap()
                        .push((Phase::Pre, TestMessage::Channel(seq as i32)));
                }
            }
            Ok(())
        }

        async fn cleanup(
            &mut self,
            _messages: BoxStream<'async_trait, WatchMessage>,
            result: Result<(), SlateDBError>,
        ) -> Result<(), SlateDBError> {
            self.cleanup_called.write(result);
            Ok(())
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_notifier_dispatches_when_watermark_advances() {
        let log = Arc::new(Mutex::new(Vec::<(Phase, TestMessage)>::new()));
        let (watermark_tx, watermark_rx) = tokio::sync::watch::channel(0u64);
        let (tx, rx) = async_channel::unbounded();
        let clock: Arc<dyn SystemClock> = Arc::new(DefaultSystemClock::new());
        let handler = WatchHandler {
            log: log.clone(),
            cleanup_called: WatchableOnceCell::new(),
            watermark_rx,
            blocked_seq: None,
        };

        let cancellation_token = CancellationToken::new();
        let mut dispatcher =
            MessageDispatcher::new(Box::new(handler), rx, clock, cancellation_token.clone());
        let join = tokio::spawn(async move { dispatcher.run().await });

        // Send work that requires watermark >= 5.
        tx.try_send(WatchMessage::Work(5)).unwrap();
        // Give the handler a chance to process the Work message.
        tokio::time::sleep(Duration::from_millis(50)).await;
        // Work should NOT be logged yet — watermark is still 0.
        assert!(log.lock().unwrap().is_empty(), "work logged too early");

        // Advance the watermark. This should trigger WatermarkAdvanced via notifier.
        watermark_tx.send(5).unwrap();
        wait_for_message_count(log.clone(), 1).await;

        assert_eq!(
            log.lock().unwrap().clone(),
            vec![(Phase::Pre, TestMessage::Channel(5))]
        );

        // Now send work where the watermark is already caught up — should process
        // immediately since handle() always checks after any event.
        tx.try_send(WatchMessage::Work(3)).unwrap();
        wait_for_message_count(log.clone(), 2).await;

        assert_eq!(
            log.lock().unwrap().clone(),
            vec![
                (Phase::Pre, TestMessage::Channel(5)),
                (Phase::Pre, TestMessage::Channel(3)),
            ]
        );

        cancellation_token.cancel();
        let result = timeout(Duration::from_secs(5), join)
            .await
            .expect("dispatcher did not stop in time")
            .expect("join failed");
        assert!(matches!(result, Ok(())));
    }
}
