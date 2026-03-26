#![allow(
    unused_imports,
    unused_mut,
    dead_code,
    unused_variables,
    unreachable_patterns,
    unused_assignments
)]

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

// ===========================================================================
// Section 0: Introduction — Building an Async Runtime from Scratch
// ===========================================================================
//
// This tutorial takes a bottom-up approach to async Rust, inspired by the
// jacko.io blog series. Instead of using tokio, we'll build a mini async
// runtime piece by piece using only the standard library.
//
// The goal: by implementing Futures, Tasks, and an event loop yourself,
// you'll deeply understand what tokio does for you.
//
// Roadmap:
//   Part 1 — Futures: the Future trait, combining futures, Sleep, event loop
//   Part 2 — Tasks: spawn(), JoinHandle, custom Wakers
//   Part 3 — IO: non-blocking IO concepts, putting it all together
//
// Key insight: an async runtime is fundamentally just a loop that calls
// poll() on futures, sleeps when nothing is ready, and wakes up when
// something becomes ready.

fn intro() {
    println!("\n=== Section 0: Introduction ===\n");

    println!("We're building a mini async runtime from scratch — no tokio!");
    println!("Everything uses only std library types.\n");
    println!("The three pillars we'll implement:");
    println!("  1. Futures — the poll()-based abstraction");
    println!("  2. Tasks  — dynamically spawned, runtime-owned futures");
    println!("  3. IO     — waking futures on external events\n");
    println!("Each section builds on the previous one, culminating in a");
    println!("working runtime with spawn(), sleep(), and JoinHandle.");
}

// ===========================================================================
// Part 1: Futures
// ===========================================================================

// ===========================================================================
// Section 1: The Future Trait
// ===========================================================================
//
// The Future trait is the foundation of async Rust:
//
//   trait Future {
//       type Output;
//       fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
//   }
//
// Key concepts:
//   - poll() returns Poll::Ready(value) when done, Poll::Pending when not
//   - Pin<&mut Self> prevents the future from being moved in memory
//     (important for self-referential futures; we'll keep things simple)
//   - Context contains a Waker — a callback the future uses to signal
//     "I might be ready now, please poll me again"
//
// async fn / async blocks are sugar for types that implement Future.
// The compiler transforms them into state machines where each .await
// point is a state transition.
//
// For our first future, we'll build a simple countdown that takes N polls
// to complete — no time involved, just a counter.

fn future_trait_basics() {
    println!("\n=== Section 1: The Future Trait ===\n");

    // --- Step 1: A minimal custom Future ---
    // CountdownFuture counts down from N, returning Pending each time,
    // then Ready("done!") when it hits zero.

    struct CountdownFuture {
        remaining: u32,
    }

    impl Future for CountdownFuture {
        type Output = String;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.remaining == 0 {
                Poll::Ready("done after countdown!".to_string())
            } else {
                println!("  CountdownFuture: {} remaining", self.remaining);
                self.remaining -= 1;
                // Tell the runtime: "poll me again soon"
                // Without this, a real runtime would never re-poll us
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    // --- Step 2: Manually polling a future ---
    // We can poll any Future by hand using Waker::noop() — a waker that
    // does nothing. This is useful for testing and understanding.
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);

    let mut future = Box::pin(CountdownFuture { remaining: 3 });
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Pending => continue,
            Poll::Ready(value) => {
                println!("  Result: {value}");
                break;
            }
        }
    }

    // --- Step 3: async blocks also implement Future ---
    // This async block becomes a compiler-generated state machine
    let mut async_future = Box::pin(async {
        "hello from async block"
    });

    match async_future.as_mut().poll(&mut cx) {
        Poll::Ready(msg) => println!("  Async block returned: {msg}"),
        Poll::Pending => println!("  (would not happen for this simple block)"),
    }
}

// ===========================================================================
// Section 2: Combining Futures — JoinAll
// ===========================================================================
//
// A single future isn't very useful. Real async code runs many futures
// concurrently. The simplest combinator is JoinAll: poll all futures,
// collecting their results.
//
// This is what tokio::join! and futures::future::join_all do internally.
// We'll build our own to see how it works.
//
// Key design decisions:
//   - Store futures as Pin<Box<dyn Future>> for heterogeneous collections
//   - Track which futures are done vs still pending
//   - Only return Ready when ALL futures have completed

fn join_all_demo() {
    println!("\n=== Section 2: Combining Futures — JoinAll ===\n");

    // --- Our CountdownFuture from Section 1, but returning a value ---
    struct CountdownFuture {
        id: u32,
        remaining: u32,
    }

    impl Future for CountdownFuture {
        type Output = String;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.remaining == 0 {
                Poll::Ready(format!("future {} done", self.id))
            } else {
                self.remaining -= 1;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    // --- Step 1: JoinAll combinator ---
    // Stores N futures and their results. Polls all pending futures each time.
    // Returns Ready only when every future has produced a result.

    struct JoinAll<T> {
        futures: Vec<Pin<Box<dyn Future<Output = T>>>>,
        results: Vec<Option<T>>,
    }

    impl<T> JoinAll<T> {
        fn new(futures: Vec<Pin<Box<dyn Future<Output = T>>>>) -> Self {
            let len = futures.len();
            JoinAll {
                futures,
                results: (0..len).map(|_| None).collect(),
            }
        }
    }

    impl<T> Future for JoinAll<T> {
        type Output = Vec<T>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            // Poll each future that hasn't completed yet
            for i in 0..self.futures.len() {
                if self.results[i].is_some() {
                    continue; // Already done
                }
                // SAFETY: we never move futures out of the Vec
                let future = unsafe { self.as_mut().get_unchecked_mut() };
                match future.futures[i].as_mut().poll(cx) {
                    Poll::Ready(value) => {
                        future.results[i] = Some(value);
                    }
                    Poll::Pending => {}
                }
            }

            // Check if all futures are done
            let this = unsafe { self.get_unchecked_mut() };
            if this.results.iter().all(|r| r.is_some()) {
                let results = this.results.iter_mut().map(|r| r.take().unwrap()).collect();
                Poll::Ready(results)
            } else {
                Poll::Pending
            }
        }
    }

    // --- Step 2: Test it ---
    let futures: Vec<Pin<Box<dyn Future<Output = String>>>> = vec![
        Box::pin(CountdownFuture { id: 1, remaining: 2 }),
        Box::pin(CountdownFuture { id: 2, remaining: 4 }),
        Box::pin(CountdownFuture { id: 3, remaining: 1 }),
    ];

    let mut combined = Box::pin(JoinAll::new(futures));
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);

    loop {
        match combined.as_mut().poll(&mut cx) {
            Poll::Pending => continue,
            Poll::Ready(results) => {
                println!("  All futures complete:");
                for r in &results {
                    println!("    {r}");
                }
                break;
            }
        }
    }
}

// ===========================================================================
// Section 3: Sleep and the Event Loop
// ===========================================================================
//
// So far our futures complete after N polls — they burn CPU constantly.
// Real async runtimes avoid this with an event loop:
//
//   1. Poll all pending futures
//   2. If nothing is ready, find the earliest wake time
//   3. thread::sleep until that time
//   4. Wake the relevant futures, go to step 1
//
// We need two things:
//   - A Sleep future that registers "wake me at time T"
//   - An event loop (block_on) that manages wake times
//
// We'll use thread_local storage for the wake-time registry. This is the
// same approach the jacko.io blog uses — it works because our runtime is
// single-threaded.

// Global registry of wake times: Instant → list of Wakers to fire
thread_local! {
    static WAKE_TIMES: RefCell<BTreeMap<Instant, Vec<Waker>>> =
        const { RefCell::new(BTreeMap::new()) };
}

/// A future that completes after a specified duration.
/// On first poll, it registers its wake time. On subsequent polls,
/// it checks if the deadline has passed.
struct Sleep {
    deadline: Instant,
    registered: bool,
}

impl Sleep {
    fn new(duration: Duration) -> Self {
        Sleep {
            deadline: Instant::now() + duration,
            registered: false,
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if Instant::now() >= self.deadline {
            Poll::Ready(())
        } else {
            if !self.registered {
                // Register wake time so the event loop knows when to wake us
                WAKE_TIMES.with(|wt| {
                    wt.borrow_mut()
                        .entry(self.deadline)
                        .or_default()
                        .push(cx.waker().clone());
                });
                self.registered = true;
            }
            Poll::Pending
        }
    }
}

// --- block_on v1: basic event loop for a single future ---
// This is the simplest possible async runtime:
//   1. Poll the future
//   2. If Pending, find earliest wake time, sleep until then
//   3. Repeat until Ready

fn block_on_v1<F: Future>(mut future: Pin<&mut F>) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => {
                // Find the earliest registered wake time
                let next_wake = WAKE_TIMES.with(|wt| {
                    let times = wt.borrow();
                    times.keys().next().copied()
                });

                if let Some(wake_time) = next_wake {
                    // Sleep until the next wake time
                    let now = Instant::now();
                    if wake_time > now {
                        std::thread::sleep(wake_time - now);
                    }
                    // Remove and fire all wakers at or before now
                    WAKE_TIMES.with(|wt| {
                        let mut times = wt.borrow_mut();
                        // Collect keys to wake
                        let expired: Vec<Instant> = times
                            .range(..=Instant::now())
                            .map(|(k, _)| *k)
                            .collect();
                        for key in expired {
                            if let Some(wakers) = times.remove(&key) {
                                for waker in wakers {
                                    waker.wake();
                                }
                            }
                        }
                    });
                }
            }
        }
    }
}

fn sleep_and_event_loop() {
    println!("\n=== Section 3: Sleep and the Event Loop ===\n");

    // --- Step 1: Sequential sleeps ---
    println!("  Sequential sleeps (should take ~300ms total):");
    let start = Instant::now();

    let mut sequential = Box::pin(async {
        Sleep::new(Duration::from_millis(100)).await;
        println!("    slept 100ms");
        Sleep::new(Duration::from_millis(100)).await;
        println!("    slept another 100ms");
        Sleep::new(Duration::from_millis(100)).await;
        println!("    slept another 100ms");
    });

    block_on_v1(sequential.as_mut());
    println!("  Total: {}ms\n", start.elapsed().as_millis());

    // --- Step 2: Concurrent sleeps with JoinAll ---
    // When we sleep concurrently, total time = max(sleeps), not sum
    println!("  Concurrent sleeps (should take ~200ms, not 600ms):");

    // We need a JoinAll here — let's define a simple helper
    struct JoinAll {
        futures: Vec<Pin<Box<dyn Future<Output = ()>>>>,
        done: Vec<bool>,
    }

    impl JoinAll {
        fn new(futures: Vec<Pin<Box<dyn Future<Output = ()>>>>) -> Self {
            let len = futures.len();
            JoinAll {
                futures,
                done: vec![false; len],
            }
        }
    }

    impl Future for JoinAll {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            let this = unsafe { self.get_unchecked_mut() };
            for i in 0..this.futures.len() {
                if this.done[i] {
                    continue;
                }
                if let Poll::Ready(()) = this.futures[i].as_mut().poll(cx) {
                    this.done[i] = true;
                }
            }
            if this.done.iter().all(|&d| d) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }

    let start = Instant::now();
    let mut concurrent = Box::pin(JoinAll::new(vec![
        Box::pin(async {
            Sleep::new(Duration::from_millis(200)).await;
            println!("    200ms sleep done");
        }),
        Box::pin(async {
            Sleep::new(Duration::from_millis(100)).await;
            println!("    100ms sleep done");
        }),
        Box::pin(async {
            Sleep::new(Duration::from_millis(150)).await;
            println!("    150ms sleep done");
        }),
    ]));

    block_on_v1(concurrent.as_mut());
    println!("  Total: {}ms (≈ max, not sum!)", start.elapsed().as_millis());
}

// ===========================================================================
// Section 4: Cancellation and Recursive Futures
// ===========================================================================
//
// Cancellation in async Rust is simple: stop polling a future and drop it.
// When a future is dropped, all its owned resources are cleaned up via Drop.
// This is a key advantage over callback-based systems where cancellation
// is notoriously difficult.
//
// Recursive async functions present a challenge: an async fn returns an
// opaque Future type whose size must be known at compile time. But a
// recursive async fn would have infinite size (it contains itself).
// Solution: Box::pin() the recursive call, allocating it on the heap.

fn cancellation_and_recursion() {
    println!("\n=== Section 4: Cancellation and Recursive Futures ===\n");

    // --- Step 1: Cancellation via Drop ---
    struct DroppableFuture {
        name: String,
    }

    impl Future for DroppableFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            println!("    {} polled — returning Pending forever", self.name);
            // Never completes — only way to stop it is to drop it
            Poll::Pending
        }
    }

    impl Drop for DroppableFuture {
        fn drop(&mut self) {
            println!("    {} DROPPED — cleanup runs automatically", self.name);
        }
    }

    {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        let mut f = Box::pin(DroppableFuture {
            name: "my-task".to_string(),
        });
        // Poll once — it returns Pending
        let _ = f.as_mut().poll(&mut cx);

        println!("  Dropping the future (cancellation)...");
        // f is dropped here — Drop::drop runs
    }

    // --- Step 2: Recursive async with Box::pin ---
    // This recursive function counts down, awaiting itself for n-1.
    // Without Box::pin, the compiler can't determine the future's size.
    async fn countdown(n: u32) -> u32 {
        if n == 0 {
            println!("    countdown reached 0!");
            0
        } else {
            println!("    countdown({n})...");
            // Box::pin allocates the recursive future on the heap
            Box::pin(countdown(n - 1)).await + 1
        }
    }

    let mut future = Box::pin(countdown(4));
    let result = block_on_v1(future.as_mut());
    println!("  Recursive result: {result}");
}

// ===========================================================================
// Part 2: Tasks
// ===========================================================================

// ===========================================================================
// Section 5: Tasks — Dynamic Dispatch and Spawn
// ===========================================================================
//
// So far, our event loop runs a single future. Real runtimes support
// dynamically spawning new tasks at any time. A "task" is a top-level
// future that:
//   - Is owned and polled by the runtime (not by another future)
//   - Can be spawned dynamically with spawn()
//   - Is type-erased (Pin<Box<dyn Future>>) so the runtime can hold many
//
// To support spawn(), we need:
//   1. A thread-local task queue where spawn() pushes new tasks
//   2. An upgraded event loop (block_on_v2) that drains new tasks each
//      iteration and removes completed ones
//
// This is conceptually similar to what tokio::spawn() does, but
// single-threaded and much simpler.

thread_local! {
    static NEW_TASKS: RefCell<Vec<Pin<Box<dyn Future<Output = ()>>>>> =
        const { RefCell::new(Vec::new()) };
}

/// Spawn a new task. The task will be picked up by the event loop
/// on its next iteration.
fn spawn(future: impl Future<Output = ()> + 'static) {
    NEW_TASKS.with(|tasks| {
        tasks.borrow_mut().push(Box::pin(future));
    });
}

// --- block_on v2: event loop with task management ---
fn block_on_v2<F: Future>(mut future: Pin<&mut F>) -> F::Output {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut tasks: Vec<Pin<Box<dyn Future<Output = ()>>>> = Vec::new();

    loop {
        // Poll the main future
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }

        // Drain newly spawned tasks
        NEW_TASKS.with(|new| {
            tasks.append(&mut new.borrow_mut());
        });

        // Poll all tasks, removing completed ones
        tasks.retain_mut(|task| {
            match task.as_mut().poll(&mut cx) {
                Poll::Ready(()) => false, // Remove completed task
                Poll::Pending => true,    // Keep pending task
            }
        });

        // Sleep until next wake time
        let next_wake = WAKE_TIMES.with(|wt| {
            wt.borrow().keys().next().copied()
        });

        if let Some(wake_time) = next_wake {
            let now = Instant::now();
            if wake_time > now {
                std::thread::sleep(wake_time - now);
            }
            WAKE_TIMES.with(|wt| {
                let mut times = wt.borrow_mut();
                let expired: Vec<Instant> = times
                    .range(..=Instant::now())
                    .map(|(k, _)| *k)
                    .collect();
                for key in expired {
                    if let Some(wakers) = times.remove(&key) {
                        for waker in wakers {
                            waker.wake();
                        }
                    }
                }
            });
        }
    }
}

fn tasks_and_spawn() {
    println!("\n=== Section 5: Tasks — Spawn and the Task Queue ===\n");

    let start = Instant::now();

    let mut main_future = Box::pin(async {
        // Spawn three concurrent tasks
        spawn(async {
            Sleep::new(Duration::from_millis(100)).await;
            println!("    task A done (100ms)");
        });

        spawn(async {
            Sleep::new(Duration::from_millis(50)).await;
            println!("    task B done (50ms)");
            // Tasks can spawn more tasks!
            spawn(async {
                Sleep::new(Duration::from_millis(50)).await;
                println!("    task C done (spawned by B, 50ms after B)");
            });
        });

        spawn(async {
            Sleep::new(Duration::from_millis(150)).await;
            println!("    task D done (150ms)");
        });

        // Main future also sleeps — everything runs concurrently
        Sleep::new(Duration::from_millis(200)).await;
        println!("    main future done (200ms)");
    });

    block_on_v2(main_future.as_mut());
    println!("  Total: {}ms", start.elapsed().as_millis());
    println!("  Notice: tasks interleave based on their sleep times!");
}

// ===========================================================================
// Section 6: JoinHandle — Getting Results Back
// ===========================================================================
//
// spawn() runs a task but discards its result. Often we need the result.
// A JoinHandle is a future that resolves to the spawned task's output.
//
// Implementation:
//   - Shared state (Arc<Mutex<JoinState<T>>>) between the task and handle
//   - The task writes its result into the shared state when done
//   - The JoinHandle polls the shared state, returning Pending until ready
//   - When the handle is polled and finds a result, it returns Ready
//
// JoinState transitions:
//   Unawaited → Pending(Waker) → Ready(T)
//               (handle stores     (task writes
//                its waker)         its result)

enum JoinState<T> {
    /// Task spawned, handle not yet awaited
    Unawaited,
    /// Handle has been polled, waiting for result
    Waiting(Waker),
    /// Task completed, result available
    Ready(T),
    /// Result has been taken
    Taken,
}

struct JoinHandle<T> {
    state: Arc<Mutex<JoinState<T>>>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.state.lock().unwrap();
        match std::mem::replace(&mut *state, JoinState::Taken) {
            JoinState::Ready(value) => Poll::Ready(value),
            JoinState::Unawaited | JoinState::Waiting(_) => {
                // Store our waker so the task can wake us when done
                *state = JoinState::Waiting(cx.waker().clone());
                Poll::Pending
            }
            JoinState::Taken => panic!("JoinHandle polled after completion"),
        }
    }
}

/// Spawn a task and return a JoinHandle to get its result.
fn spawn_with_handle<T: Send + 'static>(
    future: impl Future<Output = T> + 'static,
) -> JoinHandle<T> {
    let state = Arc::new(Mutex::new(JoinState::Unawaited));
    let task_state = state.clone();

    // Wrap the future: when it completes, store result and wake the handle
    spawn(async move {
        let result = future.await;
        let mut state = task_state.lock().unwrap();
        let old = std::mem::replace(&mut *state, JoinState::Ready(result));
        if let JoinState::Waiting(waker) = old {
            waker.wake();
        }
    });

    JoinHandle { state }
}

fn join_handle_demo() {
    println!("\n=== Section 6: JoinHandle — Getting Results Back ===\n");

    let mut main_future = Box::pin(async {
        // Spawn tasks that return values
        let handle_a = spawn_with_handle(async {
            Sleep::new(Duration::from_millis(100)).await;
            42
        });

        let handle_b = spawn_with_handle(async {
            Sleep::new(Duration::from_millis(50)).await;
            "hello from task B"
        });

        // Spawn a task that depends on another task's result
        let handle_c = spawn_with_handle(async {
            Sleep::new(Duration::from_millis(75)).await;
            vec![1, 2, 3]
        });

        // Await all handles — order doesn't matter, each resolves when ready
        let a = handle_a.await;
        let b = handle_b.await;
        let c = handle_c.await;

        println!("  Task A returned: {a}");
        println!("  Task B returned: {b}");
        println!("  Task C returned: {c:?}");
        println!("  Sum of C: {}", c.iter().sum::<i32>());
    });

    block_on_v2(main_future.as_mut());
}

// ===========================================================================
// Section 7: Custom Waker — Task-to-Task Signaling
// ===========================================================================
//
// So far our event loop uses Waker::noop() — it re-polls everything on
// every iteration. Real runtimes use custom Wakers to only re-poll tasks
// that have been explicitly woken.
//
// The Wake trait:
//   trait Wake {
//       fn wake(self: Arc<Self>);
//       fn wake_by_ref(self: &Arc<Self>) { ... }  // default
//   }
//
// A Waker is created from an Arc<impl Wake>. When wake() is called,
// it signals the runtime that this specific task should be re-polled.
//
// Our upgraded event loop (block_on_v3) will:
//   - Assign each task a unique ID
//   - Create a TaskWaker per task (with an AtomicBool "woken" flag)
//   - Only re-poll tasks whose waker has been triggered

struct TaskWaker {
    woken: AtomicBool,
}

impl TaskWaker {
    fn new() -> Arc<Self> {
        Arc::new(TaskWaker {
            woken: AtomicBool::new(true), // Start woken so first poll happens
        })
    }

    fn is_woken(&self) -> bool {
        self.woken.swap(false, Ordering::SeqCst)
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.woken.store(true, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.woken.store(true, Ordering::SeqCst);
    }
}

// --- block_on v3: efficient polling with real wakers ---
fn block_on_v3<F: Future>(mut future: Pin<&mut F>) -> F::Output {
    // Each task gets its own waker
    struct TaskEntry {
        future: Pin<Box<dyn Future<Output = ()>>>,
        waker: Arc<TaskWaker>,
    }

    let main_waker = TaskWaker::new();
    let main_waker_ref: Waker = main_waker.clone().into();
    let mut cx = Context::from_waker(&main_waker_ref);
    let mut tasks: Vec<TaskEntry> = Vec::new();
    let mut poll_count: u64 = 0;

    loop {
        // Poll main future only if woken
        if main_waker.is_woken() {
            poll_count += 1;
            if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                println!("  [v3 stats] total poll() calls: {poll_count}");
                return value;
            }
        }

        // Drain newly spawned tasks
        NEW_TASKS.with(|new| {
            for fut in new.borrow_mut().drain(..) {
                tasks.push(TaskEntry {
                    future: fut,
                    waker: TaskWaker::new(),
                });
            }
        });

        // Poll only woken tasks
        tasks.retain_mut(|entry| {
            if entry.waker.is_woken() {
                poll_count += 1;
                let waker: Waker = entry.waker.clone().into();
                let mut cx = Context::from_waker(&waker);
                match entry.future.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => false,
                    Poll::Pending => true,
                }
            } else {
                true // Not woken, keep but don't poll
            }
        });

        // Sleep until next wake time
        let next_wake = WAKE_TIMES.with(|wt| {
            wt.borrow().keys().next().copied()
        });

        if let Some(wake_time) = next_wake {
            let now = Instant::now();
            if wake_time > now {
                std::thread::sleep(wake_time - now);
            }
            WAKE_TIMES.with(|wt| {
                let mut times = wt.borrow_mut();
                let expired: Vec<Instant> = times
                    .range(..=Instant::now())
                    .map(|(k, _)| *k)
                    .collect();
                for key in expired {
                    if let Some(wakers) = times.remove(&key) {
                        for waker in wakers {
                            waker.wake();
                        }
                    }
                }
            });
        }
    }
}

fn custom_waker_demo() {
    println!("\n=== Section 7: Custom Waker — Efficient Polling ===\n");

    let start = Instant::now();

    let mut main_future = Box::pin(async {
        // Spawn tasks that communicate via JoinHandle
        let producer = spawn_with_handle(async {
            Sleep::new(Duration::from_millis(100)).await;
            println!("    producer: computed value");
            42
        });

        let consumer_handle = spawn_with_handle(async move {
            let value = producer.await;
            println!("    consumer: got {value} from producer");
            value * 2
        });

        // Spawn an independent task
        spawn(async {
            Sleep::new(Duration::from_millis(50)).await;
            println!("    independent task done");
        });

        let result = consumer_handle.await;
        println!("    main: final result = {result}");
    });

    block_on_v3(main_future.as_mut());
    println!("  Total: {}ms", start.elapsed().as_millis());
    println!("  With real wakers, tasks are only polled when signaled!");
}

// ===========================================================================
// Part 3: IO
// ===========================================================================

// ===========================================================================
// Section 8: Non-Blocking IO Concepts
// ===========================================================================
//
// The final piece of an async runtime is IO. In synchronous code,
// operations like read() block the thread until data arrives. In async
// code, we need non-blocking IO:
//
//   1. Try the operation → if data is available, great
//   2. If not (WouldBlock), return Pending and register a Waker
//   3. When the OS signals readiness, the Waker fires and we re-poll
//
// Real implementations use OS-specific APIs:
//   - Linux: epoll
//   - macOS: kqueue
//   - Windows: IOCP
//
// The mio crate provides a cross-platform abstraction, and tokio builds
// on top of mio. We won't implement real IO here (that requires libc
// bindings), but we'll simulate the pattern.
//
// The key pattern for async IO:
//
//   ```
//   // Real non-blocking TCP (conceptual — requires tokio)
//   // listener.set_nonblocking(true);
//   // match listener.accept() {
//   //     Ok(stream) => Poll::Ready(stream),
//   //     Err(e) if e.kind() == ErrorKind::WouldBlock => {
//   //         // Register waker with epoll/kqueue/IOCP
//   //         cx.waker().clone() → stored for OS callback
//   //         Poll::Pending
//   //     }
//   //     Err(e) => Poll::Ready(Err(e)),
//   // }
//   ```
//
// Our simulation: a background std::thread produces a value after a delay,
// then wakes the future via its stored Waker. This is exactly how real IO
// works — just with OS events instead of a thread::sleep.

/// Simulated async IO: a background thread "produces" a value after a delay,
/// then wakes the future.
struct SimulatedIo<T: Send + 'static> {
    state: Arc<Mutex<IoState<T>>>,
    started: bool,
}

enum IoState<T> {
    Pending(Option<Waker>),
    Ready(T),
}

impl<T: Send + 'static> SimulatedIo<T> {
    fn new<F>(delay: Duration, produce: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        SimulatedIo {
            state: Arc::new(Mutex::new(IoState::Pending(None))),
            started: false,
        }
    }

    fn start<F>(&mut self, delay: Duration, produce: F)
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let state = self.state.clone();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            let value = produce();
            let mut s = state.lock().unwrap();
            let old = std::mem::replace(&mut *s, IoState::Ready(value));
            if let IoState::Pending(Some(waker)) = old {
                waker.wake();
            }
        });
        self.started = true;
    }
}

impl<T: Send + 'static> Future for SimulatedIo<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut state = self.state.lock().unwrap();
        match std::mem::replace(&mut *state, IoState::Pending(None)) {
            IoState::Ready(value) => Poll::Ready(value),
            IoState::Pending(_) => {
                *state = IoState::Pending(Some(cx.waker().clone()));
                Poll::Pending
            }
        }
    }
}

/// Helper to create and start a simulated IO future.
fn simulated_io<T: Send + 'static>(
    delay: Duration,
    produce: impl FnOnce() -> T + Send + 'static,
) -> SimulatedIo<T> {
    let mut io = SimulatedIo {
        state: Arc::new(Mutex::new(IoState::Pending(None))),
        started: false,
    };
    io.start(delay, produce);
    io
}

fn nonblocking_io_concept() {
    println!("\n=== Section 8: Non-Blocking IO Concepts ===\n");

    println!("  Real async IO pattern:");
    println!("    1. Try operation → if WouldBlock, store Waker");
    println!("    2. OS signals readiness → Waker fires");
    println!("    3. Re-poll → operation succeeds\n");
    println!("  We simulate this with background threads.\n");

    let start = Instant::now();

    let mut main_future = Box::pin(async {
        // Simulate two IO operations happening concurrently
        let db_query = spawn_with_handle(async {
            let result = simulated_io(Duration::from_millis(80), || {
                "db: [{id: 1, name: 'Alice'}]".to_string()
            }).await;
            println!("    DB query returned: {result}");
            result
        });

        let api_call = spawn_with_handle(async {
            let result = simulated_io(Duration::from_millis(120), || {
                "api: {status: 200, body: 'OK'}".to_string()
            }).await;
            println!("    API call returned: {result}");
            result
        });

        let db = db_query.await;
        let api = api_call.await;
        println!("    Both IO operations complete!");
        (db, api)
    });

    let (db, api) = block_on_v3(main_future.as_mut());
    println!("  Total: {}ms (concurrent IO!)", start.elapsed().as_millis());
}

// ===========================================================================
// Section 9: Putting It All Together
// ===========================================================================
//
// Let's recap what we built:
//
// Our mini runtime supports:
//   ✓ Custom futures with poll()
//   ✓ Sleep with efficient event loop (no busy-waiting)
//   ✓ spawn() for dynamic task creation
//   ✓ JoinHandle for getting results from tasks
//   ✓ Custom Wakers for efficient re-polling
//   ✓ Simulated IO with background wake
//
// What tokio adds on top:
//   - Multi-threaded work-stealing scheduler
//   - Timer wheel (O(1) timer operations vs our O(log n) BTreeMap)
//   - Real IO via mio (epoll/kqueue/IOCP)
//   - Channels (mpsc, oneshot, watch, broadcast)
//   - Synchronization (Mutex, Semaphore, Notify, Barrier)
//   - Utilities (timeout, interval, select!, JoinSet)
//
// But the core architecture is the same: a loop that polls futures,
// sleeps when idle, and wakes on events.

fn mini_runtime_summary() {
    println!("\n=== Section 9: Putting It All Together ===\n");

    let start = Instant::now();

    let mut main_future = Box::pin(async {
        println!("  Final demo: combining everything we built\n");

        // 1. Spawn compute tasks with JoinHandles
        let compute = spawn_with_handle(async {
            Sleep::new(Duration::from_millis(50)).await;
            let result = (1..=10).sum::<i32>();
            println!("    [compute] sum(1..=10) = {result}");
            result
        });

        // 2. Spawn simulated IO
        let io = spawn_with_handle(async {
            let data = simulated_io(Duration::from_millis(80), || {
                vec![10, 20, 30]
            }).await;
            println!("    [io] received data: {data:?}");
            data
        });

        // 3. Spawn a task that depends on both
        let aggregator = spawn_with_handle(async move {
            let sum = compute.await;
            let data = io.await;
            let io_sum: i32 = data.iter().sum();
            let total = sum + io_sum;
            println!("    [aggregator] compute({sum}) + io({io_sum}) = {total}");
            total
        });

        // 4. Spawn independent background work
        spawn(async {
            Sleep::new(Duration::from_millis(30)).await;
            println!("    [background] heartbeat ♡");
        });

        spawn(async {
            Sleep::new(Duration::from_millis(60)).await;
            println!("    [background] heartbeat ♡♡");
        });

        let final_result = aggregator.await;
        println!("\n  Final result: {final_result}");
    });

    block_on_v3(main_future.as_mut());
    println!("  Total: {}ms\n", start.elapsed().as_millis());

    println!("  What we built (std only)      vs   What tokio provides");
    println!("  ──────────────────────────────────────────────────────────");
    println!("  block_on (single-threaded)     →   Multi-thread work-stealing");
    println!("  Sleep + BTreeMap               →   Timer wheel (O(1))");
    println!("  spawn() + task queue           →   tokio::spawn() + scheduler");
    println!("  JoinHandle (Arc<Mutex>)        →   JoinHandle + JoinSet");
    println!("  TaskWaker (AtomicBool)         →   Optimized waker vtable");
    println!("  SimulatedIo (thread::spawn)    →   mio (epoll/kqueue/IOCP)");
    println!("  (nothing)                      →   Channels, Semaphore, select!");
}

// ===========================================================================
// pub fn run()
// ===========================================================================

pub fn run() {
    intro();
    future_trait_basics();
    join_all_demo();
    sleep_and_event_loop();
    cancellation_and_recursion();
    tasks_and_spawn();
    join_handle_demo();
    custom_waker_demo();
    nonblocking_io_concept();
    mini_runtime_summary();
}
