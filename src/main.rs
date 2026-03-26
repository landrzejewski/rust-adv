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
mod mod_006a_ffi_c;
mod mod_006b_ffi_python;

fn main() {
    mod_004_async_await_and_tokio::run()
}
