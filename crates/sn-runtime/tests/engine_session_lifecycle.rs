use std::{
    sync::Arc,
    time::Duration,
};

use sn_runtime::{
    EngineSession, EngineSessionConfig, ManualClock, RuntimeClock, SessionError,
    SessionGeneration, SurfaceId,
};

fn config(
    surface_id: SurfaceId,
    session_generation: SessionGeneration,
    clock: Arc<dyn RuntimeClock>,
) -> EngineSessionConfig {
    let mut config = EngineSessionConfig::for_surface(surface_id, session_generation);
    config.clock = clock;
    config
}

fn evaluate_text(handle: &sn_runtime::EngineSessionHandle, source: &str) -> String {
    handle
        .evaluate_text(source)
        .expect("task should be accepted")
        .wait()
        .expect("JavaScript expression should evaluate")
}

#[test]
fn concurrent_surfaces_have_independent_contexts_clocks_and_identity() {
    let first_clock = Arc::new(ManualClock::default());
    let second_clock = Arc::new(ManualClock::default());

    let first = EngineSession::start(config(
        SurfaceId::new(101),
        SessionGeneration::new(1),
        first_clock.clone(),
    ))
    .expect("first Surface runtime should start");
    let second = EngineSession::start(config(
        SurfaceId::new(202),
        SessionGeneration::new(1),
        second_clock.clone(),
    ))
    .expect("second Surface runtime should start");

    let first_handle = first.handle();
    let second_handle = second.handle();

    assert!(!first_handle.is_owner_thread());
    assert!(!second_handle.is_owner_thread());

    first_handle
        .evaluate(
            "globalThis.surfaceMarker = 'first'; globalThis.timerHits = 0; setTimeout(() => timerHits++, 10);",
        )
        .expect("first Surface script should be accepted")
        .wait()
        .expect("first Surface script should execute");
    second_handle
        .evaluate(
            "globalThis.surfaceMarker = 'second'; globalThis.timerHits = 0; setTimeout(() => timerHits++, 10);",
        )
        .expect("second Surface script should be accepted")
        .wait()
        .expect("second Surface script should execute");

    assert_eq!(
        evaluate_text(
            &first_handle,
            "String(__sn_host_config.surfaceId) + ':' + String(__sn_host_config.sessionGeneration)",
        ),
        "101:1"
    );
    assert_eq!(
        evaluate_text(
            &second_handle,
            "String(__sn_host_config.surfaceId) + ':' + String(__sn_host_config.sessionGeneration)",
        ),
        "202:1"
    );
    assert_eq!(evaluate_text(&first_handle, "surfaceMarker"), "first");
    assert_eq!(evaluate_text(&second_handle, "surfaceMarker"), "second");

    first_clock.advance(Duration::from_millis(10));
    first_handle
        .pump()
        .expect("first Surface scheduler should accept a pump")
        .wait()
        .expect("first Surface scheduler should pump");

    assert_eq!(evaluate_text(&first_handle, "String(timerHits)"), "1");
    assert_eq!(evaluate_text(&second_handle, "String(timerHits)"), "0");

    first.shutdown().expect("first Surface should shut down");
    second.shutdown().expect("second Surface should shut down");
}

#[test]
fn a_surface_can_be_recreated_with_a_fresh_runtime_for_each_generation() {
    let surface_id = SurfaceId::new(303);

    for generation in 1..=3 {
        let clock = Arc::new(ManualClock::default());
        let session = EngineSession::start(config(
            surface_id,
            SessionGeneration::new(generation),
            clock,
        ))
        .expect("Surface runtime should start");
        let handle = session.handle();

        assert_eq!(
            evaluate_text(
                &handle,
                "String(__sn_host_config.surfaceId) + ':' + String(__sn_host_config.sessionGeneration)",
            ),
            format!("303:{generation}")
        );
        assert_eq!(
            evaluate_text(&handle, "typeof globalThis.generationMarker"),
            "undefined",
            "a recreated Surface must not inherit JavaScript globals"
        );

        let initial = handle
            .snapshot()
            .expect("fresh runtime should accept a snapshot")
            .wait()
            .expect("fresh runtime snapshot should be available");
        assert_eq!(initial.revision, 0);
        assert_eq!(initial.node_count, 0);
        assert_eq!(initial.callback_count, 0);
        assert!(initial.roots.is_empty());
        assert!(initial.last_commit.is_none());

        handle
            .evaluate(format!(
                "globalThis.generationMarker = {generation}; setTimeout(() => globalThis.timerFired = true, 10); __sn_register_session_cleanup(() => {{ throw new Error('generation-{generation}-cleanup'); }});"
            ))
            .expect("session state should be accepted")
            .wait()
            .expect("session state should be installed");

        session
            .shutdown()
            .expect("Surface runtime should dispose cleanly");

        let diagnostics = handle.take_diagnostics();
        assert!(
            diagnostics.iter().any(|entry| {
                entry.message.contains(&format!("generation-{generation}-cleanup"))
            }),
            "the generation cleanup callback must run during shutdown: {diagnostics:?}"
        );
        assert!(matches!(
            handle.evaluate_text("'late task'"),
            Err(SessionError::Closed)
        ));
    }
}
