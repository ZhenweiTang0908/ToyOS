// raw keyboard input buffer
use alloc::collections::vec_deque::VecDeque;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::instructions::interrupts;
lazy_static! {
    pub static ref KEYBOARD_BUFFER: Mutex<VecDeque<char>> = Mutex::new(VecDeque::new());
}
// adds a character to the input buffer
pub fn push_key(c: char) {
    interrupts::without_interrupts(|| {
        KEYBOARD_BUFFER.lock().push_back(c);
    });
}
// retrieves the next character from the buffer
pub fn pop_key() -> Option<char> {
    interrupts::without_interrupts(|| {
        KEYBOARD_BUFFER.lock().pop_front()
    })
}
