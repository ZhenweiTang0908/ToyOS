// entry point of the kernel
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(toy_os::test_runner)]
#![reexport_test_harness_main = "test_main"]
use core::panic::PanicInfo;
extern crate alloc;
use toy_os::{println, print};
use bootloader::{BootInfo, entry_point};
use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};
use toy_os::memory::BootInfoFrameAllocator;
use toy_os::{allocator, memory};
entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use x86_64::{structures::paging::Page, VirtAddr};
    println!("Hello World{}", "!");
    toy_os::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    // initialize page table mapper
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    
    // initialize the heap allocator
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
    #[cfg(test)]
    test_main();
    
    
    use toy_os::task::{Task, executor::Executor};
    use toy_os::task::shell;
    use toy_os::task::status_bar;
    let mut executor = Executor::new();
    executor.spawn(Task::new(status_bar::run()));
    executor.spawn(Task::new(shell::run()));
    // run the task executor
    executor.run();
    println!("It did not crash!");
    toy_os::hlt_loop();
}
async fn async_number() -> u32 {
    42
}
async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
#[cfg(not(test))]
#[panic_handler]
// standard panic handler
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    toy_os::hlt_loop();
}
#[cfg(test)]
#[panic_handler]
// panic handler for tests
fn panic(info: &PanicInfo) -> ! {
    toy_os::test_panic_handler(info)
}
