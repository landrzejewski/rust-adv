#![allow(
    unused_imports,
    unused_mut,
    dead_code,
    unused_variables,
    unreachable_patterns,
    unused_assignments
)]

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::runtime::{Builder, Runtime};
use tokio::sync::{broadcast, mpsc, oneshot, watch, Mutex, Notify, Semaphore};
use tokio::task::JoinSet;
use tokio::time;

// ===========================================================================
// Section 0: Introduction — From Scratch to Production Runtime
// ===========================================================================
//
// Concurrency vs parallelism:
//   Concurrency = structuring work as independent tasks that *can* interleave.
//   Parallelism = *actually* executing multiple tasks at the same time (multiple cores).
//   Async is primarily about concurrency: one thread can drive thousands of
//   tasks by switching between them at .await points. Parallelism comes when
//   the runtime spreads tasks across a thread pool (tokio's multi_thread mode).
//
// The problem async solves:
//   Thread-per-connection is expensive: each OS thread costs 2-8 MB of stack,
//   context switches are costly, and the OS scheduler degrades under thousands
//   of threads. Async runs many tasks on few threads via cooperative yielding
//   at .await — a single thread can service tens of thousands of connections.
//
// async fn mechanics:
//   Calling an async fn does NOT execute its body. It returns a Future — a
//   compiler-generated state machine where each .await is a yield point
//   (state transition). Execution only happens when an executor polls the
//   future. This is fundamentally different from Go/JS where calling an
//   async function immediately starts execution.
//
// Runtime landscape:
//   Rust's std library defines the Future trait but provides no executor.
//   tokio is the de facto standard runtime. Others: async-std, smol, glommio.
//   This separation means async Rust is runtime-agnostic at the trait level,
//   but in practice most libraries target tokio.
//

// ===========================================================================
// Section 1: The Tokio Runtime and block_on
// ===========================================================================
//
// tokio::runtime::Runtime is the async executor. It owns:
//   - A thread pool (work-stealing scheduler)
//   - A timer driver (tokio::time)
//   - An IO driver (mio-based, epoll/kqueue/IOCP)
//
// mio foundation:
//   tokio's IO driver is built on mio, which wraps OS-specific APIs:
//   epoll (Linux), kqueue (macOS/BSD), IOCP (Windows). The model is
//   token-based: register an IO source with a token → poll for events →
//   look up the ready source by token → perform IO. This is what makes
//   tokio able to handle thousands of connections on a single thread.
//
// #[tokio::main] mechanics:
//   The attribute macro transforms `async fn main()` into a sync `fn main()`
//   that creates a Runtime and calls `block_on(async { ... })`. We don't use
//   it here because our `run()` must be a sync fn (called from main.rs).
//
// Runtime flavors:
//   multi_thread — work-stealing thread pool. Tasks can migrate between
//     worker threads for optimal load balancing. Best for production servers.
//     Default worker count = std::thread::available_parallelism() (= CPU cores).
//   current_thread — all tasks run on the calling thread. Lower overhead,
//     spawned tasks don't need Send. Good for testing and simple tools.
//
// Runtime::new() gives you a multi-threaded runtime with defaults.
// Builder lets you customize: thread count, thread names, etc.
//
// Back-ref: our block_on_v1 was a single-threaded loop that called
// thread::sleep between polls. tokio's runtime uses work-stealing
// across multiple OS threads and wakes via epoll/kqueue instead of sleeping.

fn tokio_runtime_and_block_on() {
    println!("\n=== Section 1: The Tokio Runtime and block_on ===\n");

    // --- Step 1: Runtime::new() + block_on ---
    // Runtime::new() creates a multi-threaded runtime with default settings.
    // block_on() runs a future to completion on the current thread while
    // the runtime's thread pool handles spawned tasks.
    let rt = Runtime::new().unwrap();
    let result = rt.block_on(async {
        println!("  Step 1: Running inside block_on");
        println!("    Thread: {:?}", std::thread::current().id());
        42
    });
    println!("    Result: {result}");

    // --- Step 2: Builder::new_current_thread() ---
    // Single-threaded runtime — all tasks run on the calling thread.
    // Useful for testing or when you don't need parallelism.
    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        println!("\n  Step 2: Current-thread runtime");
        println!("    Thread: {:?}", std::thread::current().id());
        println!("    (Same thread as caller — no thread pool)");
    });

    // --- Step 3: Builder::new_multi_thread() with explicit worker count ---
    // We can control the number of worker threads. Each spawned task might
    // run on a different thread — work-stealing distributes them.
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        println!("\n  Step 3: Multi-thread runtime with 2 workers");
        let mut handles = vec![];
        for i in 0..4 {
            handles.push(tokio::spawn(async move {
                let tid = std::thread::current().id();
                println!("    Task {i} on thread: {tid:?}");
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        println!("    (Tasks may run on different threads — work-stealing!)");
    });
}

// ===========================================================================
// Section 2: The Future Trait and Polling
// ===========================================================================
//
// The Future trait is the foundation of async Rust. Every async fn, async
// block, and .await compiles down to this trait:
//
//   trait Future {
//       type Output;
//       fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
//   }
//
// How it works:
//   1. The executor calls poll() on a future.
//   2. The future does as much work as it can without blocking.
//   3. If complete → return Poll::Ready(value).
//      If not complete → store the Waker from cx.waker(), return Poll::Pending.
//   4. When the awaited event fires (IO ready, timer expired, etc.), the stored
//      Waker is invoked via wake(), which tells the executor to re-poll.
//   5. The executor calls poll() again. Goto step 2.
//
// Key insight: the executor only polls futures that have been woken. This is
// what makes async efficient — no busy-waiting, no polling every future in a
// loop. A server with 10,000 idle connections has 10,000 sleeping futures that
// consume zero CPU until their socket receives data.
//
// Pin<&mut Self>: async fn state machines may contain self-referential fields
// (local variable references that span .await points). Pin prevents the future
// from being moved in memory, keeping those internal references valid.
// In practice, you rarely deal with Pin directly — .await and tokio handle it.
//
// Context<'_> carries the Waker — the only way for a future to arrange its
// own re-polling. Without it, the executor would have no way to know when
// a Pending future should be polled again.
//

/// A custom future that counts down from `remaining` to 0.
/// Each poll decrements the counter. When it hits 0, the future resolves.
/// This is a busy-polling pattern (wakes itself immediately) — real futures
/// only wake when actual progress is possible.
struct Countdown {
    remaining: u32,
}

impl Future for Countdown {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.remaining == 0 {
            Poll::Ready("countdown complete!".to_string())
        } else {
            self.remaining -= 1;
            // Wake immediately so the executor re-polls us right away.
            // A real future would only wake when actual progress can be
            // made (e.g., data arrived on a socket, timer expired).
            cx.waker().wake_by_ref();
            Poll::Pending
        }
}
}

fn the_future_trait_and_polling() {
    println!("\n=== Section 2: The Future Trait and Polling ===\n");

    let rt = Runtime::new().unwrap();

    // --- Step 1: Use our custom Countdown future ---
    // The runtime's executor calls poll() repeatedly until Ready.
    rt.block_on(async {
        println!("  Step 1: Custom Countdown future");
        let result = Countdown { remaining: 5 }.await;
        println!("    Result: {result}");
        println!("    (poll was called 6 times: 5 Pending + 1 Ready)");
    });

    // --- Step 2: Combine custom future with tokio futures ---
    // Our Countdown works alongside tokio's built-in futures in join!.
    rt.block_on(async {
        println!("\n  Step 2: Custom future in join! with tokio::time::sleep");
        let start = Instant::now();

        let (countdown_result, ()) = tokio::join!(
            Countdown { remaining: 3 },
            async {
                time::sleep(Duration::from_millis(10)).await;
                println!("    tokio sleep done at {}ms", start.elapsed().as_millis());
            },
        );
        println!("    Countdown result: {countdown_result}");
        println!("    (Both futures ran concurrently via join!)");
    });

    // --- Step 3: What async fn really does ---
    // An async fn is syntactic sugar for a function returning impl Future.
    // The compiler generates a state machine struct that implements Future.
    rt.block_on(async {
        println!("\n  Step 3: async fn desugaring");

        // This async fn...
        async fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        // ...is roughly equivalent to a struct implementing Future that
        // returns Poll::Ready(a + b) on the first poll. The compiler
        // generates the state machine automatically.

        let result = add(2, 3).await;
        println!("    add(2, 3) = {result}");
        println!("    (async fn returns a Future — .await drives it via poll)");
    });
}

// ===========================================================================
// Section 3: tokio::time — sleep, interval, timeout
// ===========================================================================
//
// tokio::time provides time-based futures:
//   - sleep(Duration) — completes after the duration (like our Sleep struct)
//   - interval(Duration) — yields periodically (like a repeating timer)
//   - timeout(Duration, future) — wraps a future with a deadline
//
// Critical distinction — tokio::time::sleep vs std::thread::sleep:
//   tokio::time::sleep yields to the executor. While one task sleeps, the
//   executor polls other ready tasks — this is cooperative multitasking.
//   std::thread::sleep blocks the OS thread entirely. If you call it inside
//   an async task, the executor thread is frozen — ALL tasks on that thread
//   are starved until the sleep completes. NEVER use std::thread::sleep in
//   async code. (Use spawn_blocking if you truly need to block.)

fn tokio_time_utilities() {
    println!("\n=== Section 3: tokio::time — sleep, interval, timeout ===\n");

    let rt = Runtime::new().unwrap();

    // --- Step 1: Sequential sleeps with timing ---
    rt.block_on(async {
        println!("  Step 1: Sequential sleeps");
        let start = Instant::now();

        time::sleep(Duration::from_millis(30)).await;
        println!("    Slept 30ms (elapsed: {}ms)", start.elapsed().as_millis());

        time::sleep(Duration::from_millis(20)).await;
        println!("    Slept 20ms (elapsed: {}ms)", start.elapsed().as_millis());

        println!("    Total ≈ 50ms (sequential = sum)");
    });

    // --- Step 2: interval with tick counting ---
    // interval automatically accounts for processing time between ticks.
    // If a tick is late, the next one fires immediately to catch up.
    rt.block_on(async {
        println!("\n  Step 2: Interval ticks");
        let mut interval = time::interval(Duration::from_millis(20));
        let start = Instant::now();

        for i in 0..4 {
            interval.tick().await;
            println!("    Tick {i} at {}ms", start.elapsed().as_millis());
        }
        println!("    (First tick fires immediately, then every 20ms)");
    });

    // --- Step 3: timeout wrapping a slow operation ---
    // timeout returns Ok(value) if the future completes in time,
    // or Err(Elapsed) if it exceeds the deadline.
    rt.block_on(async {
        println!("\n  Step 3: Timeout");

        let fast = time::timeout(Duration::from_millis(50), async {
            time::sleep(Duration::from_millis(10)).await;
            "fast result"
        })
        .await;
        println!("    Fast operation: {fast:?}");

        let slow = time::timeout(Duration::from_millis(10), async {
            time::sleep(Duration::from_millis(50)).await;
            "slow result"
        })
        .await;
        println!("    Slow operation: {slow:?}");
        println!("    (Err = Elapsed — the future was cancelled)");
    });

    // --- Step 4: Concurrent sleeps via tokio::join! ---
    // join! polls all futures concurrently. Total time = max(sleeps).
    rt.block_on(async {
        println!("\n  Step 4: Concurrent sleeps with join!");
        let start = Instant::now();

        tokio::join!(
            async {
                time::sleep(Duration::from_millis(50)).await;
                println!("    50ms sleep done at {}ms", start.elapsed().as_millis());
            },
            async {
                time::sleep(Duration::from_millis(30)).await;
                println!("    30ms sleep done at {}ms", start.elapsed().as_millis());
            },
            async {
                time::sleep(Duration::from_millis(40)).await;
                println!("    40ms sleep done at {}ms", start.elapsed().as_millis());
            },
        );
        println!("    Total ≈ 50ms (concurrent = max, not sum!)");
    });
}

// ===========================================================================
// Section 4: Spawning Tasks — tokio::spawn, JoinHandle, JoinSet
// ===========================================================================
//
// async fn laziness:
//   Calling an async fn returns a Future but does NOT start execution.
//   `let future = some_async_fn();` — nothing happens until you `.await` it
//   or hand it to an executor.
//
// tokio::spawn() creates a new task that runs concurrently on the runtime's
// thread pool.
//
// async move explanation:
//   `async { ... }` captures variables by reference (like closures).
//   `async move { ... }` moves ownership into the future, making it 'static.
//   tokio::spawn requires 'static because the spawned task may outlive the
//   calling scope and may run on any thread. Without `move`, the future would
//   hold references to the caller's stack — a dangling reference once the
//   caller returns.
//
// Non-deterministic ordering:
//   Async task completion order depends on scheduling, lock timing, IO
//   latency, and other runtime factors. Don't rely on join! branches
//   completing in argument order or JoinSet tasks finishing in spawn order.
//   If order matters: use sequential .await, or collect results and sort.
//
// JoinSet collects multiple tasks and lets you await them in completion
// order (like a concurrent bag of futures).

fn spawning_tasks() {
    println!("\n=== Section 4: Spawning Tasks — spawn, JoinHandle, JoinSet ===\n");

    let rt = Runtime::new().unwrap();

    // --- Step 1: Spawn tasks with different sleeps, await JoinHandles ---
    rt.block_on(async {
        println!("  Step 1: Spawn + JoinHandle");
        let start = Instant::now();

        let h1 = tokio::spawn(async {
            time::sleep(Duration::from_millis(30)).await;
            "task A"
        });
        let h2 = tokio::spawn(async {
            time::sleep(Duration::from_millis(10)).await;
            "task B"
        });
        let h3 = tokio::spawn(async {
            time::sleep(Duration::from_millis(20)).await;
            "task C"
        });

        // .await on JoinHandle returns Result<T, JoinError>
        println!("    {} (at {}ms)", h1.await.unwrap(), start.elapsed().as_millis());
        println!("    {} (at {}ms)", h2.await.unwrap(), start.elapsed().as_millis());
        println!("    {} (at {}ms)", h3.await.unwrap(), start.elapsed().as_millis());
        println!("    (All complete by ~30ms — they ran concurrently)");
    });

    // --- Step 2: async move — transferring ownership into a task ---
    // spawn requires 'static, so we use `async move` to move data in.
    rt.block_on(async {
        println!("\n  Step 2: async move ownership transfer");
        let message = String::from("hello from main");

        let handle = tokio::spawn(async move {
            // `message` is now owned by this task
            println!("    Task received: {message}");
            message.len()
        });

        // message is no longer accessible here — it was moved
        let len = handle.await.unwrap();
        println!("    Message length: {len}");
    });

    // --- Step 3: JoinSet — collect tasks in completion order ---
    // JoinSet is like a bag of spawned tasks. join_next() returns the
    // next task that completes, regardless of spawn order.
    rt.block_on(async {
        println!("\n  Step 3: JoinSet (completion-order collection)");
        let mut set = JoinSet::new();

        for i in 0..5 {
            let delay = (5 - i) * 10; // Task 0 slowest, task 4 fastest
            set.spawn(async move {
                time::sleep(Duration::from_millis(delay as u64)).await;
                format!("task {i} (slept {delay}ms)")
            });
        }

        let mut order = 1;
        while let Some(result) = set.join_next().await {
            println!("    Finished #{order}: {}", result.unwrap());
            order += 1;
        }
        println!("    (Fastest tasks finish first — not spawn order)");
    });

    // --- Step 4: Nested spawn — tasks spawning tasks ---
    rt.block_on(async {
        println!("\n  Step 4: Tasks spawning tasks");

        let outer = tokio::spawn(async {
            println!("    Outer task started");
            let inner = tokio::spawn(async {
                println!("    Inner task started");
                time::sleep(Duration::from_millis(10)).await;
                println!("    Inner task done");
                99
            });
            let result = inner.await.unwrap();
            println!("    Outer got inner's result: {result}");
            result + 1
        });

        let final_result = outer.await.unwrap();
        println!("    Final result: {final_result}");
    });

    // --- Step 5: Async fn laziness — calling ≠ executing ---
    // The future returned by an async fn is inert until polled.
    rt.block_on(async {
        println!("\n  Step 5: Async fn laziness");

        async fn compute() -> &'static str {
            println!("    (compute is executing!)");
            "result"
        }

        let future = compute(); // Nothing printed — not running yet
        println!("    Future created, not yet awaited");
        let val = future.await; // NOW it runs
        println!("    Awaited: {val}");
        println!("    (Calling async fn only builds the future — .await drives it)");
    });
}

// ===========================================================================
// Section 5: Combining Futures — join! and select!
// ===========================================================================
//
// join! runs multiple futures concurrently and waits for ALL to complete.
// select! runs multiple futures concurrently and returns when the FIRST
// completes, cancelling (dropping) the others.
//
// join! = AND (all must finish)     → like our JoinAll
// select! = OR (first one wins)    → no equivalent in tutorial_004
//
// join! vs spawn — important distinction:
//   join! = same task, polled round-robin on one executor thread. The futures
//     don't need Send or 'static because they share the caller's stack frame.
//     Think of it as structured concurrency — all branches live and die with
//     the join! call.
//   spawn = independent tasks on the work-stealing queue. Requires Send + 'static.
//     Tasks can run in true parallel on different OS threads (multi_thread runtime).
//     On current_thread runtime, spawn'd tasks interleave but don't parallelize.
//
// Cooperative multitasking:
//   Async Rust is cooperative — tasks yield control at .await points.
//   If a task does CPU-heavy work or calls blocking APIs before its first
//   .await, it monopolizes the executor thread. Other tasks starve until
//   it yields. Fix: insert tokio::task::yield_now().await in tight loops,
//   or use spawn_blocking for CPU-bound work.
//
// select! is powerful but subtle: the losing branches are dropped, which
// means any work between .await points in those branches is lost. This is
// the essence of "cancel safety" (covered in Section 5).

fn combining_futures() {
    println!("\n=== Section 5: Combining Futures — join! and select! ===\n");

    let rt = Runtime::new().unwrap();

    // --- Step 1: tokio::join! with timing ---
    rt.block_on(async {
        println!("  Step 1: join! — all must complete");
        let start = Instant::now();

        let (a, b, c) = tokio::join!(
            async {
                time::sleep(Duration::from_millis(30)).await;
                "alpha"
            },
            async {
                time::sleep(Duration::from_millis(50)).await;
                "beta"
            },
            async {
                time::sleep(Duration::from_millis(20)).await;
                "gamma"
            },
        );

        println!("    Results: {a}, {b}, {c}");
        println!("    Elapsed: {}ms (≈ max = 50ms)", start.elapsed().as_millis());
    });

    // --- Step 2: join! with mixed return types ---
    // Each branch can return a different type — the result is a tuple.
    rt.block_on(async {
        println!("\n  Step 2: join! with mixed types");

        let (count, name, flag) = tokio::join!(
            async { 42u64 },
            async { String::from("tokio") },
            async { true },
        );
        println!("    ({count}, {name:?}, {flag})");
    });

    // --- Step 3: select! — first wins, others cancelled ---
    // select! races multiple futures. The first to complete wins.
    // The other branches are dropped (cancelled).
    rt.block_on(async {
        println!("\n  Step 3: select! — first wins");
        let start = Instant::now();

        tokio::select! {
            _ = time::sleep(Duration::from_millis(20)) => {
                println!("    20ms timer won at {}ms", start.elapsed().as_millis());
            }
            _ = time::sleep(Duration::from_millis(50)) => {
                println!("    50ms timer won (this won't print)");
            }
        }
        println!("    (The 50ms branch was cancelled/dropped)");
    });

    // --- Step 4: select! in a loop — race work against a deadline ---
    // Common pattern: do work in iterations, but stop after a deadline.
    rt.block_on(async {
        println!("\n  Step 4: select! in a loop with deadline");
        let deadline = time::sleep(Duration::from_millis(60));
        tokio::pin!(deadline);
        let mut ticks = 0u32;

        loop {
            tokio::select! {
                _ = &mut deadline => {
                    println!("    Deadline hit after {ticks} ticks");
                    break;
                }
                _ = time::sleep(Duration::from_millis(15)) => {
                    ticks += 1;
                    println!("    Tick {ticks}");
                }
            }
        }
    });

    // --- Step 5: Cooperative multitasking — blocking before .await starves others ---
    // std::thread::sleep blocks the OS thread. In join!, the branches are polled
    // on the same thread, so a blocking call prevents other branches from progressing.
    rt.block_on(async {
        println!("\n  Step 5: Cooperative multitasking");
        let start = Instant::now();

        // Both branches use tokio::time::sleep → they run concurrently
        let ((), ()) = tokio::join!(
            async {
                time::sleep(Duration::from_millis(30)).await;
                println!("    [cooperative] Branch A done at {}ms", start.elapsed().as_millis());
            },
            async {
                time::sleep(Duration::from_millis(30)).await;
                println!("    [cooperative] Branch B done at {}ms", start.elapsed().as_millis());
            },
        );
        println!("    Total ≈ 30ms (both yielded, ran concurrently)");

        // yield_now() voluntarily reschedules without any time delay —
        // useful in CPU-bound loops to let other tasks run.
        let start = Instant::now();
        let ((), ()) = tokio::join!(
            async {
                for _ in 0..3 {
                    tokio::task::yield_now().await; // give others a chance
                }
                println!("    [yield_now] Branch A done at {}ms", start.elapsed().as_millis());
            },
            async {
                for _ in 0..3 {
                    tokio::task::yield_now().await;
                }
                println!("    [yield_now] Branch B done at {}ms", start.elapsed().as_millis());
            },
        );
        println!("    yield_now() lets branches interleave without sleeping");
        println!("    Rule: never use std::thread::sleep in async code");
    });
}

// ===========================================================================
// Section 6: Cancellation — abort, Drop, cancel safety
// ===========================================================================
//
// Cancellation in async Rust happens by dropping futures. Three ways:
//
//   1. Drop the JoinHandle without awaiting — task is DETACHED (keeps running)
//   2. Call handle.abort() — task is cancelled, its future is dropped
//   3. select! drops the losing branches automatically
//
// Cancel safety: a future is "cancel-safe" if dropping it between .await
// points doesn't lose data or corrupt state.
//
// Cancel-safe examples:
//   - tokio::time::sleep — no state to lose, just a timer registration
//   - mpsc::Receiver::recv() — message stays in channel if future is dropped
//   - TcpListener::accept() — connection goes back to the OS accept queue
//
// NOT cancel-safe examples:
//   - AsyncReadExt::read() into a buffer — partial bytes may be consumed
//     from the source but not yet written to your buffer (data lost)
//   - Multi-step protocol: first send completes, then future is dropped
//     before second send → protocol is in an inconsistent state
//
// Mitigation strategies:
//   - tokio::pin! to reuse a future across select! loop iterations (see Step 3)
//   - Use cancel-safe APIs (recv, sleep) in select! branches
//   - Know that timeout() drops the inner future if the deadline expires
//

fn cancellation_patterns() {
    println!("\n=== Section 6: Cancellation — abort, Drop, cancel safety ===\n");

    let rt = Runtime::new().unwrap();

    // --- Step 1: Spawn + abort, showing JoinError::is_cancelled ---
    rt.block_on(async {
        println!("  Step 1: abort() cancels a task");

        let handle = tokio::spawn(async {
            time::sleep(Duration::from_millis(100)).await;
            "this will never be returned"
        });

        // Abort before the task completes
        handle.abort();

        match handle.await {
            Ok(val) => println!("    Got value: {val}"),
            Err(e) if e.is_cancelled() => println!("    Task was cancelled: {e}"),
            Err(e) => println!("    Task panicked: {e}"),
        }
    });

    // --- Step 2: Detach — drop handle, task continues ---
    // Dropping a JoinHandle does NOT cancel the task. The task is detached
    // and continues running until it completes (or the runtime shuts down).
    rt.block_on(async {
        println!("\n  Step 2: Detached task (drop handle, task continues)");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let handle = tokio::spawn(async move {
            time::sleep(Duration::from_millis(20)).await;
            counter_clone.store(42, Ordering::Relaxed);
        });

        // Drop the handle — task is NOT cancelled, just detached
        drop(handle);
        println!("    Handle dropped, counter = {}", counter.load(Ordering::Relaxed));

        // Wait for the task to finish
        time::sleep(Duration::from_millis(40)).await;
        println!("    After wait, counter = {}", counter.load(Ordering::Relaxed));
        println!("    (Task ran to completion even without the handle)");
    });

    // --- Step 3: Cancel safety demonstration ---
    // When select! picks a winner, the losing branch is dropped between
    // its .await points. Any in-progress state in the losing future is lost.
    rt.block_on(async {
        println!("\n  Step 3: Cancel safety — work lost between awaits");

        let (tx, mut rx) = mpsc::channel::<String>(10);

        // Spawn a producer
        tokio::spawn(async move {
            for i in 0..3 {
                time::sleep(Duration::from_millis(15)).await;
                tx.send(format!("msg-{i}")).await.unwrap();
            }
        });

        let mut received = Vec::new();
        let deadline = time::sleep(Duration::from_millis(50));
        tokio::pin!(deadline);

        // recv() is cancel-safe: if select! drops it, no message is lost.
        // The message stays in the channel for the next recv().
        loop {
            tokio::select! {
                _ = &mut deadline => {
                    println!("    Deadline reached");
                    break;
                }
                msg = rx.recv() => {
                    if let Some(m) = msg {
                        println!("    Received: {m}");
                        received.push(m);
                    } else {
                        break;
                    }
                }
            }
        }
        println!("    Total received: {} (recv is cancel-safe)", received.len());
    });
}

// ===========================================================================
// Section 7: Async Channels — mpsc, oneshot, watch, broadcast
// ===========================================================================
//
// Why not std::sync::mpsc? It blocks the calling thread when waiting for
// messages, which would stall the entire async executor. tokio's channels
// are async — they yield control so other tasks can run while waiting.
//
// Channel types:
//   mpsc      — multi-producer, single-consumer (bounded or unbounded)
//   oneshot   — single-value, single-use (request-response pattern)
//   watch     — single-value, multi-consumer (latest-value broadcast)
//   broadcast — multi-producer, multi-consumer (all receivers see all msgs)
//
// Backpressure:
//   Backpressure is the mechanism by which a consumer signals a producer to
//   slow down. Without it, a fast producer grows an unbounded queue in memory
//   until the process crashes (OOM). Bounded channels provide natural
//   backpressure — when the buffer is full, send().await blocks the producer.
//
// Backpressure strategies beyond bounded channels:
//   - Rate limiting (Semaphore / token bucket) — cap throughput
//   - Dropping (telemetry / sampling) — discard excess under load
//   - Load shedding — reject new work at the entry point (e.g., HTTP 503)
//   - Buffering with overflow policy — ring buffer, drop-oldest
//
// broadcast overload: slow receivers get RecvError::Lagged(n) — the channel
// uses a drop-oldest policy. The receiver can handle this or skip forward.
//
// Warning: mpsc::unbounded_channel() provides NO backpressure. The internal
// queue grows without limit. Use it only when the producer is guaranteed to
// be slower than the consumer (e.g., rare event notifications).
//
// Note: oneshot::Sender::send() is NOT async — it sends immediately or
// returns Err(value) if the receiver was dropped. No .await needed.
//
// Back-ref: our mini runtime only had JoinHandle — effectively a oneshot.
// tokio has a full channel suite for different communication patterns.

fn async_channels() {
    println!("\n=== Section 7: Async Channels — mpsc, oneshot, watch, broadcast ===\n");

    let rt = Runtime::new().unwrap();

    // --- Step 1: mpsc — bounded channel with backpressure ---
    // Channel capacity of 3 means the 4th send will block until space opens.
    rt.block_on(async {
        println!("  Step 1: mpsc channel (capacity 3)");
        let (tx, mut rx) = mpsc::channel::<i32>(3);

        let producer = tokio::spawn(async move {
            for i in 1..=5 {
                println!("    Sending {i}...");
                tx.send(i).await.unwrap();
                println!("    Sent {i}");
            }
        });

        // Small delay so producer fills the buffer
        time::sleep(Duration::from_millis(5)).await;

        while let Some(val) = rx.recv().await {
            println!("    Received: {val}");
        }
        producer.await.unwrap();
        println!("    (Sender blocks when buffer is full — that's backpressure)");
    });

    // --- Step 2: oneshot — single value, request-response ---
    // Perfect for spawning a task and getting its result back.
    rt.block_on(async {
        println!("\n  Step 2: oneshot channel");
        let (tx, rx) = oneshot::channel::<String>();

        tokio::spawn(async move {
            time::sleep(Duration::from_millis(10)).await;
            tx.send("computed result".to_string()).unwrap();
        });

        let result = rx.await.unwrap();
        println!("    Got: {result}");
    });

    // --- Step 3: watch — latest-value broadcast ---
    // Receivers always see the most recent value. Useful for config updates.
    // Multiple receivers can exist; they each track what they've seen.
    rt.block_on(async {
        println!("\n  Step 3: watch channel (config update pattern)");
        let (tx, mut rx1) = watch::channel("v1.0");
        let mut rx2 = tx.subscribe();

        tx.send("v2.0").unwrap();
        tx.send("v3.0").unwrap(); // Overwrites v2.0

        // Receivers see only the latest value
        rx1.changed().await.unwrap();
        println!("    Receiver 1 sees: {}", *rx1.borrow());

        rx2.changed().await.unwrap();
        println!("    Receiver 2 sees: {}", *rx2.borrow());
        println!("    (Both see v3.0 — intermediate v2.0 was overwritten)");
    });

    // --- Step 4: broadcast — all receivers see all messages ---
    // Unlike mpsc, every receiver gets every message. Useful for fan-out.
    rt.block_on(async {
        println!("\n  Step 4: broadcast channel");
        let (tx, mut rx1) = broadcast::channel::<String>(16);
        let mut rx2 = tx.subscribe();

        tx.send("hello".to_string()).unwrap();
        tx.send("world".to_string()).unwrap();

        println!("    Receiver 1: {}", rx1.recv().await.unwrap());
        println!("    Receiver 1: {}", rx1.recv().await.unwrap());
        println!("    Receiver 2: {}", rx2.recv().await.unwrap());
        println!("    Receiver 2: {}", rx2.recv().await.unwrap());
        println!("    (Both receivers got both messages)");
    });

    // --- Step 5: Comparison table ---
    println!("\n  Channel comparison:");
    println!("    Type        Producers  Consumers  Values  Backpressure");
    println!("    ────────────────────────────────────────────────────────");
    println!("    mpsc        many       one        many    yes (bounded)");
    println!("    oneshot     one        one        one     N/A");
    println!("    watch       one        many       latest  no (overwrites)");
    println!("    broadcast   many       many       many    yes (bounded)");
}

// ===========================================================================
// Section 8: Synchronization — Mutex, Semaphore, Notify
// ===========================================================================
//
// tokio::sync::Mutex vs std::sync::Mutex:
//   std::sync::Mutex in async code is FINE for short critical sections where
//   the lock is never held across an .await point. It's faster than tokio's
//   Mutex because it doesn't need async-aware machinery — just a simple
//   OS-level lock/unlock. Most mutexes in async code should be std::sync.
//
//   tokio::sync::Mutex is required when you must hold the lock across an
//   .await point. If you use std::sync::Mutex and .await while holding
//   the MutexGuard, the blocked OS thread can't run other tasks — the
//   executor starves. tokio's Mutex yields when contended, letting the
//   executor poll other tasks while waiting.
//
//   Rule of thumb: start with std::sync::Mutex. Switch to tokio::sync::Mutex
//   only when you need to .await inside the critical section.
//
// Semaphore limits concurrent access to a resource (rate limiting, connection
// pools). Notify is a lightweight signal — one task can wake another.
//
// Back-ref: our TaskWaker used AtomicBool for wake signaling. Notify is
// the production equivalent — it handles the waker registration correctly
// and avoids spurious wakes.

fn sync_primitives() {
    println!("\n=== Section 8: Synchronization — Mutex, Semaphore, Notify ===\n");

    let rt = Runtime::new().unwrap();

    // --- Step 1: tokio::sync::Mutex across .await ---
    // We increment a counter from multiple tasks, with a sleep inside the
    // critical section to show that the lock is held across await points.
    rt.block_on(async {
        println!("  Step 1: tokio::sync::Mutex held across .await");
        let counter = Arc::new(Mutex::new(0u32));
        let mut handles = vec![];

        for i in 0..3 {
            let counter = counter.clone();
            handles.push(tokio::spawn(async move {
                let mut lock = counter.lock().await;
                let before = *lock;
                // Simulating async work while holding the lock
                time::sleep(Duration::from_millis(5)).await;
                *lock += 1;
                println!("    Task {i}: {before} → {}", *lock);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }
        println!("    Final counter: {}", *counter.lock().await);
    });

    // --- Step 2: Semaphore — limit concurrency ---
    // 5 tasks compete, but only 2 can run concurrently.
    // We track the peak concurrency with an AtomicUsize.
    rt.block_on(async {
        println!("\n  Step 2: Semaphore (max 2 concurrent)");
        let semaphore = Arc::new(Semaphore::new(2));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for i in 0..5 {
            let sem = semaphore.clone();
            let active = active.clone();
            let peak = peak.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let current = active.fetch_add(1, Ordering::Relaxed) + 1;
                peak.fetch_max(current, Ordering::Relaxed);
                println!("    Task {i} entered (active: {current})");

                time::sleep(Duration::from_millis(20)).await;

                active.fetch_sub(1, Ordering::Relaxed);
                println!("    Task {i} exited");
                // _permit is dropped here, releasing the semaphore slot
            }));
        }

        for h in handles {
            h.await.unwrap();
        }
        println!("    Peak concurrency: {}", peak.load(Ordering::Relaxed));
    });

    // --- Step 3: Notify — producer-consumer signaling ---
    // Notify lets one task wake another without sending data.
    rt.block_on(async {
        println!("\n  Step 3: Notify (signal between tasks)");
        let notify = Arc::new(Notify::new());
        let data = Arc::new(std::sync::Mutex::new(Vec::<i32>::new()));

        let notify_clone = notify.clone();
        let data_clone = data.clone();

        // Consumer waits for notification, then reads data
        let consumer = tokio::spawn(async move {
            for _ in 0..3 {
                notify_clone.notified().await;
                let items = data_clone.lock().unwrap();
                println!("    Consumer woke up, data: {:?}", *items);
            }
        });

        // Producer writes data, then notifies
        for i in 1..=3 {
            time::sleep(Duration::from_millis(10)).await;
            data.lock().unwrap().push(i);
            println!("    Producer pushed {i}, notifying...");
            notify.notify_one();
        }

        consumer.await.unwrap();
    });
}

// ===========================================================================
// Section 9: Real Async IO — tokio::fs and spawn_blocking
// ===========================================================================
//
// tokio::fs wraps std::fs operations inside spawn_blocking internally —
// file IO on most OSes doesn't have true async support (except io_uring
// on Linux), so tokio offloads it to a blocking thread pool.
//
// spawn_blocking is also your tool for CPU-intensive work: it runs a
// closure on a dedicated thread pool so it doesn't block async tasks.
//
// Back-ref: our SimulatedIo used std::thread::spawn with stored Wakers
// to run blocking work in the background. tokio::fs and spawn_blocking
// do the same thing, but integrated with the runtime's thread pool.

fn async_io() {
    println!("\n=== Section 9: Real Async IO — tokio::fs and spawn_blocking ===\n");

    let rt = Runtime::new().unwrap();
    let tmp_path = std::env::temp_dir().join("tutorial_004b_test.txt");

    // --- Step 1: tokio::fs::write + read_to_string ---
    let path = tmp_path.clone();
    rt.block_on(async {
        println!("  Step 1: tokio::fs write + read");

        tokio::fs::write(&path, "hello from tokio async fs!").await.unwrap();
        println!("    Wrote file: {}", path.display());

        let contents = tokio::fs::read_to_string(&path).await.unwrap();
        println!("    Read back: {contents}");
    });

    // --- Step 2: Lower-level File + AsyncReadExt ---
    let path = tmp_path.clone();
    rt.block_on(async {
        println!("\n  Step 2: File::open + AsyncReadExt");
        use tokio::io::AsyncReadExt;

        let mut file = tokio::fs::File::open(&path).await.unwrap();
        let mut buf = vec![0u8; 5];
        let n = file.read(&mut buf).await.unwrap();
        println!("    Read {n} bytes: {:?}", String::from_utf8_lossy(&buf[..n]));
    });

    // --- Step 3: spawn_blocking for CPU-intensive work ---
    rt.block_on(async {
        println!("\n  Step 3: spawn_blocking for CPU work");
        let start = Instant::now();

        let handle = tokio::task::spawn_blocking(|| {
            // Simulate CPU-intensive work
            let mut sum = 0u64;
            for i in 0..1_000_000 {
                sum = sum.wrapping_add(i);
            }
            sum
        });

        let result = handle.await.unwrap();
        println!("    Computed sum: {result} (in {}ms)", start.elapsed().as_millis());
        println!("    (Ran on blocking thread pool — didn't starve async tasks)");
    });

    // --- Step 4: Cleanup ---
    rt.block_on(async {
        println!("\n  Step 4: Cleanup");
        tokio::fs::remove_file(&tmp_path).await.unwrap();
        println!("    Removed {}", tmp_path.display());
    });
}

// ===========================================================================
// Section 10: Practical Pattern — Graceful Shutdown
// ===========================================================================
//
// Graceful shutdown combines several tokio features:
//   - watch channel as a shutdown signal
//   - select! to race work against the shutdown signal
//   - spawned tasks that finish their current work unit before exiting
//
// This pattern is common in production services: you want to stop accepting
// new work but let in-flight requests complete.
//
// We also show a brief actor pattern teaser: a task that owns state and
// processes messages via an mpsc channel, using oneshot for responses.
//
// Back-ref: our mini runtime couldn't express this — we had no channels,
// no select!, and no way to signal running tasks. This demonstrates why
// tokio needs these abstractions beyond what a basic poll loop provides.

fn graceful_shutdown_pattern() {
    println!("\n=== Section 10: Practical Pattern — Graceful Shutdown ===\n");

    let rt = Runtime::new().unwrap();

    // --- Shutdown with watch channel ---
    rt.block_on(async {
        println!("  Graceful shutdown demo:");

        // Step 1: watch channel as shutdown signal
        let (shutdown_tx, _) = watch::channel(false);
        let mut worker_handles = vec![];

        // Step 2: Spawn 3 worker tasks
        for id in 0..3 {
            let mut shutdown_rx = shutdown_tx.subscribe();
            worker_handles.push(tokio::spawn(async move {
                let mut ticks = 0u32;

                loop {
                    tokio::select! {
                        // Listen for shutdown signal
                        _ = shutdown_rx.changed() => {
                            if *shutdown_rx.borrow() {
                                println!("    Worker {id}: shutting down after {ticks} ticks");
                                break;
                            }
                        }
                        // Do periodic work
                        _ = time::sleep(Duration::from_millis(15)) => {
                            ticks += 1;
                        }
                    }
                }
                ticks
            }));
        }

        // Step 3: Send shutdown after 80ms
        time::sleep(Duration::from_millis(80)).await;
        println!("    Main: sending shutdown signal");
        shutdown_tx.send(true).unwrap();

        // Step 4: Await all workers and collect tick counts
        for (i, handle) in worker_handles.into_iter().enumerate() {
            let ticks = handle.await.unwrap();
            println!("    Worker {i} completed {ticks} ticks");
        }
    });

    // --- Actor pattern teaser ---
    // A task that owns state and communicates via channels.
    // Requests come in via mpsc, responses go back via oneshot.
    rt.block_on(async {
        println!("\n  Actor pattern teaser (counter actor):");

        enum CounterMsg {
            Increment,
            Get(oneshot::Sender<u32>),
        }

        let (tx, mut rx) = mpsc::channel::<CounterMsg>(32);

        // The actor task — owns the state, processes messages
        tokio::spawn(async move {
            let mut count = 0u32;
            while let Some(msg) = rx.recv().await {
                match msg {
                    CounterMsg::Increment => count += 1,
                    CounterMsg::Get(reply) => {
                        let _ = reply.send(count);
                    }
                }
            }
        });

        // Interact with the actor via messages
        tx.send(CounterMsg::Increment).await.unwrap();
        tx.send(CounterMsg::Increment).await.unwrap();
        tx.send(CounterMsg::Increment).await.unwrap();

        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(CounterMsg::Get(reply_tx)).await.unwrap();
        let count = reply_rx.await.unwrap();
        println!("    Counter actor value: {count}");
        println!("    (State is fully encapsulated — no shared mutexes needed)");
    });
}

// ===========================================================================
// pub fn run()
// ===========================================================================

pub fn run() {
    tokio_runtime_and_block_on();
    the_future_trait_and_polling();
    tokio_time_utilities();
    spawning_tasks();
    combining_futures();
    cancellation_patterns();
    async_channels();
    sync_primitives();
    async_io();
    graceful_shutdown_pattern();
}
