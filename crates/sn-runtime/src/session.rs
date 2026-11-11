use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashSet, VecDeque},
    fmt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
    },
    thread::{self, JoinHandle, ThreadId},
    time::{Duration, Instant},
};

use rquickjs::{Context, Exception, Function, Persistent, Runtime, TypedArray, Value};
use sn_core::{NodeId, SessionGeneration, SurfaceId};

use crate::{
    clock::{MonotonicClock, RuntimeClock},
    host::{self, HostState, SharedHostState},
    scheduler::{self, SchedulerState},
};

pub const MAX_EXTERNAL_COMPLETION_BYTES: usize = 1024 * 1024;
const MAX_RETAINED_DIAGNOSTICS: usize = 128;
const MAX_DIAGNOSTIC_TEXT_BYTES: usize = 4_096;
const MAX_SESSION_CLEANUPS: usize = 4_096;
const DEFAULT_QUEUE_CAPACITY: usize = 256;
const DEFAULT_MAX_QUEUED_BYTES: usize = 16 * 1024 * 1024;
const MAX_QUEUED_BYTES: usize = 1024 * 1024 * 1024;
const MAX_QUEUE_CAPACITY: usize = 65_536;
const DEFAULT_JOB_BUDGET: usize = 64;
const DEFAULT_COMMAND_QUANTUM: usize = 8;
const DEFAULT_JOB_SLICE_TIME: Duration = Duration::from_millis(4);
const DEFAULT_MAX_EXECUTION_TIME: Duration = Duration::from_millis(100);
const DEFAULT_HEAP_LIMIT: usize = 64 * 1024 * 1024;
const DEFAULT_STACK_LIMIT: usize = 256 * 1024;
const MAX_IDLE_WAIT: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct EngineSessionConfig {
    pub surface_id: SurfaceId,
    pub session_generation: SessionGeneration,
    pub queue_capacity: usize,
    pub max_queued_bytes: usize,
    pub max_jobs_per_slice: usize,
    pub max_job_slice_time: Duration,
    pub max_execution_time: Duration,
    pub heap_limit_bytes: usize,
    pub stack_limit_bytes: usize,
    pub clock: Arc<dyn RuntimeClock>,
}

impl Default for EngineSessionConfig {
    fn default() -> Self {
        Self {
            surface_id: SurfaceId::new(1),
            session_generation: SessionGeneration::new(1),
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
            max_queued_bytes: DEFAULT_MAX_QUEUED_BYTES,
            max_jobs_per_slice: DEFAULT_JOB_BUDGET,
            max_job_slice_time: DEFAULT_JOB_SLICE_TIME,
            max_execution_time: DEFAULT_MAX_EXECUTION_TIME,
            heap_limit_bytes: DEFAULT_HEAP_LIMIT,
            stack_limit_bytes: DEFAULT_STACK_LIMIT,
            clock: Arc::new(MonotonicClock::default()),
        }
    }
}

impl EngineSessionConfig {
    fn validate(&self) -> Result<(), SessionError> {
        if self.surface_id.get() == 0 || self.session_generation.get() == 0 {
            return Err(SessionError::InvalidConfig(
                "SurfaceId and SessionGeneration must be non-zero".into(),
            ));
        }
        if self.queue_capacity == 0
            || self.queue_capacity > MAX_QUEUE_CAPACITY
            || self.max_queued_bytes == 0
            || self.max_queued_bytes > MAX_QUEUED_BYTES
            || self.max_jobs_per_slice == 0
            || self.max_job_slice_time.is_zero()
            || self.max_execution_time.is_zero()
            || self.heap_limit_bytes == 0
            || self.stack_limit_bytes == 0
        {
            return Err(SessionError::InvalidConfig(
                "Queue, job budgets, execution limits, heap and stack limits must be positive"
                    .into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    QueueFull,
    Closed,
    Cancelled,
    InvalidConfig(String),
    PayloadTooLarge { actual: usize, limit: usize },
    Runtime(String),
    ResponseLost,
    WorkerPanicked,
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => write!(f, "EngineSession command queue is full"),
            Self::Closed => write!(f, "EngineSession is closing or closed"),
            Self::Cancelled => write!(f, "EngineSession task was cancelled"),
            Self::InvalidConfig(message) => {
                write!(f, "Invalid EngineSession configuration: {message}")
            }
            Self::PayloadTooLarge { actual, limit } => {
                write!(f, "Payload is {actual} bytes; configured limit is {limit}")
            }
            Self::Runtime(message) => write!(f, "QuickJS runtime error: {message}"),
            Self::ResponseLost => write!(f, "EngineSession worker dropped a task response"),
            Self::WorkerPanicked => write!(f, "EngineSession worker thread panicked"),
        }
    }
}

impl std::error::Error for SessionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    JavaScriptException,
    PromiseJobException,
    PromiseRejection,
    SchedulerBudgetExhausted,
    TaskCancelled,
    StaleCompletion,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDiagnostic {
    pub kind: DiagnosticKind,
    pub at: Duration,
    pub message: String,
    pub stack: Option<String>,
    pub handled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    pub revision: u64,
    pub node_count: usize,
    pub callback_count: usize,
    pub roots: Vec<u64>,
    pub button_node_id: Option<u64>,
    pub has_view: bool,
    pub has_button: bool,
    pub has_initial_count: bool,
    pub has_click_binding: bool,
    pub has_insert: bool,
    pub last_commit_operation_count: usize,
    pub last_commit_replace_text_count: usize,
    pub last_commit_revision: Option<u64>,
    pub counter_text: Option<String>,
}

#[derive(Default)]
struct TaskRegistry {
    active: HashSet<u64>,
    cancelled: HashSet<u64>,
}

impl TaskRegistry {
    fn register(&mut self, id: u64) {
        self.active.insert(id);
    }

    fn cancel(&mut self, id: u64) -> bool {
        if self.active.contains(&id) {
            self.cancelled.insert(id);
            true
        } else {
            false
        }
    }

    fn is_cancelled(&self, id: u64) -> bool {
        self.cancelled.contains(&id)
    }

    fn finish<T>(&mut self, id: u64, result: Result<T, SessionError>) -> Result<T, SessionError> {
        let was_cancelled = self.cancelled.remove(&id);
        self.active.remove(&id);
        if was_cancelled {
            Err(SessionError::Cancelled)
        } else {
            result
        }
    }
}

struct SessionCleanupRegistry {
    next_id: u32,
    callbacks: BTreeMap<u32, Persistent<Function<'static>>>,
    closed: bool,
}

impl Default for SessionCleanupRegistry {
    fn default() -> Self {
        Self {
            next_id: 1,
            callbacks: BTreeMap::new(),
            closed: false,
        }
    }
}

impl SessionCleanupRegistry {
    fn take_all_reverse(&mut self) -> Vec<Persistent<Function<'static>>> {
        self.closed = true;
        std::mem::take(&mut self.callbacks)
            .into_values()
            .rev()
            .collect()
    }
}

pub struct TaskTicket<T> {
    task_id: u64,
    response: Receiver<Result<T, SessionError>>,
    tasks: Arc<Mutex<TaskRegistry>>,
}

impl<T> TaskTicket<T> {
    pub fn task_id(&self) -> u64 {
        self.task_id
    }

    /// Marks this task for cancellation. The interrupt handler stops a running
    /// JS turn; a queued task is rejected before it starts.
    pub fn cancel(&self) -> bool {
        lock_recover(&self.tasks).cancel(self.task_id)
    }

    pub fn wait(self) -> Result<T, SessionError> {
        self.response
            .recv()
            .unwrap_or(Err(SessionError::ResponseLost))
    }

    pub fn wait_timeout(&self, timeout: Duration) -> Result<T, SessionError> {
        match self.response.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(SessionError::Runtime(
                "Timed out waiting for an EngineSession task".into(),
            )),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(SessionError::ResponseLost),
        }
    }
}

#[derive(Clone)]
pub struct EngineSessionHandle {
    sender: SyncSender<Command>,
    queue_capacity: usize,
    max_queued_bytes: usize,
    pending_commands: Arc<AtomicUsize>,
    pending_queue_bytes: Arc<AtomicUsize>,
    submission_gate: Arc<Mutex<()>>,
    next_task_id: Arc<AtomicU64>,
    closed: Arc<AtomicBool>,
    shutdown_requested: Arc<AtomicBool>,
    active_turn: Arc<AtomicU64>,
    frame_requested: Arc<AtomicBool>,
    tasks: Arc<Mutex<TaskRegistry>>,
    diagnostics: Arc<Mutex<VecDeque<RuntimeDiagnostic>>>,
    owner_thread: Arc<Mutex<Option<ThreadId>>>,
}

impl EngineSessionHandle {
    fn allocate_task_id(&self) -> Result<u64, SessionError> {
        self.next_task_id
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| SessionError::Runtime("EngineSession task id space exhausted".into()))
    }

    fn enqueue<T>(
        &self,
        payload_bytes: usize,
        build: impl FnOnce(u64, SyncSender<Result<T, SessionError>>) -> Command,
    ) -> Result<TaskTicket<T>, SessionError> {
        let _gate = lock_recover(&self.submission_gate);
        if self.closed.load(Ordering::Acquire) {
            return Err(SessionError::Closed);
        }
        if payload_bytes > self.max_queued_bytes {
            return Err(SessionError::PayloadTooLarge {
                actual: payload_bytes,
                limit: self.max_queued_bytes,
            });
        }

        let reserved =
            self.pending_commands
                .try_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                    (pending < self.queue_capacity).then_some(pending + 1)
                });
        if reserved.is_err() {
            return Err(SessionError::QueueFull);
        }
        let byte_reservation =
            self.pending_queue_bytes
                .try_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                    pending
                        .checked_add(payload_bytes)
                        .filter(|total| *total <= self.max_queued_bytes)
                });
        if byte_reservation.is_err() {
            self.pending_commands.fetch_sub(1, Ordering::AcqRel);
            return Err(SessionError::QueueFull);
        }

        let task_id = match self.allocate_task_id() {
            Ok(id) => id,
            Err(error) => {
                self.pending_commands.fetch_sub(1, Ordering::AcqRel);
                self.pending_queue_bytes
                    .fetch_sub(payload_bytes, Ordering::AcqRel);
                return Err(error);
            }
        };
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        lock_recover(&self.tasks).register(task_id);
        match self.sender.try_send(build(task_id, response_tx)) {
            Ok(()) => Ok(TaskTicket {
                task_id,
                response: response_rx,
                tasks: self.tasks.clone(),
            }),
            Err(TrySendError::Full(_)) => {
                lock_recover(&self.tasks).active.remove(&task_id);
                self.pending_commands.fetch_sub(1, Ordering::AcqRel);
                self.pending_queue_bytes
                    .fetch_sub(payload_bytes, Ordering::AcqRel);
                Err(SessionError::QueueFull)
            }
            Err(TrySendError::Disconnected(_)) => {
                lock_recover(&self.tasks).active.remove(&task_id);
                self.pending_commands.fetch_sub(1, Ordering::AcqRel);
                self.pending_queue_bytes
                    .fetch_sub(payload_bytes, Ordering::AcqRel);
                Err(SessionError::Closed)
            }
        }
    }

    /// Evaluates a script on the QuickJS owner thread, then runs one bounded
    /// Promise-job slice and flushes the typed host batch.
    pub fn evaluate(
        &self,
        source: impl Into<String>,
    ) -> Result<TaskTicket<RuntimeSnapshot>, SessionError> {
        let source = source.into();
        self.enqueue(source.len(), |id, response| Command::Evaluate {
            id,
            source,
            response,
        })
    }

    /// Evaluates an expression which must return a JavaScript string. Useful
    /// for diagnostics and host tooling without exposing QuickJS values.
    pub fn evaluate_text(
        &self,
        source: impl Into<String>,
    ) -> Result<TaskTicket<String>, SessionError> {
        let source = source.into();
        self.enqueue(source.len(), |id, response| Command::EvaluateText {
            id,
            source,
            response,
        })
    }

    pub fn find_element(
        &self,
        tag: impl Into<String>,
    ) -> Result<TaskTicket<Option<u64>>, SessionError> {
        let tag = tag.into();
        self.enqueue(tag.len(), |id, response| Command::FindElement {
            id,
            tag,
            response,
        })
    }

    pub fn dispatch_event(
        &self,
        node_id: u64,
        event_name: impl Into<String>,
    ) -> Result<TaskTicket<bool>, SessionError> {
        let event_name = event_name.into();
        self.enqueue(event_name.len(), |id, response| Command::DispatchEvent {
            id,
            node_id,
            event_name,
            response,
        })
    }

    /// Delivers a platform frame notification. RAF callbacks are coalesced and
    /// invoked in request order on the owner thread.
    pub fn deliver_frame(&self, timestamp: Duration) -> Result<TaskTicket<usize>, SessionError> {
        self.enqueue(0, |id, response| Command::Frame {
            id,
            timestamp,
            response,
        })
    }

    /// Delivers an external/resource completion using an owned byte payload.
    /// The callback is one-shot and removed before it is invoked.
    pub fn post_completion(
        &self,
        callback_id: u64,
        payload: Vec<u8>,
    ) -> Result<TaskTicket<bool>, SessionError> {
        if payload.len() > MAX_EXTERNAL_COMPLETION_BYTES {
            return Err(SessionError::PayloadTooLarge {
                actual: payload.len(),
                limit: MAX_EXTERNAL_COMPLETION_BYTES,
            });
        }
        let payload_bytes = payload.len();
        self.enqueue(payload_bytes, |id, response| Command::Completion {
            id,
            callback_id,
            payload,
            response,
        })
    }

    pub fn cancel_timer(&self, timer_id: u32) -> Result<TaskTicket<bool>, SessionError> {
        self.enqueue(0, |id, response| Command::CancelTimer {
            id,
            timer_id,
            response,
        })
    }

    pub fn snapshot(&self) -> Result<TaskTicket<RuntimeSnapshot>, SessionError> {
        self.enqueue(0, |id, response| Command::Snapshot { id, response })
    }

    /// Wakes the event loop after a host clock advance or external state change.
    /// Useful with ManualClock and deterministic host integrations.
    pub fn pump(&self) -> Result<TaskTicket<bool>, SessionError> {
        self.enqueue(0, |id, response| Command::Pump { id, response })
    }

    pub fn frame_requested(&self) -> bool {
        self.frame_requested.load(Ordering::Acquire)
    }

    pub fn take_diagnostics(&self) -> Vec<RuntimeDiagnostic> {
        lock_recover(&self.diagnostics).drain(..).collect()
    }

    pub fn is_owner_thread(&self) -> bool {
        lock_recover(&self.owner_thread)
            .as_ref()
            .is_some_and(|owner| *owner == thread::current().id())
    }
}

pub struct EngineSession {
    handle: EngineSessionHandle,
    worker: Option<JoinHandle<()>>,
}

impl EngineSession {
    pub fn start(config: EngineSessionConfig) -> Result<Self, SessionError> {
        config.validate()?;

        let (sender, receiver) = mpsc::sync_channel(config.queue_capacity + 1);
        let handle = EngineSessionHandle {
            sender,
            queue_capacity: config.queue_capacity,
            max_queued_bytes: config.max_queued_bytes,
            pending_commands: Arc::new(AtomicUsize::new(0)),
            pending_queue_bytes: Arc::new(AtomicUsize::new(0)),
            submission_gate: Arc::new(Mutex::new(())),
            next_task_id: Arc::new(AtomicU64::new(1)),
            closed: Arc::new(AtomicBool::new(false)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            active_turn: Arc::new(AtomicU64::new(0)),
            frame_requested: Arc::new(AtomicBool::new(false)),
            tasks: Arc::new(Mutex::new(TaskRegistry::default())),
            diagnostics: Arc::new(Mutex::new(VecDeque::with_capacity(
                MAX_RETAINED_DIAGNOSTICS,
            ))),
            owner_thread: Arc::new(Mutex::new(None)),
        };

        let worker_handle = handle.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name(format!(
                "solid-native-engine-{}-{}",
                config.surface_id.get(),
                config.session_generation.get()
            ))
            .spawn(move || worker_main(config, receiver, worker_handle, ready_tx))
            .map_err(|error| {
                SessionError::Runtime(format!("failed to spawn engine thread: {error}"))
            })?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                handle,
                worker: Some(worker),
            }),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(_) => {
                let _ = worker.join();
                Err(SessionError::WorkerPanicked)
            }
        }
    }

    pub fn handle(&self) -> EngineSessionHandle {
        self.handle.clone()
    }

    /// Performs ordered session teardown on the owner thread and joins it.
    pub fn shutdown(mut self) -> Result<(), SessionError> {
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        {
            let _gate = lock_recover(&self.handle.submission_gate);
            self.handle.closed.store(true, Ordering::Release);
            self.handle
                .shutdown_requested
                .store(true, Ordering::Release);
            self.handle
                .sender
                .try_send(Command::Shutdown(Some(response_tx)))
                .map_err(|error| match error {
                    TrySendError::Full(_) => SessionError::Runtime(
                        "Reserved shutdown queue slot is unexpectedly full".into(),
                    ),
                    TrySendError::Disconnected(_) => SessionError::Closed,
                })?;
        }

        let result = response_rx
            .recv()
            .unwrap_or(Err(SessionError::ResponseLost));
        self.join_worker()?;
        result
    }

    fn join_worker(&mut self) -> Result<(), SessionError> {
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| SessionError::WorkerPanicked)?;
        }
        Ok(())
    }
}

impl Drop for EngineSession {
    fn drop(&mut self) {
        let _gate = lock_recover(&self.handle.submission_gate);
        self.handle.closed.store(true, Ordering::Release);
        self.handle
            .shutdown_requested
            .store(true, Ordering::Release);
        let _ = self.handle.sender.try_send(Command::Shutdown(None));
        // Dropping JoinHandle detaches; Drop must never block a platform/UI caller.
        self.worker.take();
    }
}

enum Command {
    Evaluate {
        id: u64,
        source: String,
        response: SyncSender<Result<RuntimeSnapshot, SessionError>>,
    },
    EvaluateText {
        id: u64,
        source: String,
        response: SyncSender<Result<String, SessionError>>,
    },
    FindElement {
        id: u64,
        tag: String,
        response: SyncSender<Result<Option<u64>, SessionError>>,
    },
    DispatchEvent {
        id: u64,
        node_id: u64,
        event_name: String,
        response: SyncSender<Result<bool, SessionError>>,
    },
    Frame {
        id: u64,
        timestamp: Duration,
        response: SyncSender<Result<usize, SessionError>>,
    },
    Completion {
        id: u64,
        callback_id: u64,
        payload: Vec<u8>,
        response: SyncSender<Result<bool, SessionError>>,
    },
    CancelTimer {
        id: u64,
        timer_id: u32,
        response: SyncSender<Result<bool, SessionError>>,
    },
    Snapshot {
        id: u64,
        response: SyncSender<Result<RuntimeSnapshot, SessionError>>,
    },
    Pump {
        id: u64,
        response: SyncSender<Result<bool, SessionError>>,
    },
    Shutdown(Option<SyncSender<Result<(), SessionError>>>),
}

impl Command {
    fn payload_bytes(&self) -> usize {
        match self {
            Self::Evaluate { source, .. } | Self::EvaluateText { source, .. } => source.len(),
            Self::FindElement { tag, .. } => tag.len(),
            Self::DispatchEvent { event_name, .. } => event_name.len(),
            Self::Completion { payload, .. } => payload.len(),
            Self::Frame { .. }
            | Self::CancelTimer { .. }
            | Self::Snapshot { .. }
            | Self::Pump { .. }
            | Self::Shutdown(_) => 0,
        }
    }
}

struct RuntimeWorker {
    // Persistent roots are declared first. Rust drops fields in declaration
    // order, so these registries and their Context-owned callback closures are
    // released before Runtime is destroyed.
    host: SharedHostState,
    scheduler: std::rc::Rc<RefCell<SchedulerState>>,
    cleanup: std::rc::Rc<RefCell<SessionCleanupRegistry>>,
    context: Context,
    runtime: Runtime,
    config: EngineSessionConfig,
    active_turn: Arc<AtomicU64>,
    tasks: Arc<Mutex<TaskRegistry>>,
    shutdown_requested: Arc<AtomicBool>,
    deadline: std::rc::Rc<Cell<Option<Instant>>>,
    diagnostics: Arc<Mutex<VecDeque<RuntimeDiagnostic>>>,
}

fn worker_main(
    config: EngineSessionConfig,
    receiver: Receiver<Command>,
    handle: EngineSessionHandle,
    ready: SyncSender<Result<(), SessionError>>,
) {
    *lock_recover(&handle.owner_thread) = Some(thread::current().id());

    let worker = match RuntimeWorker::new(
        config,
        handle.tasks.clone(),
        handle.shutdown_requested.clone(),
        handle.active_turn.clone(),
        handle.diagnostics.clone(),
        handle.frame_requested.clone(),
    ) {
        Ok(worker) => {
            let _ = ready.send(Ok(()));
            worker
        }
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };

    run_event_loop(worker, receiver, &handle);
    handle.closed.store(true, Ordering::Release);
    lock_recover(&handle.owner_thread).take();
}

impl RuntimeWorker {
    fn new(
        config: EngineSessionConfig,
        tasks: Arc<Mutex<TaskRegistry>>,
        shutdown_requested: Arc<AtomicBool>,
        active_turn: Arc<AtomicU64>,
        diagnostics: Arc<Mutex<VecDeque<RuntimeDiagnostic>>>,
        frame_requested: Arc<AtomicBool>,
    ) -> Result<Self, SessionError> {
        let runtime = Runtime::new().map_err(runtime_error)?;
        runtime.set_memory_limit(config.heap_limit_bytes);
        runtime.set_max_stack_size(config.stack_limit_bytes);

        let deadline = std::rc::Rc::new(Cell::new(None::<Instant>));

        {
            let active_turn = active_turn.clone();
            let deadline = deadline.clone();
            let tasks = tasks.clone();
            let shutdown_requested = shutdown_requested.clone();
            runtime.set_interrupt_handler(Some(Box::new(move || {
                if shutdown_requested.load(Ordering::Acquire) {
                    return true;
                }
                let task_id = active_turn.load(Ordering::Acquire);
                if task_id != 0 && lock_recover(&tasks).is_cancelled(task_id) {
                    return true;
                }
                deadline.get().is_some_and(|limit| Instant::now() >= limit)
            })));
        }

        {
            let diagnostics = diagnostics.clone();
            let clock = config.clock.clone();
            runtime.set_host_promise_rejection_tracker(Some(Box::new(
                move |_ctx, _promise, reason, is_handled| {
                    let (message, stack) = match Exception::from_value(reason.clone()) {
                        Ok(exception) => (
                            exception
                                .message()
                                .unwrap_or_else(|| "Promise rejected with an Error".into()),
                            exception.stack(),
                        ),
                        Err(_) => {
                            let message = reason
                                .as_string()
                                .and_then(|value| value.to_string().ok())
                                .unwrap_or_else(|| format!("{reason:?}"));
                            (message, None)
                        }
                    };
                    push_diagnostic(
                        &diagnostics,
                        RuntimeDiagnostic {
                            kind: DiagnosticKind::PromiseRejection,
                            at: clock.now(),
                            message,
                            stack,
                            handled: Some(is_handled),
                        },
                    );
                },
            )));
        }

        let context = Context::full(&runtime).map_err(runtime_error)?;
        let host = std::rc::Rc::new(RefCell::new(HostState::new(
            config.surface_id,
            config.session_generation,
        )));
        let scheduler =
            std::rc::Rc::new(RefCell::new(SchedulerState::new(frame_requested.clone())));
        let cleanup = std::rc::Rc::new(RefCell::new(SessionCleanupRegistry::default()));

        context
            .with(|ctx| -> rquickjs::Result<()> {
                host::install_host_bindings(ctx.clone(), host.clone())?;
                scheduler::install_bindings(ctx.clone(), scheduler.clone(), config.clock.clone())?;
                install_cleanup_binding(ctx.clone(), cleanup.clone())?;
                ctx.globals()
                    .set("__solid_native_runtime__", "QuickJS-NG")?;
                Ok(())
            })
            .map_err(runtime_error)?;

        Ok(Self {
            context,
            runtime,
            host,
            scheduler,
            cleanup,
            config,
            active_turn,
            tasks,
            shutdown_requested,
            deadline,
            diagnostics,
        })
    }

    fn snapshot(&self) -> Result<RuntimeSnapshot, SessionError> {
        let host = self
            .host
            .try_borrow()
            .map_err(|_| SessionError::Runtime("Host state is already borrowed".into()))?;
        let last_commit = host.sink().commits().back();
        let has_operation = |predicate: fn(&sn_core::HostOperation) -> bool| {
            last_commit.is_some_and(|commit| commit.delta.iter().any(predicate))
        };
        let has_view = has_operation(|operation| {
            matches!(
                operation,
                sn_core::HostOperation::CreateElement { tag, .. } if tag == "view"
            )
        });
        let has_button = has_operation(|operation| {
            matches!(
                operation,
                sn_core::HostOperation::CreateElement { tag, .. } if tag == "button"
            )
        });
        let has_initial_count = has_operation(|operation| {
            matches!(
                operation,
                sn_core::HostOperation::CreateText { text, .. } if text == "Count: 0"
            )
        });
        let has_click_binding = has_operation(|operation| {
            matches!(
                operation,
                sn_core::HostOperation::BindEvent { event, .. } if event == "click"
            )
        });
        let has_insert = has_operation(|operation| {
            matches!(operation, sn_core::HostOperation::InsertNode { .. })
        });
        let last_commit_replace_text_count = last_commit.map_or(0, |commit| {
            commit
                .delta
                .iter()
                .filter(|operation| matches!(operation, sn_core::HostOperation::ReplaceText { .. }))
                .count()
        });
        let counter_text = host
            .tree
            .find_text_containing("Count:")
            .and_then(|id| host.tree.text_content(id).ok())
            .map(str::to_owned);
        Ok(RuntimeSnapshot {
            revision: host.tree.revision(),
            node_count: host.tree.node_count(),
            callback_count: host.callback_count(),
            roots: host.tree.roots().iter().map(|id| id.get()).collect(),
            button_node_id: host.tree.find_element("button").map(|id| id.get()),
            has_view,
            has_button,
            has_initial_count,
            has_click_binding,
            has_insert,
            last_commit_operation_count: last_commit.map_or(0, |ack| ack.delta.len()),
            last_commit_replace_text_count,
            last_commit_revision: last_commit.map(|ack| ack.revision),
            counter_text,
        })
    }

    fn record(
        &self,
        kind: DiagnosticKind,
        message: impl Into<String>,
        stack: Option<String>,
        handled: Option<bool>,
    ) {
        push_diagnostic(
            &self.diagnostics,
            RuntimeDiagnostic {
                kind,
                at: self.config.clock.now(),
                message: message.into(),
                stack,
                handled,
            },
        );
    }

    fn record_js_error(&self, error: &rquickjs::Error, kind: DiagnosticKind) {
        self.context
            .with(|ctx| record_exception(&ctx, error, &self.diagnostics, &self.config.clock, kind));
    }

    fn set_turn(&self, id: u64) {
        self.active_turn.store(id, Ordering::Release);
        self.deadline
            .set(Some(Instant::now() + self.config.max_execution_time));
    }

    fn end_turn(&self) {
        self.deadline.set(None);
        self.active_turn.store(0, Ordering::Release);
    }

    fn check_cancelled(&self, id: u64) -> Result<(), SessionError> {
        if self.shutdown_requested.load(Ordering::Acquire) {
            return Err(SessionError::Closed);
        }
        if lock_recover(&self.tasks).is_cancelled(id) {
            self.record(
                DiagnosticKind::TaskCancelled,
                format!("EngineSession task {id} was cancelled"),
                None,
                None,
            );
            Err(SessionError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn flush_host_batch(&self) -> Result<(), SessionError> {
        let result = self.context.with(|ctx| -> rquickjs::Result<()> {
            let exists: bool =
                ctx.eval("typeof globalThis.__sn_flush_host_batch === 'function'")?;
            if exists {
                let flush: Function = ctx.globals().get("__sn_flush_host_batch")?;
                let _: Value = flush.call(())?;
            }
            Ok(())
        });
        if let Err(error) = &result {
            self.record_js_error(error, DiagnosticKind::JavaScriptException);
        }
        result.map_err(runtime_error)
    }

    fn drain_job_slice(&self, task_id: u64) -> bool {
        let started = Instant::now();
        let mut executed = 0usize;

        while executed < self.config.max_jobs_per_slice
            && started.elapsed() < self.config.max_job_slice_time
            && self.runtime.is_job_pending()
        {
            self.active_turn.store(task_id, Ordering::Release);
            self.deadline
                .set(Some(Instant::now() + self.config.max_execution_time));
            let result = self.runtime.execute_pending_job();
            self.deadline.set(None);
            self.active_turn.store(0, Ordering::Release);
            executed += 1;

            if let Err(error) = result {
                let diagnostics = self.diagnostics.clone();
                let clock = self.config.clock.clone();
                error.0.with(|ctx| {
                    let thrown = ctx.catch();
                    let (message, stack) = exception_parts(thrown);
                    push_diagnostic(
                        &diagnostics,
                        RuntimeDiagnostic {
                            kind: DiagnosticKind::PromiseJobException,
                            at: clock.now(),
                            message,
                            stack,
                            handled: None,
                        },
                    );
                });
            }

            if self.shutdown_requested.load(Ordering::Acquire)
                || (task_id != 0 && lock_recover(&self.tasks).is_cancelled(task_id))
            {
                break;
            }
        }

        let pending = self.runtime.is_job_pending();
        if pending
            && (executed >= self.config.max_jobs_per_slice
                || started.elapsed() >= self.config.max_job_slice_time)
        {
            self.record(
                DiagnosticKind::SchedulerBudgetExhausted,
                format!(
                    "Promise job slice yielded after {executed} jobs and {:?}",
                    started.elapsed()
                ),
                None,
                None,
            );
        }
        let _ = self.flush_host_batch();
        pending
    }

    fn evaluate(&self, id: u64, source: &str) -> Result<RuntimeSnapshot, SessionError> {
        self.check_cancelled(id)?;
        self.set_turn(id);
        let result = self.context.with(|ctx| ctx.eval::<(), _>(source));
        self.end_turn();

        if let Err(error) = &result {
            self.record_js_error(error, DiagnosticKind::JavaScriptException);
        }
        self.check_cancelled(id)?;
        result.map_err(runtime_error)?;

        self.drain_job_slice(id);
        self.check_cancelled(id)?;
        self.flush_host_batch()?;
        self.snapshot()
    }

    fn evaluate_text(&self, id: u64, source: &str) -> Result<String, SessionError> {
        self.check_cancelled(id)?;
        self.set_turn(id);
        let result = self.context.with(|ctx| ctx.eval::<String, _>(source));
        self.end_turn();

        if let Err(error) = &result {
            self.record_js_error(error, DiagnosticKind::JavaScriptException);
        }
        self.check_cancelled(id)?;
        let value = result.map_err(runtime_error)?;
        self.drain_job_slice(id);
        self.check_cancelled(id)?;
        self.flush_host_batch()?;
        Ok(value)
    }

    fn find_element(&self, id: u64, tag: &str) -> Result<Option<u64>, SessionError> {
        self.check_cancelled(id)?;
        let host = self
            .host
            .try_borrow()
            .map_err(|_| SessionError::Runtime("Host state is already borrowed".into()))?;
        Ok(host.tree.find_element(tag).map(|node_id| node_id.get()))
    }

    fn dispatch_event(
        &self,
        id: u64,
        node_id: u64,
        event_name: &str,
    ) -> Result<bool, SessionError> {
        self.check_cancelled(id)?;
        self.set_turn(id);
        let result = self
            .context
            .with(|ctx| host::dispatch_event(&ctx, &self.host, NodeId::new(node_id), event_name));
        self.end_turn();

        let dispatched = match result {
            Ok(value) => value,
            Err(error) => {
                self.record_js_error(&error, DiagnosticKind::JavaScriptException);
                self.check_cancelled(id)?;
                return Err(runtime_error(error));
            }
        };
        self.check_cancelled(id)?;
        if dispatched {
            self.drain_job_slice(id);
            self.check_cancelled(id)?;
            self.flush_host_batch()?;
        }
        Ok(dispatched)
    }

    fn deliver_frame(&self, id: u64, timestamp: Duration) -> Result<usize, SessionError> {
        self.check_cancelled(id)?;
        self.check_cancelled(id)?;
        let cutoff = self
            .scheduler
            .try_borrow_mut()
            .map_err(|_| SessionError::Runtime("Scheduler state is already borrowed".into()))?
            .begin_frame();
        let Some(cutoff) = cutoff else {
            return Ok(0);
        };

        let mut count = 0;
        loop {
            // Take only one callback at a time. A previous callback can still
            // cancel a later callback from the same frame before it starts.
            let callback = self
                .scheduler
                .try_borrow_mut()
                .map_err(|_| SessionError::Runtime("Scheduler state is already borrowed".into()))?
                .take_next_frame_callback(cutoff);
            let Some((_callback_id, callback)) = callback else {
                break;
            };

            self.set_turn(id);
            let timestamp_ms = timestamp.as_secs_f64() * 1000.0;
            let result = self
                .context
                .with(|ctx| callback.restore(&ctx)?.call::<_, ()>((timestamp_ms,)));
            self.end_turn();
            if let Err(error) = result {
                self.record_js_error(&error, DiagnosticKind::JavaScriptException);
            }
            self.check_cancelled(id)?;
            self.drain_job_slice(id);
            count += 1;
        }
        self.flush_host_batch()?;
        Ok(count)
    }

    fn deliver_completion(
        &self,
        id: u64,
        callback_id: u64,
        payload: &[u8],
    ) -> Result<bool, SessionError> {
        self.check_cancelled(id)?;
        let callback = self
            .scheduler
            .try_borrow_mut()
            .map_err(|_| SessionError::Runtime("Scheduler state is already borrowed".into()))?
            .take_completion(callback_id);

        let Some(callback) = callback else {
            self.record(
                DiagnosticKind::StaleCompletion,
                format!("Completion callback {callback_id} is no longer registered"),
                None,
                None,
            );
            return Ok(false);
        };

        self.set_turn(id);
        let result = self.context.with(|ctx| {
            let data = TypedArray::<u8>::new_copy(ctx.clone(), payload)?;
            callback.restore(&ctx)?.call::<_, ()>((data,))
        });
        self.end_turn();

        if let Err(error) = result {
            self.record_js_error(&error, DiagnosticKind::JavaScriptException);
            self.check_cancelled(id)?;
            return Err(runtime_error(error));
        }
        self.check_cancelled(id)?;
        self.drain_job_slice(id);
        self.flush_host_batch()?;
        Ok(true)
    }

    fn fire_due_timer(&self, task_id: u64) -> bool {
        let now = self.config.clock.now();
        let callback = self
            .scheduler
            .try_borrow_mut()
            .ok()
            .and_then(|mut state| state.take_due_timer(now));
        let Some((_timer_id, callback)) = callback else {
            return false;
        };

        self.set_turn(task_id);
        let result = self
            .context
            .with(|ctx| callback.restore(&ctx)?.call::<_, ()>(()));
        self.end_turn();
        if let Err(error) = result {
            self.record_js_error(&error, DiagnosticKind::JavaScriptException);
        }
        self.drain_job_slice(task_id);
        let _ = self.flush_host_batch();
        true
    }

    fn next_timer_deadline(&self) -> Option<Duration> {
        self.scheduler
            .try_borrow_mut()
            .ok()
            .and_then(|mut scheduler| scheduler.next_deadline())
    }

    fn cancel_timer(&self, timer_id: u32) -> Result<bool, SessionError> {
        let mut scheduler = self
            .scheduler
            .try_borrow_mut()
            .map_err(|_| SessionError::Runtime("Scheduler state is already borrowed".into()))?;
        Ok(scheduler.clear_timer(timer_id))
    }

    fn shutdown(&self) -> Result<(), SessionError> {
        // The active JS turn has now returned to the owner loop. Re-enable JS
        // execution only for deterministic teardown callbacks on this thread.
        self.shutdown_requested.store(false, Ordering::Release);
        self.record(
            DiagnosticKind::Shutdown,
            "EngineSession shutdown started",
            None,
            None,
        );

        self.scheduler
            .try_borrow_mut()
            .map_err(|_| SessionError::Runtime("Scheduler state is already borrowed".into()))?
            .cancel_timers_and_frames();

        let cleanups = self.cleanup.borrow_mut().take_all_reverse();
        for cleanup in cleanups {
            let result = self
                .context
                .with(|ctx| cleanup.restore(&ctx)?.call::<_, ()>(()));
            if let Err(error) = result {
                self.record_js_error(&error, DiagnosticKind::JavaScriptException);
            }
            self.drain_job_slice(0);
        }

        // Teardown is best-effort but always releases every Persistent root
        // before Runtime/Context destruction, even if a final flush failed.
        let flush_result = self.flush_host_batch();
        if let Ok(mut host) = self.host.try_borrow_mut() {
            host.dispose();
        }
        if let Ok(mut scheduler) = self.scheduler.try_borrow_mut() {
            scheduler.dispose();
        }

        self.runtime.run_gc();
        self.runtime.set_interrupt_handler(None);
        self.runtime.set_host_promise_rejection_tracker(None);
        flush_result
    }
}

fn install_cleanup_binding<'js>(
    ctx: rquickjs::Ctx<'js>,
    cleanup: std::rc::Rc<RefCell<SessionCleanupRegistry>>,
) -> rquickjs::Result<()> {
    let register = Function::new(ctx.clone(), {
        let cleanup = cleanup.clone();
        move |ctx: rquickjs::Ctx<'js>, callback: Function<'js>| -> rquickjs::Result<u32> {
            let mut registry = cleanup.try_borrow_mut().map_err(|_| {
                Exception::throw_internal(&ctx, "Session cleanup registry is already borrowed")
            })?;
            if registry.closed {
                return Err(Exception::throw_type(
                    &ctx,
                    "EngineSession is shutting down",
                ));
            }
            if registry.callbacks.len() >= MAX_SESSION_CLEANUPS {
                return Err(Exception::throw_range(
                    &ctx,
                    "Too many session cleanup callbacks",
                ));
            }
            let id = registry.next_id;
            registry.next_id = registry.next_id.checked_add(1).ok_or_else(|| {
                Exception::throw_range(&ctx, "Session cleanup id space exhausted")
            })?;
            registry
                .callbacks
                .insert(id, Persistent::save(&ctx, callback));
            Ok(id)
        }
    })?;
    ctx.globals()
        .set("__sn_register_session_cleanup", register)?;

    let unregister = Function::new(ctx.clone(), {
        let cleanup = cleanup.clone();
        move |ctx: rquickjs::Ctx<'js>, id: u32| -> rquickjs::Result<bool> {
            let mut registry = cleanup.try_borrow_mut().map_err(|_| {
                Exception::throw_internal(&ctx, "Session cleanup registry is already borrowed")
            })?;
            Ok(registry.callbacks.remove(&id).is_some())
        }
    })?;
    ctx.globals()
        .set("__sn_unregister_session_cleanup", unregister)?;
    Ok(())
}

fn run_event_loop(
    worker: RuntimeWorker,
    receiver: Receiver<Command>,
    handle: &EngineSessionHandle,
) {
    let mut commands_since_job_slice = 0usize;

    loop {
        match receiver.try_recv() {
            Ok(command) => {
                let is_shutdown = matches!(&command, Command::Shutdown(_));
                if !is_shutdown {
                    handle.pending_commands.fetch_sub(1, Ordering::AcqRel);
                    handle
                        .pending_queue_bytes
                        .fetch_sub(command.payload_bytes(), Ordering::AcqRel);
                }
                let response = if handle.closed.load(Ordering::Acquire) && !is_shutdown {
                    reject_closed_command(command, &worker.tasks);
                    None
                } else {
                    process_command(&worker, command)
                };
                if is_shutdown {
                    let result = worker.shutdown();
                    if let Some(response) = response {
                        let _ = response.send(result);
                    }
                    break;
                }

                commands_since_job_slice += 1;
                if commands_since_job_slice < DEFAULT_COMMAND_QUANTUM {
                    continue;
                }
            }
            Err(TryRecvError::Disconnected) => {
                let _ = worker.shutdown();
                break;
            }
            Err(TryRecvError::Empty) => {}
        }

        commands_since_job_slice = 0;

        let now = worker.config.clock.now();
        if worker
            .next_timer_deadline()
            .is_some_and(|deadline| deadline <= now)
        {
            worker.fire_due_timer(0);
            continue;
        }

        if worker.runtime.is_job_pending() {
            worker.drain_job_slice(0);
            continue;
        }

        if handle.closed.load(Ordering::Acquire) {
            let _ = worker.shutdown();
            break;
        }

        let timeout = worker
            .next_timer_deadline()
            .map(|deadline| deadline.saturating_sub(worker.config.clock.now()))
            .unwrap_or(MAX_IDLE_WAIT)
            .min(MAX_IDLE_WAIT);

        match receiver.recv_timeout(timeout) {
            Ok(command) => {
                let is_shutdown = matches!(&command, Command::Shutdown(_));
                if !is_shutdown {
                    handle.pending_commands.fetch_sub(1, Ordering::AcqRel);
                    handle
                        .pending_queue_bytes
                        .fetch_sub(command.payload_bytes(), Ordering::AcqRel);
                }
                let response = if handle.closed.load(Ordering::Acquire) && !is_shutdown {
                    reject_closed_command(command, &worker.tasks);
                    None
                } else {
                    process_command(&worker, command)
                };
                if is_shutdown {
                    let result = worker.shutdown();
                    if let Some(response) = response {
                        let _ = response.send(result);
                    }
                    break;
                }
                commands_since_job_slice = 1;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                let _ = worker.shutdown();
                break;
            }
        }
    }
}

fn process_command(
    worker: &RuntimeWorker,
    command: Command,
) -> Option<SyncSender<Result<(), SessionError>>> {
    match command {
        Command::Evaluate {
            id,
            source,
            response,
        } => {
            let result = finish_task(&worker.tasks, id, worker.evaluate(id, &source));
            let _ = response.send(result);
            None
        }
        Command::EvaluateText {
            id,
            source,
            response,
        } => {
            let result = finish_task(&worker.tasks, id, worker.evaluate_text(id, &source));
            let _ = response.send(result);
            None
        }
        Command::FindElement { id, tag, response } => {
            let result = finish_task(&worker.tasks, id, worker.find_element(id, &tag));
            let _ = response.send(result);
            None
        }
        Command::DispatchEvent {
            id,
            node_id,
            event_name,
            response,
        } => {
            let result = finish_task(
                &worker.tasks,
                id,
                worker.dispatch_event(id, node_id, &event_name),
            );
            let _ = response.send(result);
            None
        }
        Command::Frame {
            id,
            timestamp,
            response,
        } => {
            let result = finish_task(&worker.tasks, id, worker.deliver_frame(id, timestamp));
            let _ = response.send(result);
            None
        }
        Command::Completion {
            id,
            callback_id,
            payload,
            response,
        } => {
            let result = finish_task(
                &worker.tasks,
                id,
                worker.deliver_completion(id, callback_id, &payload),
            );
            let _ = response.send(result);
            None
        }
        Command::CancelTimer {
            id,
            timer_id,
            response,
        } => {
            let result = finish_task(
                &worker.tasks,
                id,
                worker
                    .check_cancelled(id)
                    .and_then(|()| worker.cancel_timer(timer_id)),
            );
            let _ = response.send(result);
            None
        }
        Command::Snapshot { id, response } => {
            let result = finish_task(
                &worker.tasks,
                id,
                worker.check_cancelled(id).and_then(|()| worker.snapshot()),
            );
            let _ = response.send(result);
            None
        }
        Command::Pump { id, response } => {
            let result = finish_task(
                &worker.tasks,
                id,
                worker.check_cancelled(id).map(|()| {
                    let fired = worker.fire_due_timer(id);
                    if worker.runtime.is_job_pending() {
                        worker.drain_job_slice(id);
                    }
                    fired
                }),
            );
            let _ = response.send(result);
            None
        }
        Command::Shutdown(response) => response,
    }
}

fn finish_task<T>(
    tasks: &Arc<Mutex<TaskRegistry>>,
    id: u64,
    result: Result<T, SessionError>,
) -> Result<T, SessionError> {
    lock_recover(tasks).finish(id, result)
}

fn reject_closed_command(command: Command, tasks: &Arc<Mutex<TaskRegistry>>) {
    match command {
        Command::Evaluate { id, response, .. } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::EvaluateText { id, response, .. } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::FindElement { id, response, .. } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::DispatchEvent { id, response, .. } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::Frame { id, response, .. } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::Completion { id, response, .. } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::CancelTimer { id, response, .. } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::Snapshot { id, response } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::Pump { id, response } => {
            let _ = response.send(finish_task(tasks, id, Err(SessionError::Closed)));
        }
        Command::Shutdown(response) => {
            if let Some(response) = response {
                let _ = response.send(Err(SessionError::Closed));
            }
        }
    }
}

fn push_diagnostic(
    diagnostics: &Arc<Mutex<VecDeque<RuntimeDiagnostic>>>,
    mut diagnostic: RuntimeDiagnostic,
) {
    truncate_diagnostic_text(&mut diagnostic.message);
    if let Some(stack) = diagnostic.stack.as_mut() {
        truncate_diagnostic_text(stack);
    }

    let mut entries = lock_recover(diagnostics);
    if entries.len() >= MAX_RETAINED_DIAGNOSTICS {
        entries.pop_front();
    }
    entries.push_back(diagnostic);
}

fn truncate_diagnostic_text(value: &mut String) {
    if value.len() <= MAX_DIAGNOSTIC_TEXT_BYTES {
        return;
    }

    const SUFFIX: &str = "…";
    let mut end = MAX_DIAGNOSTIC_TEXT_BYTES - SUFFIX.len();
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value.push_str(SUFFIX);
}

fn record_exception(
    ctx: &rquickjs::Ctx<'_>,
    error: &rquickjs::Error,
    diagnostics: &Arc<Mutex<VecDeque<RuntimeDiagnostic>>>,
    clock: &Arc<dyn RuntimeClock>,
    kind: DiagnosticKind,
) {
    let (message, stack) = if error.is_exception() && ctx.has_exception() {
        exception_parts(ctx.catch())
    } else {
        (error.to_string(), None)
    };
    push_diagnostic(
        diagnostics,
        RuntimeDiagnostic {
            kind,
            at: clock.now(),
            message,
            stack,
            handled: None,
        },
    );
}

fn exception_parts(value: Value<'_>) -> (String, Option<String>) {
    match Exception::from_value(value) {
        Ok(exception) => {
            let message = exception
                .message()
                .unwrap_or_else(|| "JavaScript Error without a message".into());
            (message, exception.stack())
        }
        Err(_) => ("QuickJS raised a non-Error JavaScript value".into(), None),
    }
}

fn runtime_error(error: impl fmt::Display) -> SessionError {
    SessionError::Runtime(error.to_string())
}

fn lock_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::ManualClock;

    fn config(clock: Arc<dyn RuntimeClock>) -> EngineSessionConfig {
        EngineSessionConfig {
            clock,
            ..EngineSessionConfig::default()
        }
    }

    #[test]
    fn engine_session_owns_quickjs_on_a_dedicated_thread() {
        let session = EngineSession::start(config(Arc::new(MonotonicClock::default()))).unwrap();
        let handle = session.handle();
        assert!(!handle.is_owner_thread());

        handle
            .evaluate("globalThis.ready = true;")
            .unwrap()
            .wait()
            .unwrap();
        assert_eq!(
            handle
                .evaluate_text("String(ready)")
                .unwrap()
                .wait()
                .unwrap(),
            "true"
        );
        session.shutdown().unwrap();
    }

    #[test]
    fn promise_jobs_and_queue_microtasks_run_in_quickjs_order() {
        let session = EngineSession::start(config(Arc::new(MonotonicClock::default()))).unwrap();
        let handle = session.handle();
        handle.evaluate(
            "globalThis.order = []; order.push('sync'); Promise.resolve().then(() => order.push('promise')); queueMicrotask(() => order.push('microtask')); order.push('end');",
        ).unwrap().wait().unwrap();
        assert_eq!(
            handle
                .evaluate_text("order.join(',')")
                .unwrap()
                .wait()
                .unwrap(),
            "sync,end,promise,microtask"
        );
        session.shutdown().unwrap();
    }

    #[test]
    fn cancelled_timeout_does_not_fire_and_shutdown_releases_roots() {
        let clock = Arc::new(ManualClock::default());
        let session = EngineSession::start(config(clock.clone())).unwrap();
        let handle = session.handle();
        handle
            .evaluate(
                "globalThis.timerHits = 0; globalThis.timerId = setTimeout(() => timerHits++, 5);",
            )
            .unwrap()
            .wait()
            .unwrap();
        assert_eq!(
            handle
                .evaluate_text("String(timerId)")
                .unwrap()
                .wait()
                .unwrap(),
            "1"
        );
        assert!(handle.cancel_timer(1).unwrap().wait().unwrap());
        clock.advance(Duration::from_millis(10));
        assert!(!handle.pump().unwrap().wait().unwrap());
        assert_eq!(
            handle
                .evaluate_text("String(timerHits)")
                .unwrap()
                .wait()
                .unwrap(),
            "0"
        );
        session.shutdown().unwrap();
    }

    #[test]
    fn due_timeout_fires_after_manual_clock_advance() {
        let clock = Arc::new(ManualClock::default());
        let session = EngineSession::start(config(clock.clone())).unwrap();
        let handle = session.handle();
        handle
            .evaluate("globalThis.timerHits = 0; setTimeout(() => timerHits++, 5);")
            .unwrap()
            .wait()
            .unwrap();
        clock.advance(Duration::from_millis(5));
        let _ = handle.pump().unwrap().wait().unwrap();
        assert_eq!(
            handle
                .evaluate_text("String(timerHits)")
                .unwrap()
                .wait()
                .unwrap(),
            "1"
        );
        session.shutdown().unwrap();
    }

    #[test]
    fn animation_frame_callbacks_are_ordered_and_cancellable() {
        let session = EngineSession::start(config(Arc::new(MonotonicClock::default()))).unwrap();
        let handle = session.handle();
        handle.evaluate(
            "globalThis.frames = []; requestAnimationFrame(t => frames.push('first:' + t)); const cancelled = requestAnimationFrame(() => frames.push('cancelled')); cancelAnimationFrame(cancelled); requestAnimationFrame(t => frames.push('last:' + t));",
        ).unwrap().wait().unwrap();
        assert!(handle.frame_requested());
        assert_eq!(
            handle
                .deliver_frame(Duration::from_millis(16))
                .unwrap()
                .wait()
                .unwrap(),
            2
        );
        assert!(!handle.frame_requested());
        assert_eq!(
            handle
                .evaluate_text("frames.join(',')")
                .unwrap()
                .wait()
                .unwrap(),
            "first:16,last:16"
        );
        session.shutdown().unwrap();
    }

    #[test]
    fn completion_callback_is_one_shot_and_receives_owned_bytes() {
        let session = EngineSession::start(config(Arc::new(MonotonicClock::default()))).unwrap();
        let handle = session.handle();
        handle.evaluate(
            "globalThis.completionBytes = 0; __sn_register_completion_callback(7, bytes => { completionBytes = bytes.length; });",
        ).unwrap().wait().unwrap();
        assert!(
            handle
                .post_completion(7, vec![1, 2, 3])
                .unwrap()
                .wait()
                .unwrap()
        );
        assert!(!handle.post_completion(7, vec![1]).unwrap().wait().unwrap());
        assert_eq!(
            handle
                .evaluate_text("String(completionBytes)")
                .unwrap()
                .wait()
                .unwrap(),
            "3"
        );
        session.shutdown().unwrap();
    }

    #[test]
    fn job_budget_yields_and_unhandled_rejections_are_diagnosed() {
        let mut config = config(Arc::new(MonotonicClock::default()));
        config.max_jobs_per_slice = 4;
        config.max_job_slice_time = Duration::from_secs(1);
        let session = EngineSession::start(config).unwrap();
        let handle = session.handle();
        handle.evaluate(
            "globalThis.chainCount = 0; const step = () => { chainCount++; Promise.resolve().then(step); }; Promise.resolve().then(step);",
        ).unwrap().wait().unwrap();
        assert!(
            handle
                .take_diagnostics()
                .iter()
                .any(|entry| { entry.kind == DiagnosticKind::SchedulerBudgetExhausted })
        );

        handle
            .evaluate("Promise.reject(new Error('expected rejection'));")
            .unwrap()
            .wait()
            .unwrap();
        assert!(handle.take_diagnostics().iter().any(|entry| {
            entry.kind == DiagnosticKind::PromiseRejection
                && entry.handled == Some(false)
                && entry.message.contains("expected rejection")
        }));
        session.shutdown().unwrap();
    }

    #[test]
    fn diagnostic_text_is_bounded_without_splitting_utf8() {
        let diagnostics = Arc::new(Mutex::new(VecDeque::new()));
        push_diagnostic(
            &diagnostics,
            RuntimeDiagnostic {
                kind: DiagnosticKind::JavaScriptException,
                at: Duration::ZERO,
                message: "Ж".repeat(MAX_DIAGNOSTIC_TEXT_BYTES),
                stack: Some("λ".repeat(MAX_DIAGNOSTIC_TEXT_BYTES)),
                handled: None,
            },
        );

        let entries = lock_recover(&diagnostics);
        let entry = entries.front().unwrap();
        assert!(entry.message.len() <= MAX_DIAGNOSTIC_TEXT_BYTES);
        assert!(entry.stack.as_ref().unwrap().len() <= MAX_DIAGNOSTIC_TEXT_BYTES);
        assert!(entry.message.ends_with('…'));
        assert!(entry.stack.as_ref().unwrap().ends_with('…'));
    }

    #[test]
    fn queued_script_payload_respects_the_configured_byte_budget() {
        let mut config = config(Arc::new(MonotonicClock::default()));
        config.max_queued_bytes = 8;
        let session = EngineSession::start(config).unwrap();
        let handle = session.handle();

        assert!(matches!(
            handle.evaluate("globalThis.value = 123;"),
            Err(SessionError::PayloadTooLarge { actual, limit: 8 }) if actual > 8
        ));
        assert_eq!(handle.evaluate_text("''").unwrap().wait().unwrap(), "");
        session.shutdown().unwrap();
    }

    #[test]
    fn task_registry_rejects_cancellation_after_completion_without_leaking_ids() {
        let mut tasks = TaskRegistry::default();
        assert!(!tasks.cancel(1));

        tasks.register(1);
        assert!(tasks.cancel(1));
        assert_eq!(tasks.finish(1, Ok(42)), Err(SessionError::Cancelled));
        assert!(!tasks.cancel(1));
        assert!(tasks.active.is_empty());
        assert!(tasks.cancelled.is_empty());
    }

    #[test]
    fn session_shutdown_interrupts_a_running_script_and_still_runs_cleanup() {
        let mut config = config(Arc::new(MonotonicClock::default()));
        config.max_execution_time = Duration::from_secs(5);
        let session = EngineSession::start(config).unwrap();
        let handle = session.handle();
        let ticket = handle
            .evaluate("__sn_register_session_cleanup(() => { throw new Error('shutdown cleanup ran'); }); while (true) {}")
            .unwrap();

        let start_deadline = Instant::now() + Duration::from_secs(2);
        while handle.active_turn.load(Ordering::Acquire) != ticket.task_id() {
            assert!(
                Instant::now() < start_deadline,
                "script did not enter its active JS turn"
            );
            thread::yield_now();
        }

        let started = Instant::now();
        session.shutdown().unwrap();
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(ticket.wait(), Err(SessionError::Closed));
        assert!(handle.take_diagnostics().iter().any(|diagnostic| {
            diagnostic.kind == DiagnosticKind::JavaScriptException
                && diagnostic.message.contains("shutdown cleanup ran")
        }));
    }

    #[test]
    fn cancelled_running_script_is_interrupted_on_the_owner_thread() {
        let mut config = config(Arc::new(MonotonicClock::default()));
        config.max_execution_time = Duration::from_secs(5);
        let session = EngineSession::start(config).unwrap();
        let handle = session.handle();

        let ticket = handle.evaluate("while (true) {}").unwrap();
        let start_deadline = Instant::now() + Duration::from_secs(2);
        while handle.active_turn.load(Ordering::Acquire) != ticket.task_id() {
            assert!(
                Instant::now() < start_deadline,
                "script did not enter its active JS turn"
            );
            thread::yield_now();
        }

        assert!(ticket.cancel());
        assert_eq!(ticket.wait(), Err(SessionError::Cancelled));

        session.shutdown().unwrap();
    }

    #[test]
    fn session_cleanup_roots_unregister_and_shutdown_in_lifo_order() {
        let session = EngineSession::start(config(Arc::new(MonotonicClock::default()))).unwrap();
        let handle = session.handle();
        handle.evaluate(
            "globalThis.firstCleanup = __sn_register_session_cleanup(() => { throw new Error('first cleanup'); }); const removed = __sn_register_session_cleanup(() => { throw new Error('must not run'); }); if (!__sn_unregister_session_cleanup(removed)) throw new Error('cleanup unregister failed'); __sn_register_session_cleanup(() => { throw new Error('second cleanup'); });",
        ).unwrap().wait().unwrap();

        session.shutdown().unwrap();
        let messages: Vec<_> = handle
            .take_diagnostics()
            .into_iter()
            .filter(|entry| entry.kind == DiagnosticKind::JavaScriptException)
            .map(|entry| entry.message)
            .collect();
        assert_eq!(messages, vec!["second cleanup", "first cleanup"]);
    }
}
