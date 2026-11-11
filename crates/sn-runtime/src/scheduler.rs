use std::{
    cell::RefCell,
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap, HashMap},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use rquickjs::{Ctx, Exception, Function, Object, Persistent};

use crate::clock::RuntimeClock;

const MAX_PENDING_TIMERS: usize = 4_096;
const MAX_TIMER_QUEUE_ENTRIES: usize = 8_192;
const MAX_PENDING_FRAME_CALLBACKS: usize = 4_096;
const MAX_PENDING_COMPLETIONS: usize = 4_096;
const MAX_TIMER_DELAY_MS: f64 = 2_147_483_647.0;
const MIN_TIMER_RESOLUTION: Duration = Duration::from_millis(1);

type PersistentCallback = Persistent<Function<'static>>;

struct TimerEntry {
    deadline: Duration,
    interval: Option<Duration>,
    callback: PersistentCallback,
}

/// All values in this registry are created, accessed and dropped on the
/// owning QuickJS thread. No QuickJS handle crosses the engine-thread boundary.
pub(crate) struct SchedulerState {
    next_id: u32,
    timers: HashMap<u32, TimerEntry>,
    deadlines: BinaryHeap<Reverse<(Duration, u32)>>,
    frame_callbacks: BTreeMap<u32, PersistentCallback>,
    completion_callbacks: HashMap<u64, PersistentCallback>,
    frame_requested: Arc<AtomicBool>,
}

impl SchedulerState {
    pub(crate) fn new(frame_requested: Arc<AtomicBool>) -> Self {
        Self {
            next_id: 1,
            timers: HashMap::new(),
            deadlines: BinaryHeap::new(),
            frame_callbacks: BTreeMap::new(),
            completion_callbacks: HashMap::new(),
            frame_requested,
        }
    }

    fn allocate_id(&mut self, ctx: &Ctx<'_>) -> rquickjs::Result<u32> {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| Exception::throw_range(ctx, "Scheduler callback id space exhausted"))?;
        Ok(id)
    }

    fn set_timer<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        callback: Function<'js>,
        delay_ms: f64,
        repeating: bool,
        now: Duration,
    ) -> rquickjs::Result<u32> {
        if self.deadlines.len() >= MAX_TIMER_QUEUE_ENTRIES {
            self.rebuild_deadlines();
        }
        if self.deadlines.len() >= MAX_TIMER_QUEUE_ENTRIES {
            return Err(Exception::throw_range(ctx, "Timer deadline queue is full"));
        }
        if self.timers.len() >= MAX_PENDING_TIMERS {
            return Err(Exception::throw_range(ctx, "Too many pending timers"));
        }

        let mut delay = normalized_delay(delay_ms);
        if delay < MIN_TIMER_RESOLUTION {
            delay = MIN_TIMER_RESOLUTION;
        }

        let id = self.allocate_id(ctx)?;
        let deadline = now.saturating_add(delay);
        let interval = repeating.then_some(delay);
        self.timers.insert(
            id,
            TimerEntry {
                deadline,
                interval,
                callback: Persistent::save(ctx, callback),
            },
        );
        self.deadlines.push(Reverse((deadline, id)));
        Ok(id)
    }

    pub(crate) fn clear_timer(&mut self, id: u32) -> bool {
        self.timers.remove(&id).is_some()
    }

    fn request_frame<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        callback: Function<'js>,
    ) -> rquickjs::Result<u32> {
        if self.frame_callbacks.len() >= MAX_PENDING_FRAME_CALLBACKS {
            return Err(Exception::throw_range(
                ctx,
                "Too many pending animation frame callbacks",
            ));
        }
        let id = self.allocate_id(ctx)?;
        self.frame_callbacks
            .insert(id, Persistent::save(ctx, callback));
        self.frame_requested.store(true, Ordering::Release);
        Ok(id)
    }

    pub(crate) fn cancel_frame(&mut self, id: u32) -> bool {
        let removed = self.frame_callbacks.remove(&id).is_some();
        if self.frame_callbacks.is_empty() {
            self.frame_requested.store(false, Ordering::Release);
        }
        removed
    }

    pub(crate) fn begin_frame(&mut self) -> Option<u32> {
        let cutoff = self.frame_callbacks.keys().next_back().copied()?;
        // Callbacks requested from inside this frame get larger ids and remain
        // queued for the next platform frame.
        self.frame_requested.store(false, Ordering::Release);
        Some(cutoff)
    }

    pub(crate) fn take_next_frame_callback(
        &mut self,
        cutoff: u32,
    ) -> Option<(u32, PersistentCallback)> {
        let id = self.frame_callbacks.keys().next().copied()?;
        if id > cutoff {
            return None;
        }
        self.frame_callbacks
            .remove(&id)
            .map(|callback| (id, callback))
    }

    fn register_completion<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        callback_id: u64,
        callback: Function<'js>,
    ) -> rquickjs::Result<()> {
        if callback_id == 0 {
            return Err(Exception::throw_range(
                ctx,
                "Completion callback id must be non-zero",
            ));
        }
        if self.completion_callbacks.contains_key(&callback_id) {
            return Err(Exception::throw_type(
                ctx,
                "Completion callback id is already registered",
            ));
        }
        if self.completion_callbacks.len() >= MAX_PENDING_COMPLETIONS {
            return Err(Exception::throw_range(
                ctx,
                "Too many pending completion callbacks",
            ));
        }
        self.completion_callbacks
            .insert(callback_id, Persistent::save(ctx, callback));
        Ok(())
    }

    pub(crate) fn take_completion(&mut self, callback_id: u64) -> Option<PersistentCallback> {
        self.completion_callbacks.remove(&callback_id)
    }

    pub(crate) fn take_due_timer(&mut self, now: Duration) -> Option<(u32, PersistentCallback)> {
        self.discard_stale_deadlines();
        let Reverse((deadline, id)) = *self.deadlines.peek()?;
        if deadline > now {
            return None;
        }
        self.deadlines.pop();

        let entry = self.timers.remove(&id)?;
        if let Some(interval) = entry.interval {
            // Fixed-delay policy: base the next deadline on dispatch time.
            // Missed intervals are skipped rather than replayed in a burst.
            let next_deadline = now.saturating_add(interval);
            let callback = entry.callback.clone();
            self.timers.insert(
                id,
                TimerEntry {
                    deadline: next_deadline,
                    interval: Some(interval),
                    callback: entry.callback,
                },
            );
            self.deadlines.push(Reverse((next_deadline, id)));
            Some((id, callback))
        } else {
            Some((id, entry.callback))
        }
    }

    pub(crate) fn next_deadline(&mut self) -> Option<Duration> {
        self.discard_stale_deadlines();
        self.deadlines.peek().map(|entry| entry.0.0)
    }

    fn rebuild_deadlines(&mut self) {
        self.deadlines = self
            .timers
            .iter()
            .map(|(id, entry)| Reverse((entry.deadline, *id)))
            .collect();
    }

    fn discard_stale_deadlines(&mut self) {
        while let Some(Reverse((deadline, id))) = self.deadlines.peek().copied() {
            let is_current = self
                .timers
                .get(&id)
                .is_some_and(|entry| entry.deadline == deadline);
            if is_current {
                break;
            }
            self.deadlines.pop();
        }
    }

    pub(crate) fn cancel_timers_and_frames(&mut self) {
        self.timers.clear();
        self.deadlines.clear();
        self.frame_callbacks.clear();
        self.frame_requested.store(false, Ordering::Release);
    }

    pub(crate) fn dispose(&mut self) {
        self.cancel_timers_and_frames();
        self.completion_callbacks.clear();
    }

    #[cfg(test)]
    fn pending_counts(&self) -> (usize, usize, usize) {
        (
            self.timers.len(),
            self.frame_callbacks.len(),
            self.completion_callbacks.len(),
        )
    }
}

fn normalized_delay(delay_ms: f64) -> Duration {
    let delay_ms = if delay_ms.is_nan() {
        0.0
    } else {
        delay_ms.clamp(0.0, MAX_TIMER_DELAY_MS)
    };
    Duration::from_millis(delay_ms.trunc() as u64)
}

/// Installs the host timers and frame APIs into one QuickJS context.
pub(crate) fn install_bindings<'js>(
    ctx: Ctx<'js>,
    state: Rc<RefCell<SchedulerState>>,
    clock: Arc<dyn RuntimeClock>,
) -> rquickjs::Result<()> {
    let set_timer = Function::new(ctx.clone(), {
        let state = state.clone();
        let clock = clock.clone();
        move |ctx: Ctx<'js>,
              callback: Function<'js>,
              delay_ms: f64,
              repeating: bool|
              -> rquickjs::Result<u32> {
            let mut scheduler = state.try_borrow_mut().map_err(|_| {
                Exception::throw_internal(&ctx, "Scheduler state is already borrowed")
            })?;
            scheduler.set_timer(&ctx, callback, delay_ms, repeating, clock.now())
        }
    })?;
    ctx.globals().set("__sn_set_timer", set_timer)?;

    let clear_timer = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>, id: u32| -> rquickjs::Result<bool> {
            let mut scheduler = state.try_borrow_mut().map_err(|_| {
                Exception::throw_internal(&ctx, "Scheduler state is already borrowed")
            })?;
            Ok(scheduler.clear_timer(id))
        }
    })?;
    ctx.globals().set("__sn_clear_timer", clear_timer)?;

    let request_frame = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>, callback: Function<'js>| -> rquickjs::Result<u32> {
            let mut scheduler = state.try_borrow_mut().map_err(|_| {
                Exception::throw_internal(&ctx, "Scheduler state is already borrowed")
            })?;
            scheduler.request_frame(&ctx, callback)
        }
    })?;
    ctx.globals()
        .set("__sn_request_animation_frame", request_frame)?;

    let cancel_frame = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>, id: u32| -> rquickjs::Result<bool> {
            let mut scheduler = state.try_borrow_mut().map_err(|_| {
                Exception::throw_internal(&ctx, "Scheduler state is already borrowed")
            })?;
            Ok(scheduler.cancel_frame(id))
        }
    })?;
    ctx.globals()
        .set("__sn_cancel_animation_frame", cancel_frame)?;

    let register_completion = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>, callback_id: u64, callback: Function<'js>| {
            let mut scheduler = state.try_borrow_mut().map_err(|_| {
                Exception::throw_internal(&ctx, "Scheduler state is already borrowed")
            })?;
            scheduler.register_completion(&ctx, callback_id, callback)
        }
    })?;
    ctx.globals()
        .set("__sn_register_completion_callback", register_completion)?;

    let unregister_completion = Function::new(ctx.clone(), {
        let state = state.clone();
        move |ctx: Ctx<'js>, callback_id: u64| -> rquickjs::Result<bool> {
            let mut scheduler = state.try_borrow_mut().map_err(|_| {
                Exception::throw_internal(&ctx, "Scheduler state is already borrowed")
            })?;
            Ok(scheduler.take_completion(callback_id).is_some())
        }
    })?;
    ctx.globals()
        .set("__sn_unregister_completion_callback", unregister_completion)?;

    let performance = Object::new(ctx.clone())?;
    let now = Function::new(ctx.clone(), move || -> f64 {
        clock.now().as_secs_f64() * 1000.0
    })?;
    performance.set("now", now)?;
    ctx.globals().set("performance", performance)?;

    ctx.eval::<(), _>(
        r#"
        (() => {
          const setTimer = globalThis.__sn_set_timer;
          const clearTimer = globalThis.__sn_clear_timer;

          globalThis.setTimeout = (callback, delay = 0, ...args) => {
            if (typeof callback !== "function") {
              throw new TypeError("setTimeout callback must be a function");
            }
            return setTimer(() => callback(...args), Number(delay), false);
          };
          globalThis.setInterval = (callback, delay = 0, ...args) => {
            if (typeof callback !== "function") {
              throw new TypeError("setInterval callback must be a function");
            }
            return setTimer(() => callback(...args), Number(delay), true);
          };
          globalThis.clearTimeout = id => clearTimer(Number(id) >>> 0);
          globalThis.clearInterval = id => clearTimer(Number(id) >>> 0);

          globalThis.queueMicrotask = callback => {
            if (typeof callback !== "function") {
              throw new TypeError("queueMicrotask callback must be a function");
            }
            // Do not return the callback result into the Promise chain:
            // queueMicrotask does not assimilate a returned thenable.
            Promise.resolve().then(() => { callback(); });
          };

          globalThis.requestAnimationFrame = callback => {
            if (typeof callback !== "function") {
              throw new TypeError("requestAnimationFrame callback must be a function");
            }
            return globalThis.__sn_request_animation_frame(callback);
          };
          globalThis.cancelAnimationFrame = id =>
            globalThis.__sn_cancel_animation_frame(Number(id) >>> 0);
        })();
        "#,
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{ManualClock, RuntimeClock};
    use rquickjs::{Context, Runtime};

    fn run_jobs(runtime: &Runtime, limit: usize) {
        for _ in 0..limit {
            if !runtime.is_job_pending() {
                break;
            }
            match runtime.execute_pending_job() {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => {
                    error.0.with(|ctx| {
                        if ctx.has_exception() {
                            let _ = ctx.catch();
                        }
                    });
                }
            }
        }
    }

    #[test]
    fn timer_delay_normalization_matches_numeric_clamping() {
        assert_eq!(normalized_delay(f64::NAN), Duration::ZERO);
        assert_eq!(normalized_delay(f64::NEG_INFINITY), Duration::ZERO);
        assert_eq!(normalized_delay(-5.0), Duration::ZERO);
        assert_eq!(normalized_delay(1.9), Duration::from_millis(1));
        assert_eq!(
            normalized_delay(f64::INFINITY),
            Duration::from_millis(MAX_TIMER_DELAY_MS as u64)
        );
        assert_eq!(
            normalized_delay(f64::MAX),
            Duration::from_millis(MAX_TIMER_DELAY_MS as u64)
        );
    }

    #[test]
    fn repeated_timer_cancellation_keeps_deadline_queue_bounded() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let clock: Arc<dyn RuntimeClock> = Arc::new(ManualClock::default());
        let frame_requested = Arc::new(AtomicBool::new(false));
        let state = Rc::new(RefCell::new(SchedulerState::new(frame_requested)));

        context.with(|ctx| {
            install_bindings(ctx.clone(), state.clone(), clock).unwrap();
            let callback = Function::new(ctx.clone(), || {}).unwrap();
            for _ in 0..(MAX_TIMER_QUEUE_ENTRIES * 2) {
                let id = state
                    .borrow_mut()
                    .set_timer(&ctx, callback.clone(), 60_000.0, false, Duration::ZERO)
                    .unwrap();
                assert!(state.borrow_mut().clear_timer(id));
            }
            let scheduler = state.borrow();
            assert_eq!(scheduler.pending_counts().0, 0);
            assert!(scheduler.deadlines.len() < MAX_TIMER_QUEUE_ENTRIES);
        });
    }

    #[test]
    fn promise_and_queue_microtask_share_the_quickjs_job_queue_in_registration_order() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let clock: Arc<dyn RuntimeClock> = Arc::new(ManualClock::default());
        let frame_requested = Arc::new(AtomicBool::new(false));
        let state = Rc::new(RefCell::new(SchedulerState::new(frame_requested)));

        context.with(|ctx| {
            install_bindings(ctx.clone(), state.clone(), clock).unwrap();
            ctx.eval::<(), _>(
                "globalThis.order = []; order.push('sync'); Promise.resolve().then(() => order.push('promise')); queueMicrotask(() => order.push('microtask')); order.push('end');",
            )
            .unwrap();
            run_jobs(&runtime, 16);
            let order: String = ctx.eval("order.join(',')").unwrap();
            assert_eq!(order, "sync,end,promise,microtask");
        });
    }

    #[test]
    fn cancelled_timeout_never_fires_and_disposal_releases_scheduled_roots() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let manual = Arc::new(ManualClock::default());
        let clock: Arc<dyn RuntimeClock> = manual.clone();
        let frame_requested = Arc::new(AtomicBool::new(false));
        let state = Rc::new(RefCell::new(SchedulerState::new(frame_requested)));

        context.with(|ctx| {
            install_bindings(ctx.clone(), state.clone(), clock).unwrap();
            ctx.eval::<(), _>(
                "globalThis.hits = 0; globalThis.timer = setTimeout(() => hits++, 0); clearTimeout(timer);",
            )
            .unwrap();
            manual.advance(Duration::from_millis(2));
            assert!(state.borrow_mut().take_due_timer(manual.now()).is_none());
            assert_eq!(state.borrow().pending_counts(), (0, 0, 0));

            state.borrow_mut().dispose();
            assert_eq!(state.borrow().pending_counts(), (0, 0, 0));
        });
    }

    #[test]
    fn animation_frames_are_coalesced_ordered_and_cancellable() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let frame_requested = Arc::new(AtomicBool::new(false));
        let state = Rc::new(RefCell::new(SchedulerState::new(frame_requested.clone())));
        let clock: Arc<dyn RuntimeClock> = Arc::new(ManualClock::default());

        context.with(|ctx| {
            install_bindings(ctx.clone(), state.clone(), clock).unwrap();
            ctx.eval::<(), _>(
                "globalThis.frames = []; let second; requestAnimationFrame(t => { frames.push('first:' + t); cancelAnimationFrame(second); requestAnimationFrame(next => frames.push('next:' + next)); }); second = requestAnimationFrame(() => frames.push('cancelled')); requestAnimationFrame(t => frames.push('last:' + t));",
            )
            .unwrap();

            assert!(frame_requested.load(Ordering::Acquire));
            let cutoff = state.borrow_mut().begin_frame().unwrap();
            let mut delivered = 0;
            while let Some((_, callback)) = state.borrow_mut().take_next_frame_callback(cutoff) {
                callback
                    .restore(&ctx)
                    .unwrap()
                    .call::<_, ()>((16.5f64,))
                    .unwrap();
                delivered += 1;
            }
            assert_eq!(delivered, 2);
            assert!(frame_requested.load(Ordering::Acquire));
            let frames: String = ctx.eval("frames.join(',')").unwrap();
            assert_eq!(frames, "first:16.5,last:16.5");

            let next_cutoff = state.borrow_mut().begin_frame().unwrap();
            let (_, callback) = state.borrow_mut().take_next_frame_callback(next_cutoff).unwrap();
            callback.restore(&ctx).unwrap().call::<_, ()>((33.0f64,)).unwrap();
            assert!(!frame_requested.load(Ordering::Acquire));
            let frames: String = ctx.eval("frames.join(',')").unwrap();
            assert_eq!(frames, "first:16.5,last:16.5,next:33");
        });
    }

    #[test]
    fn repeating_timer_uses_fixed_delay_and_skips_missed_intervals() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let manual = Arc::new(ManualClock::default());
        let clock: Arc<dyn RuntimeClock> = manual.clone();
        let frame_requested = Arc::new(AtomicBool::new(false));
        let state = Rc::new(RefCell::new(SchedulerState::new(frame_requested)));

        context.with(|ctx| {
            install_bindings(ctx.clone(), state.clone(), clock).unwrap();
            ctx.eval::<(), _>("globalThis.intervals = 0; setInterval(() => intervals++, 2);")
                .unwrap();

            manual.advance(Duration::from_millis(20));
            let (_, callback) = state.borrow_mut().take_due_timer(manual.now()).unwrap();
            callback.restore(&ctx).unwrap().call::<_, ()>(()).unwrap();
            assert!(state.borrow_mut().take_due_timer(manual.now()).is_none());

            manual.advance(Duration::from_millis(2));
            let (_, callback) = state.borrow_mut().take_due_timer(manual.now()).unwrap();
            callback.restore(&ctx).unwrap().call::<_, ()>(()).unwrap();
            let intervals: u32 = ctx.eval("intervals").unwrap();
            assert_eq!(intervals, 2);
        });
    }
}
