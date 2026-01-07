// async timer stream implementation
use core::{pin::Pin, task::{Poll, Context}};
use futures_util::stream::Stream;
use crate::interrupts::{TIMER_WAKERS, TICK_COUNTER};
use core::sync::atomic::{Ordering, AtomicUsize};
static NEXT_WAKER_ID: AtomicUsize = AtomicUsize::new(0);
pub struct TickStream {
    last_seen: usize,
    waker_id: usize,
}
impl TickStream {
    pub fn new() -> Self {
        let id = NEXT_WAKER_ID.fetch_add(1, Ordering::Relaxed) % TIMER_WAKERS.len();
        TickStream { 
            last_seen: TICK_COUNTER.load(Ordering::Relaxed), 
            waker_id: id 
        }
    }
}
impl Stream for TickStream {
    type Item = usize;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<usize>> {
        let current = TICK_COUNTER.load(Ordering::Relaxed);
        
        if current > self.last_seen {
            self.last_seen = current;
            Poll::Ready(Some(current))
        } else {
            TIMER_WAKERS[self.waker_id].register(&cx.waker());
            let current_recheck = TICK_COUNTER.load(Ordering::Relaxed);
            if current_recheck > self.last_seen {
                TIMER_WAKERS[self.waker_id].take();
                self.last_seen = current_recheck;
                Poll::Ready(Some(current_recheck))
            } else {
                Poll::Pending
            }
        }
    }
}
