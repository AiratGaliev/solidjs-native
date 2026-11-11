# Performance Baseline and Benchmark Protocol

- Status: mandatory measurement policy.
- Current state: a reproducible benchmark suite and numeric performance budgets are not yet established.
- Roadmap alignment: performance budgets and regression baselines belong to runtime hardening (M11) and to the relevant platform milestones.

This document defines how performance claims and future baselines must be established. It intentionally does not invent target numbers or claim that benchmark infrastructure already exists.

## 1. Measurement principles

- Do not optimize based only on intuition or a synthetic microbenchmark.
- Capture a baseline before changing a performance-sensitive path, then run the same workload after the change.
- Separate debug and optimized builds. Use release-equivalent settings for performance claims.
- Record all material environment details and exact commit SHA.
- Repeat measurements and report distributions, not a single favorable sample.
- Keep correctness and lifecycle checks enabled; a faster path that drops events, leaks resources or violates ordering is not an optimization.
- Do not state that a platform meets a performance target until the target and measurement procedure are documented and the result is reproducible.

## 2. Performance dimensions

As the corresponding subsystems become implemented, baseline at least the applicable dimensions:

| Area | Candidate metrics |
|---|---|
| Runtime startup | Time from process/runtime start to application-ready and first committed UI. |
| First render | Time to first frame/paint and time to stable layout. |
| Interaction | Input-to-state-update and input-to-visible-frame latency. |
| Host protocol | Batch encode/decode time, bytes per batch, throughput, commit latency and queue delay. |
| UI tree | Insert/move/remove/update cost at representative tree sizes and depths. |
| Layout/style | Layout duration, style resolution cost, dirty propagation and number of remeasured nodes. |
| Scheduling | Queue wait, job execution time, frame budget use, starvation and missed-frame behavior. |
| Memory | Baseline/peak resident memory, retained nodes, JS heap, native resources and growth after repeated mount/dispose cycles. |
| Idle overhead | Idle CPU, wakeups and background timer/scheduler activity. |
| Distribution | Binary, installer, SDK package and application bundle sizes where applicable. |

Do not measure or publish a metric for a subsystem that does not exist yet as though it were current product behavior.

## 3. Benchmark design

Every benchmark intended to support an engineering decision must document:
1. **Question/hypothesis** — what bottleneck or regression is being tested.
2. **Workload** — realistic app scenario, tree shape/size, event frequency, resource count and data set.
3. **Environment** — OS/version, CPU, memory, architecture, device/power mode, toolchain, runtime and build profile.
4. **Procedure** — setup, warm-up, number of repetitions, sampling interval and cleanup.
5. **Metrics** — unit, aggregation and whether lower/higher is better.
6. **Comparison** — baseline SHA versus candidate SHA, with configuration kept constant.
7. **Results** — raw data or artifact link, median and tail percentiles (p90/p95/p99 where sample size supports them), variance and outliers.
8. **Interpretation** — effect size, confidence/limitations and whether the result changes a product decision.

Use the same machine and workload for before/after comparisons whenever possible. If the platform, build profile or environment changes, treat it as a separate baseline rather than a direct comparison.

## 4. Regression budgets

M11 should introduce explicit, per-platform budgets only after representative measurements exist. Each budget must specify:
- metric and unit;
- workload and target platform/device class;
- threshold and allowed variance;
- measurement tool and procedure;
- owner and review cadence;
- CI enforcement or documented manual gate;
- conditions for recalibration.

A budget must not be copied from a reference project without verifying its relevance to this framework. Avoid a universal threshold that hides platform-specific behavior.

## 5. Runtime and platform workloads

Prioritize representative end-to-end workloads:
- mount, update, move and dispose of a realistic component tree;
- signal-driven updates crossing the Universal adapter and typed host protocol;
- a frame containing layout invalidation and style updates;
- repeated input and focus transitions;
- text measurement, editing and IME composition when implemented;
- image/resource load, cancellation, cache reuse and disposal;
- multiple Surfaces/windows where supported;
- app startup, idle, foreground/background and shutdown lifecycle.

Measure both steady-state behavior and pathological but valid inputs. Always pair performance measurements with correctness and resource-leak checks.

## 6. Reporting

A performance report must include exact SHA, build profile, toolchain, target hardware/OS, workload, repetitions, raw results, percentile summaries, comparison baseline and known limitations. Link it from the Linear issue and relevant milestone.

A local profiler screenshot, one benchmark run or a result from a different machine is diagnostic material, not a confirmed performance claim.
