use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct CountdownFuture {
    name: String,
    count: u32,
}

impl CountdownFuture {
    fn new(name: &str, from: u32) -> Self {
        Self {
            name: name.to_string(),
            count: from,
        }
    }
}

impl Future for CountdownFuture {
    type Output = String;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.count == 0 {
            Poll::Ready(format!("{}: liftoff!", this.name))
        } else {
            println!("  [{}] countdown: {}...", this.name, this.count);
            this.count -= 1;
            Poll::Pending
        }
    }
}

// ============================
// JoinTwo combinator
// ============================

struct JoinTwo<A, B>
where
    A: Future,
    B: Future,
{
    a: Pin<Box<A>>,
    b: Pin<Box<B>>,
    result_a: Option<A::Output>,
    result_b: Option<B::Output>,
}

impl<A: Future, B: Future> Unpin for JoinTwo<A, B> {}

impl<A, B> JoinTwo<A, B>
where
    A: Future,
    B: Future,
{
    fn new(a: A, b: B) -> Self {
        Self {
            a: Box::pin(a),
            b: Box::pin(b),
            result_a: None,
            result_b: None,
        }
    }
}

impl<A, B> Future for JoinTwo<A, B>
where
    A: Future,
    B: Future,
{
    type Output = (A::Output, B::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Poll A if not yet ready
        if this.result_a.is_none() {
            if let Poll::Ready(val) = this.a.as_mut().poll(cx) {
                this.result_a = Some(val);
            }
        }

        // Poll B if not yet ready
        if this.result_b.is_none() {
            if let Poll::Ready(val) = this.b.as_mut().poll(cx) {
                this.result_b = Some(val);
            }
        }

        // Both done?
        if this.result_a.is_some() && this.result_b.is_some() {
            let a = this.result_a.take().unwrap();
            let b = this.result_b.take().unwrap();
            Poll::Ready((a, b))
        } else {
            Poll::Pending
        }
    }
}

// ============================
// Simple block_on executor
// ============================

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(result) => return result,
            Poll::Pending => {
                // Busy-poll: keeps re-polling until the future completes.
            }
        }
    }
}

pub fn run() {
    // --- Single countdown ---
    println!("--- Single countdown ---");
    let result = block_on(CountdownFuture::new("Rocket", 5));
    println!("  Result: {}\n", result);

    // --- Two countdowns via JoinTwo ---
    println!("--- JoinTwo combinator ---");
    let joined = JoinTwo::new(
        CountdownFuture::new("Alpha", 3),
        CountdownFuture::new("Beta", 4),
    );
    let (a, b) = block_on(joined);
    println!("  Result A: {}", a);
    println!("  Result B: {}", b);
}
