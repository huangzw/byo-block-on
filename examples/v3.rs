use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;

use crossbeam::sync::Parker;
use futures::channel::oneshot;

/// Runs a future to completion on the current thread.
fn block_on<F: Future>(future: F) -> F::Output {
    // Pin the future on the stack.
    pin_utils::pin_mut!(future);

    thread_local! {
        // Parker and waker associated with the current thread.
        static CACHE: (Parker, Waker) = {
            let parker = Parker::new();
            let unparker = parker.unparker().clone();
            let waker = async_task::waker_fn(move || unparker.unpark());
            (parker, waker)
        };
    }

    CACHE.with(|(parker, waker)| {
        // Create the task context.
        let cx = &mut Context::from_waker(&waker);

        // Keep polling the future until completion.
        loop {
            match future.as_mut().poll(cx) {
                Poll::Ready(output) => return output,
                Poll::Pending => parker.park(),
            }
        }
    })
}

fn main() {
    let (s, r) = oneshot::channel();

    // Spawn a thread that will send a message through the channel.
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        s.send("Hello, world!").unwrap();
    });

    // Block until the message is received.
    let msg = block_on(async {
        println!("Awaiting...");
        r.await.unwrap()
    });

    println!("{}", msg);
}
