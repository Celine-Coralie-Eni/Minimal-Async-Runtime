use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
    sync::{Arc, Mutex},
};

type Task = Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Clone)]
struct Runtime {
    tasks: Arc<Mutex<VecDeque<Task>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl Runtime {
    fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(VecDeque::new())),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push_back(Box::pin(future));
        if let Some(waker) = self.waker.lock().unwrap().take() {
            waker.wake();
        }
    }

    fn block_on<F>(&mut self, future: F) -> F::Output
    where
        F: Future,
    {
        let mut future = Box::pin(future);
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(output) => return output,
                Poll::Pending => {
                    let mut tasks = self.tasks.lock().unwrap();
                    if tasks.is_empty() {
                        break;
                    }
                    while let Some(mut task) = tasks.pop_front() {
                        if task.as_mut().poll(&mut cx).is_pending() {
                            tasks.push_back(task);
                        }
                    }
                }
            }
        }
        panic!("Future never completed");
    }
}

pub fn sleep(duration: Duration) -> Sleep {
    Sleep {
        deadline: Instant::now() + duration,
    }
}

pub struct Sleep {
    deadline: Instant,
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (sender, receiver) = futures::channel::oneshot::channel();
    let future = async move {
        let output = future.await;
        let _ = sender.send(output);
    };
    Runtime::new().spawn(future);
    JoinHandle { receiver }
}

pub struct JoinHandle<T> {
    receiver: futures::channel::oneshot::Receiver<T>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.receiver).poll(cx).map(|result| result.unwrap())
    }
}

lazy_static::lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new();
}

#[macro_export]
macro_rules! mini_rt {
    ($($t:tt)*) => {
        fn main() {
            let mut rt = $crate::Runtime::new();
            rt.block_on(async { $($t)* });
        }
    };
} 