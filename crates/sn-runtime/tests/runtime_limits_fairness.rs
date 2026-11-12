use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use sn_runtime::{EngineSession, EngineSessionConfig, MonotonicClock, RuntimeClock, SessionError};

fn start_config() -> EngineSessionConfig {
    EngineSessionConfig {
        clock: Arc::new(MonotonicClock::default()) as Arc<dyn RuntimeClock>,
        ..EngineSessionConfig::default()
    }
}

#[test]
fn execution_deadline_interrupts_an_unbounded_script_and_keeps_session_usable() {
    let mut config = start_config();
    config.max_execution_time = Duration::from_millis(30);
    let session = EngineSession::start(config).expect("runtime should start");
    let handle = session.handle();

    let started = Instant::now();
    let result = handle
        .evaluate("while (true) {}")
        .expect("script should be accepted")
        .wait();

    assert!(
        matches!(result, Err(SessionError::Runtime(_))),
        "an execution deadline should interrupt the script, got {result:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "an unbounded script exceeded the configured execution deadline"
    );

    assert_eq!(
        handle
            .evaluate_text("String(6 * 7)")
            .expect("follow-up task should be accepted")
            .wait()
            .expect("runtime should remain usable after interruption"),
        "42"
    );

    session.shutdown().expect("runtime should shut down");
}

#[test]
fn heap_limit_rejects_excessive_allocation_without_poisoning_the_session() {
    let mut config = start_config();
    config.heap_limit_bytes = 16 * 1024 * 1024;
    let session = EngineSession::start(config).expect("runtime should start");
    let handle = session.handle();

    let result = handle
        .evaluate("globalThis.heapPressure = new Array(4_000_000).fill(0);")
        .expect("allocation task should be accepted")
        .wait();

    assert!(
        matches!(result, Err(SessionError::Runtime(_))),
        "allocation beyond the configured heap limit should fail, got {result:?}"
    );
    assert_eq!(
        handle
            .evaluate_text("String(6 * 7)")
            .expect("follow-up task should be accepted")
            .wait()
            .expect("runtime should remain usable after allocation failure"),
        "42"
    );

    session.shutdown().expect("runtime should shut down");
}

#[test]
fn stack_limit_stops_deep_recursion_without_poisoning_the_session() {
    let mut config = start_config();
    config.stack_limit_bytes = 64 * 1024;
    let session = EngineSession::start(config).expect("runtime should start");
    let handle = session.handle();

    let result = handle
        .evaluate("function recurse() { return recurse(); } recurse();")
        .expect("recursive task should be accepted")
        .wait();

    assert!(
        matches!(result, Err(SessionError::Runtime(_))),
        "recursion beyond the configured stack limit should fail, got {result:?}"
    );
    assert_eq!(
        handle
            .evaluate_text("String(6 * 7)")
            .expect("follow-up task should be accepted")
            .wait()
            .expect("runtime should remain usable after stack exhaustion"),
        "42"
    );

    session.shutdown().expect("runtime should shut down");
}

#[test]
fn command_responses_remain_bounded_while_promise_jobs_keep_rescheduling() {
    let mut config = start_config();
    config.max_jobs_per_slice = 4;
    config.max_job_slice_time = Duration::from_millis(2);
    config.max_execution_time = Duration::from_millis(100);
    let session = EngineSession::start(config).expect("runtime should start");
    let handle = session.handle();

    handle
        .evaluate(
            "globalThis.chainCount = 0; const reschedule = () => { chainCount++; Promise.resolve().then(reschedule); }; Promise.resolve().then(reschedule);",
        )
        .expect("microtask chain should be accepted")
        .wait()
        .expect("initial script should execute");

    // The initial evaluate drains one bounded job slice. Capture that baseline
    // before the command burst so the assertion below proves that Promise jobs
    // continue making progress while commands are being serviced.
    let chain_count_before = handle
        .evaluate_text("String(chainCount)")
        .expect("baseline query should be accepted")
        .wait()
        .expect("baseline query should complete")
        .parse::<u64>()
        .expect("baseline chain count should be numeric");

    let tickets = (0..24)
        .map(|index| {
            handle
                .evaluate_text(format!("String({index})"))
                .expect("command should be accepted")
        })
        .collect::<Vec<_>>();

    for (index, ticket) in tickets.iter().enumerate() {
        assert_eq!(
            ticket
                .wait_timeout(Duration::from_secs(2))
                .expect("command should complete while microtasks remain pending"),
            index.to_string()
        );
    }

    let chain_count_after = handle
        .evaluate_text("String(chainCount)")
        .expect("progress query should be accepted")
        .wait()
        .expect("progress query should complete")
        .parse::<u64>()
        .expect("chain count should be numeric");
    assert!(
        chain_count_after > chain_count_before,
        "Promise jobs should keep making progress between command batches: before={chain_count_before}, after={chain_count_after}"
    );

    session.shutdown().expect("runtime should shut down");
}
