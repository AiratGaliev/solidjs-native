use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use rquickjs::{Ctx, Exception, Function, Object, Persistent, TypedArray, Value};

use sn_core::{
    CommitAck, HandlerId, HostBatch, HostOperation, MAX_BATCH_BYTES, NodeId, ProtocolError,
    SessionGeneration, SurfaceId, TreeError, UiTree, decode_batch,
};

#[cfg(test)]
use sn_core::BatchHeader;

pub type SharedHostState = Rc<RefCell<HostState>>;

const MAX_RETAINED_HEADLESS_COMMITS: usize = 64;
const MAX_RETAINED_HEADLESS_BYTES: usize = MAX_BATCH_BYTES;

#[derive(Debug, Default)]
pub struct HeadlessMountSink {
    commits: VecDeque<CommitAck>,
    retained_bytes: usize,
}

impl HeadlessMountSink {
    fn commit(&mut self, ack: CommitAck) {
        let ack_bytes = estimate_ack_bytes(&ack);
        if ack_bytes > MAX_RETAINED_HEADLESS_BYTES {
            return;
        }

        while self.commits.len() >= MAX_RETAINED_HEADLESS_COMMITS
            || self.retained_bytes.saturating_add(ack_bytes) > MAX_RETAINED_HEADLESS_BYTES
        {
            let Some(removed) = self.commits.pop_front() else {
                break;
            };
            self.retained_bytes = self
                .retained_bytes
                .saturating_sub(estimate_ack_bytes(&removed));
        }

        self.retained_bytes += ack_bytes;
        self.commits.push_back(ack);
    }

    pub fn commits(&self) -> &VecDeque<CommitAck> {
        &self.commits
    }
}

fn estimate_ack_bytes(ack: &CommitAck) -> usize {
    use std::mem::size_of;

    let inline_bytes = ack.delta.len().saturating_mul(size_of::<HostOperation>());
    ack.delta.iter().fold(inline_bytes, |total, operation| {
        total.saturating_add(operation_string_bytes(operation))
    })
}

fn operation_string_bytes(operation: &HostOperation) -> usize {
    match operation {
        HostOperation::CreateElement { tag, .. } => tag.len(),
        HostOperation::CreateText { text, .. } | HostOperation::ReplaceText { text, .. } => {
            text.len()
        }
        HostOperation::SetProperty { name, value, .. } => {
            name.len()
                + match value {
                    sn_core::PropertyValue::String(value) => value.len(),
                    _ => 0,
                }
        }
        HostOperation::RemoveProperty { name, .. }
        | HostOperation::AttachResource { name, .. }
        | HostOperation::DetachResource { name, .. } => name.len(),
        HostOperation::BindEvent { event, .. } | HostOperation::UnbindEvent { event, .. } => {
            event.len()
        }
        HostOperation::InsertNode { .. } | HostOperation::RemoveNode { .. } => 0,
    }
}

pub struct HostState {
    pub tree: UiTree,
    callbacks: HashMap<HandlerId, Persistent<Function<'static>>>,
    next_handler_id: u32,
    sink: HeadlessMountSink,
    disposed: bool,
}

impl HostState {
    pub fn new(surface_id: SurfaceId, session_generation: SessionGeneration) -> Self {
        Self {
            tree: UiTree::new(surface_id, session_generation),
            callbacks: HashMap::new(),
            next_handler_id: 1,
            sink: HeadlessMountSink::default(),
            disposed: false,
        }
    }

    pub fn callback_count(&self) -> usize {
        self.callbacks.len()
    }

    pub fn sink(&self) -> &HeadlessMountSink {
        &self.sink
    }

    fn register_callback<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        callback: Function<'js>,
    ) -> rquickjs::Result<u32> {
        if self.disposed {
            return Err(Exception::throw_internal(
                ctx,
                "Surface session has been disposed",
            ));
        }
        let id = self.next_handler_id;
        self.next_handler_id = self
            .next_handler_id
            .checked_add(1)
            .ok_or_else(|| Exception::throw_range(ctx, "Callback handler id space exhausted"))?;
        self.callbacks
            .insert(HandlerId::new(id), Persistent::save(ctx, callback));
        Ok(id)
    }

    fn apply_encoded(&mut self, bytes: &[u8]) -> Result<CommitAck, HostApplyError> {
        if self.disposed {
            return Err(HostApplyError::Disposed);
        }
        let batch = match decode_batch(bytes) {
            Ok(batch) => batch,
            Err(error) => {
                self.prune_callbacks();
                return Err(HostApplyError::Protocol(error));
            }
        };
        self.commit_batch(&batch)
    }

    fn commit_batch(&mut self, batch: &HostBatch) -> Result<CommitAck, HostApplyError> {
        if self.disposed {
            return Err(HostApplyError::Disposed);
        }
        for operation in &batch.operations {
            if let HostOperation::BindEvent { handler_id, .. } = operation {
                if !self.callbacks.contains_key(handler_id) {
                    self.prune_callbacks();
                    return Err(HostApplyError::Tree(TreeError::MissingHandler(
                        handler_id.get(),
                    )));
                }
            }
        }

        match self.tree.apply_batch(batch) {
            Ok(ack) => {
                self.prune_callbacks();
                self.sink.commit(ack.clone());
                Ok(ack)
            }
            Err(error) => {
                self.prune_callbacks();
                Err(HostApplyError::Tree(error))
            }
        }
    }

    fn callback_for_event(
        &self,
        node_id: NodeId,
        event_name: &str,
    ) -> Option<Persistent<Function<'static>>> {
        let handler_id = self
            .tree
            .node(node_id)
            .ok()?
            .events
            .get(event_name)
            .copied()?;
        self.callbacks.get(&handler_id).cloned()
    }

    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.tree.dispose_session();
        self.callbacks.clear();
        self.disposed = true;
    }

    fn prune_callbacks(&mut self) {
        let active = self.tree.handler_ids();
        self.callbacks.retain(|id, _| active.contains(id));
    }
}

#[derive(Debug)]
enum HostApplyError {
    Protocol(ProtocolError),
    Tree(TreeError),
    Disposed,
}

impl std::fmt::Display for HostApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(error) => write!(f, "{error}"),
            Self::Tree(error) => write!(f, "{error}"),
            Self::Disposed => write!(f, "Surface session has been disposed"),
        }
    }
}

pub fn install_host_bindings<'js>(ctx: Ctx<'js>, state: SharedHostState) -> rquickjs::Result<()> {
    let (surface_id, session_generation) = {
        let host = state
            .try_borrow()
            .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
        (
            host.tree.surface_id().get(),
            host.tree.session_generation().get(),
        )
    };

    let register_callback = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>, callback: Function<'js>| -> rquickjs::Result<u32> {
            let mut host = state
                .try_borrow_mut()
                .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
            host.register_callback(&ctx, callback)
        }
    })?;
    ctx.globals()
        .set("__sn_register_callback", register_callback)?;

    let discard_pending_callbacks = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>| -> rquickjs::Result<()> {
            let mut host = state
                .try_borrow_mut()
                .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
            host.prune_callbacks();
            Ok(())
        }
    })?;
    ctx.globals()
        .set("__sn_discard_pending_callbacks", discard_pending_callbacks)?;

    let apply_batch = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>, bytes: TypedArray<'js, u8>| -> rquickjs::Result<Object<'js>> {
            if bytes.len() > MAX_BATCH_BYTES {
                let mut host = state.try_borrow_mut().map_err(|_| {
                    Exception::throw_internal(&ctx, "Host state is already borrowed")
                })?;
                host.prune_callbacks();
                return Err(Exception::throw_range(
                    &ctx,
                    "Host batch exceeds the byte limit",
                ));
            }
            // QuickJS execution is single-threaded for this context. Copy the
            // typed array synchronously; no JS backing-store reference escapes.
            let owned = unsafe { bytes.as_slice() }.to_vec();
            let mut host = state
                .try_borrow_mut()
                .map_err(|_| Exception::throw_internal(&ctx, "Host state is already borrowed"))?;
            let ack = host
                .apply_encoded(&owned)
                .map_err(|error| Exception::throw_type(&ctx, &error.to_string()))?;

            let result = Object::new(ctx.clone())?;
            result.set("ok", true)?;
            result.set("surfaceId", ack.surface_id.get())?;
            result.set("sessionGeneration", ack.session_generation.get())?;
            result.set("transactionId", ack.transaction_id)?;
            result.set("revision", ack.revision)?;
            result.set("nextNodeId", ack.next_node_id)?;
            Ok(result)
        }
    })?;
    ctx.globals().set("__sn_apply_batch", apply_batch)?;

    let dispatch_event = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>, node_id: u64, event_name: String| -> rquickjs::Result<bool> {
            dispatch_event(&ctx, &state, NodeId::new(node_id), &event_name)
        }
    })?;
    ctx.globals().set("__sn_dispatch_event", dispatch_event)?;

    let dispose = Function::new(ctx.clone(), move || -> rquickjs::Result<()> {
        let mut host = state
            .try_borrow_mut()
            .map_err(|_| rquickjs::Error::Unknown)?;
        host.dispose();
        Ok(())
    })?;
    ctx.globals().set("__sn_dispose", dispose)?;

    let config = Object::new(ctx.clone())?;
    config.set("surfaceId", surface_id)?;
    config.set("sessionGeneration", session_generation)?;
    ctx.globals().set("__sn_host_config", config)?;

    Ok(())
}

pub fn dispatch_event<'js>(
    ctx: &Ctx<'js>,
    state: &SharedHostState,
    node_id: NodeId,
    event_name: &str,
) -> rquickjs::Result<bool> {
    // Clone the persistent root and release the RefCell borrow before entering
    // JavaScript. Event handlers may synchronously cause new renderer bindings.
    let persistent = {
        let host = state
            .try_borrow()
            .map_err(|_| Exception::throw_internal(ctx, "Host state is already borrowed"))?;
        host.callback_for_event(node_id, event_name)
    };
    let Some(persistent) = persistent else {
        return Ok(false);
    };
    let callback = persistent.restore(ctx)?;
    let event = Object::new(ctx.clone())?;
    event.set("type", event_name)?;
    event.set("targetNodeId", node_id.get())?;
    callback.call::<_, ()>((event,))?;
    Ok(true)
}

pub fn dispatch_event_and_flush<'js>(
    ctx: &Ctx<'js>,
    state: &SharedHostState,
    node_id: NodeId,
    event_name: &str,
) -> rquickjs::Result<bool> {
    let dispatched = dispatch_event(ctx, state, node_id, event_name)?;
    if dispatched {
        let flush: Function = ctx.globals().get("__sn_flush_host_batch")?;
        let _: Value<'js> = flush.call(())?;
    }
    Ok(dispatched)
}

// Kept here as an explicit helper for tests and transaction construction.
#[cfg(test)]
fn event_batch(tree: &UiTree, transaction_id: u64, operations: Vec<HostOperation>) -> HostBatch {
    HostBatch {
        header: BatchHeader {
            major: 1,
            minor: 0,
            surface_id: tree.surface_id().get(),
            session_generation: tree.session_generation().get(),
            transaction_id,
            expected_revision: tree.revision(),
        },
        operations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Context, Runtime};

    #[test]
    fn host_encoder_config_uses_the_actual_surface_session_identity() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let state: SharedHostState = Rc::new(RefCell::new(HostState::new(
            SurfaceId::new(17),
            SessionGeneration::new(23),
        )));

        context
            .with(|ctx| -> rquickjs::Result<()> {
                install_host_bindings(ctx.clone(), state.clone())?;
                let config: Object = ctx.globals().get("__sn_host_config")?;
                let surface_id: u64 = config.get("surfaceId")?;
                let session_generation: u64 = config.get("sessionGeneration")?;

                assert_eq!(surface_id, 17);
                assert_eq!(session_generation, 23);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn callback_registry_dispatches_and_returns_to_baseline_after_remove() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let state: SharedHostState = Rc::new(RefCell::new(HostState::new(
            SurfaceId::new(1),
            SessionGeneration::new(1),
        )));

        context.with(|ctx| -> rquickjs::Result<()> {
            install_host_bindings(ctx.clone(), state.clone())?;
            ctx.eval::<(), _>("globalThis.__hits = 0")?;

            for cycle in 0..12u64 {
                let handler_id: u32 = ctx.eval("globalThis.__sn_register_callback((event) => { if (event.type === 'click') globalThis.__hits += 1; })")?;
                let mut host = state.borrow_mut();
                let root = NodeId::new(host.tree.next_node_id());
                let button = NodeId::new(root.get() + 1);
                let bind = event_batch(&host.tree, cycle * 2 + 1, vec![
                    HostOperation::CreateElement { id: root, tag: "root".into() },
                    HostOperation::CreateElement { id: button, tag: "button".into() },
                    HostOperation::InsertNode { parent: root, child: button, anchor: None },
                    HostOperation::BindEvent { id: button, event: "click".into(), handler_id: HandlerId::new(handler_id) },
                ]);
                host.commit_batch(&bind).unwrap();
                drop(host);

                let dispatch: Function = ctx.globals().get("__sn_dispatch_event")?;
                let handled: bool = dispatch.call((button.get(), "click"))?;
                assert!(handled);
                let hits: u32 = ctx.globals().get("__hits")?;
                assert_eq!(hits, cycle as u32 + 1);

                let mut host = state.borrow_mut();
                let remove = event_batch(&host.tree, cycle * 2 + 2, vec![HostOperation::RemoveNode { id: root }]);
                host.commit_batch(&remove).unwrap();
                assert_eq!(host.callback_count(), 0);
            }
            assert_eq!(state.borrow().sink().commits().len(), 24);
            Ok(())
        }).unwrap();
    }

    #[test]
    fn event_without_ui_mutations_accepts_an_empty_flush() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let state: SharedHostState = Rc::new(RefCell::new(HostState::new(
            SurfaceId::new(3),
            SessionGeneration::new(5),
        )));

        context
            .with(|ctx| -> rquickjs::Result<()> {
                install_host_bindings(ctx.clone(), state.clone())?;
                ctx.eval::<(), _>("globalThis.__event_hits = 0; globalThis.__sn_flush_host_batch = () => undefined")?;
                let handler_id: u32 = ctx.eval(
                    "globalThis.__sn_register_callback(() => { globalThis.__event_hits += 1; })",
                )?;

                {
                    let mut host = state.borrow_mut();
                    let root = NodeId::new(1);
                    let batch = event_batch(
                        &host.tree,
                        1,
                        vec![
                            HostOperation::CreateElement {
                                id: root,
                                tag: "button".into(),
                            },
                            HostOperation::BindEvent {
                                id: root,
                                event: "click".into(),
                                handler_id: HandlerId::new(handler_id),
                            },
                        ],
                    );
                    host.commit_batch(&batch).unwrap();
                }

                assert!(dispatch_event_and_flush(
                    &ctx,
                    &state,
                    NodeId::new(1),
                    "click",
                )?);
                let hits: u32 = ctx.globals().get("__event_hits")?;
                assert_eq!(hits, 1);
                assert_eq!(state.borrow().sink().commits().len(), 1);

                state.borrow_mut().dispose();
                assert_eq!(state.borrow().callback_count(), 0);
                assert_eq!(state.borrow().tree.node_count(), 0);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn headless_mount_sink_retains_a_bounded_commit_history() {
        let mut sink = HeadlessMountSink::default();
        for transaction_id in 1..=(MAX_RETAINED_HEADLESS_COMMITS as u64 + 5) {
            sink.commit(CommitAck {
                surface_id: SurfaceId::new(1),
                session_generation: SessionGeneration::new(1),
                transaction_id,
                revision: transaction_id,
                next_node_id: 1,
                delta: Vec::new(),
            });
        }

        assert_eq!(sink.commits().len(), MAX_RETAINED_HEADLESS_COMMITS);
        assert_eq!(
            sink.commits().front().map(|ack| ack.transaction_id),
            Some(6)
        );
        assert_eq!(
            sink.commits().back().map(|ack| ack.transaction_id),
            Some(69)
        );
    }

    #[test]
    fn headless_mount_sink_enforces_a_total_retained_byte_budget() {
        let mut sink = HeadlessMountSink::default();
        let large_text = "x".repeat(1024 * 1024);

        for transaction_id in 1..=5 {
            sink.commit(CommitAck {
                surface_id: SurfaceId::new(1),
                session_generation: SessionGeneration::new(1),
                transaction_id,
                revision: transaction_id,
                next_node_id: 1,
                delta: vec![HostOperation::CreateText {
                    id: NodeId::new(transaction_id),
                    text: large_text.clone(),
                }],
            });
        }

        assert!(sink.retained_bytes <= MAX_RETAINED_HEADLESS_BYTES);
        assert!(sink.commits().len() < 5);
        assert_eq!(sink.commits().back().map(|ack| ack.transaction_id), Some(5));
    }

    #[test]
    fn encoding_failure_can_release_uncommitted_callback_roots() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let state: SharedHostState = Rc::new(RefCell::new(HostState::new(
            SurfaceId::new(1),
            SessionGeneration::new(1),
        )));

        context
            .with(|ctx| -> rquickjs::Result<()> {
                install_host_bindings(ctx.clone(), state.clone())?;
                let _: u32 = ctx.eval("globalThis.__sn_register_callback(() => {})")?;
                assert_eq!(state.borrow().callback_count(), 1);

                let discard: Function = ctx.globals().get("__sn_discard_pending_callbacks")?;
                discard.call::<_, ()>(())?;
                assert_eq!(state.borrow().callback_count(), 0);
                assert_eq!(state.borrow().tree.revision(), 0);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn failed_batch_discards_pending_callback_roots_and_preserves_tree() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let state: SharedHostState = Rc::new(RefCell::new(HostState::new(
            SurfaceId::new(1),
            SessionGeneration::new(1),
        )));

        context
            .with(|ctx| -> rquickjs::Result<()> {
                install_host_bindings(ctx.clone(), state.clone())?;
                let handler_id: u32 = ctx.eval("globalThis.__sn_register_callback(() => {})")?;
                let mut host = state.borrow_mut();
                let root = NodeId::new(1);
                let batch = event_batch(
                    &host.tree,
                    1,
                    vec![
                        HostOperation::CreateElement {
                            id: root,
                            tag: "root".into(),
                        },
                        HostOperation::BindEvent {
                            id: NodeId::new(99),
                            event: "click".into(),
                            handler_id: HandlerId::new(handler_id),
                        },
                    ],
                );
                assert!(host.commit_batch(&batch).is_err());
                assert_eq!(host.tree.node_count(), 0);
                assert_eq!(host.tree.revision(), 0);
                assert_eq!(host.callback_count(), 0);
                Ok(())
            })
            .unwrap();
    }
}
