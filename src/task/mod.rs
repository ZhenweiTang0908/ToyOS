use core::{future::Future, pin::Pin};
use alloc::boxed::Box;
use core::task::{Context, Poll};
use core::sync::atomic::{AtomicU64, Ordering};
pub mod simple_executor;
pub mod keyboard;
pub mod executor;
pub mod shell;
pub mod time;
pub mod status_bar;
pub mod snake;
pub struct Task {
    id: TaskId,
    name: &'static str,
    future: Pin<Box<dyn Future<Output = ()>>>,
}
impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task::new_named("task", future)
    }
    pub fn new_named(name: &'static str, future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            name,
            future: Box::pin(future),
        }
    }
    pub(crate) fn id(&self) -> TaskId {
        self.id
    }
    pub(crate) fn name(&self) -> &'static str {
        self.name
    }
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TaskId(u64);
impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
    pub(crate) fn as_u64(self) -> u64 {
        self.0
    }
    pub(crate) fn from_raw(raw: u64) -> Self {
        TaskId(raw)
    }
}
