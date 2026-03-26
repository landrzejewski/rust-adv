use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::runtime::Builder;
use tokio::time;

mod mod_001_idioms_and_patterns;
mod mod_002_macros;
mod mod_003_threads_and_concurrency;
mod exercises;
mod mod_004_async_await_custom;
mod mod_004_async_await_and_tokio;

async fn run() -> i32 {
    println!("Thread: {}", std::thread::current().name().unwrap());
    println!("Starting exercise");
    40
}

struct Countdown {
    count: i32,
}

impl Future for Countdown {
    type Output = i32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.count == 0 {
            Poll::Ready(0)
        } else {
            self.count -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }

}

fn main() {
   // let rt = tokio::runtime::Runtime::new().unwrap();
    let rt = Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

        let data = vec![1, 2, 3, 4, 5];

        rt.block_on(async {
           let h1 = tokio::spawn(async move {
               println!("{:?}", data);
               time::sleep(Duration::from_secs(2)).await;
               println!("Thread finished 1: {}", std::thread::current().name().unwrap());
           });
            let h2 = tokio::spawn(async {
                time::sleep(Duration::from_secs(1)).await;
                println!("Thread finished 2: {}", std::thread::current().name().unwrap());
            });
            h1.await.unwrap();
            h2.await.unwrap();
        });
       //sleep(Duration::from_secs(5));

}
