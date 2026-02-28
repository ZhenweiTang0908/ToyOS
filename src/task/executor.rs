use super::{Task, TaskId};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    task::Wake,
    vec::Vec,
};
use core::task::{Context, Poll, Waker};
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Waiting,
    KillRequested,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskState::Ready => "Ready",
            TaskState::Running => "Running",
            TaskState::Waiting => "Waiting",
            TaskState::KillRequested => "KillRequested",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TaskSnapshot {
    pub id: u64,
    pub name: &'static str,
    pub state: TaskState,
    pub poll_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillRequestResult {
    Queued,
    AlreadyQueued,
    NotFound,
}

#[derive(Debug, Clone, Copy)]
struct TaskStats {
    name: &'static str,
    state: TaskState,
    poll_count: u64,
}

lazy_static! {
    static ref TASK_STATS: Mutex<BTreeMap<u64, TaskStats>> = Mutex::new(BTreeMap::new());
    static ref KILL_REQUESTS: Mutex<BTreeSet<u64>> = Mutex::new(BTreeSet::new());
}

pub fn snapshot_tasks() -> Vec<TaskSnapshot> {
    let stats = TASK_STATS.lock();
    stats
        .iter()
        .map(|(&id, info)| TaskSnapshot {
            id,
            name: info.name,
            state: info.state,
            poll_count: info.poll_count,
        })
        .collect()
}

pub fn request_kill(task_id: u64) -> KillRequestResult {
    if !TASK_STATS.lock().contains_key(&task_id) {
        return KillRequestResult::NotFound;
    }

    {
        let mut requests = KILL_REQUESTS.lock();
        if !requests.insert(task_id) {
            return KillRequestResult::AlreadyQueued;
        }
    }

    set_task_state(TaskId::from_raw(task_id), TaskState::KillRequested);
    KillRequestResult::Queued
}

fn register_task(task_id: TaskId, name: &'static str) {
    TASK_STATS.lock().insert(
        task_id.as_u64(),
        TaskStats {
            name,
            state: TaskState::Ready,
            poll_count: 0,
        },
    );
}

fn unregister_task(task_id: TaskId) {
    TASK_STATS.lock().remove(&task_id.as_u64());
}

fn set_task_state(task_id: TaskId, state: TaskState) {
    let mut stats = TASK_STATS.lock();
    if let Some(info) = stats.get_mut(&task_id.as_u64()) {
        if info.state != TaskState::KillRequested || state == TaskState::KillRequested {
            info.state = state;
        }
    }
}

fn bump_poll_count(task_id: TaskId) {
    if let Some(info) = TASK_STATS.lock().get_mut(&task_id.as_u64()) {
        info.poll_count += 1;
    }
}

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}
impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new(),
        }
    }
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id();
        let task_name = task.name();
        if self.tasks.insert(task_id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
        register_task(task_id, task_name);
        self.task_queue.push(task_id).expect("queue full");
        set_task_state(task_id, TaskState::Ready);
    }
    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }
    fn run_ready_tasks(&mut self) {
        self.apply_kill_requests();
        while let Some(task_id) = self.task_queue.pop() {
            let waker = self
                .waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, self.task_queue.clone()));
            let mut context = Context::from_waker(waker);

            set_task_state(task_id, TaskState::Running);
            bump_poll_count(task_id);

            let poll_result = match self.tasks.get_mut(&task_id) {
                Some(task) => task.poll(&mut context),
                None => continue,
            };

            match poll_result {
                Poll::Ready(()) => {
                    self.tasks.remove(&task_id);
                    self.waker_cache.remove(&task_id);
                    unregister_task(task_id);
                }
                Poll::Pending => {
                    set_task_state(task_id, TaskState::Waiting);
                }
            }

            self.apply_kill_requests();
        }
        self.apply_kill_requests();
    }
    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};
        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    fn apply_kill_requests(&mut self) {
        let pending_ids: Vec<u64> = {
            let mut requests = KILL_REQUESTS.lock();
            if requests.is_empty() {
                return;
            }
            let ids: Vec<u64> = requests.iter().copied().collect();
            requests.clear();
            ids
        };

        for raw_id in pending_ids {
            let task_id = TaskId::from_raw(raw_id);
            self.tasks.remove(&task_id);
            self.waker_cache.remove(&task_id);
            unregister_task(task_id);
        }
    }
}
struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}
impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}
impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
