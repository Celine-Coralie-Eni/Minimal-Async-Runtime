use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, RawWaker, RawWakerVTable, Waker},
    time::{Duration, Instant},
};

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;
type TaskQueue = Arc<Mutex<VecDeque<Task>>>;

struct MiniRuntime {
    queue: TaskQueue,
}

impl MiniRuntime {
    fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn spawn<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.queue.lock().unwrap().push_back(Box::pin(fut));
    }

    fn run(&self) {
        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);

        while let Some(mut task) = self.queue.lock().unwrap().pop_front() {
            if task.as_mut().poll(&mut cx).is_pending() {
                self.queue.lock().unwrap().push_back(task);
            }
        }
    }
}

fn dummy_waker() -> Waker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

fn sleep(duration: Duration) -> impl Future<Output = ()> {
    let start = Instant::now();
    async move {
        while Instant::now() - start < duration {
            yield_now().await;
        }
    }
}

fn yield_now() -> impl Future<Output = ()> {
    async {}
}

/// Join multiple futures
#[macro_export]
macro_rules! join_all {
    ($($fut:expr),+ $(,)?) => {
        async {
            ($($fut.await,)+)
        }
    };
}

#[macro_export]
macro_rules! mini_rt {
    ($body:expr) => {{
        let rt = MiniRuntime::new();
        rt.spawn(async move { $body.await });
        rt.run();
    }};
}

async fn task_one() {
    println!("task one: start");
    yield_now().await;
    sleep(Duration::from_secs(1)).await;
    println!("task one: done");
}

async fn task_two() {
    println!("task two: start");
    yield_now().await;
    sleep(Duration::from_secs(2)).await;
    println!("task two: done");
}

fn main() {
    println!("Runtime started...");
    mini_rt! {
        async {
            join_all!(task_one(), task_two()).await;
        }
    }
}