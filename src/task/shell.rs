// interactive shell task with command support
use crate::{println, print};
use crate::vga_buffer::WRITER;
use crate::task::keyboard::ScancodeStream;
use crate::task::time::TickStream;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use futures_util::stream::StreamExt;
use alloc::string::String;
use alloc::vec::Vec;
use x86_64::instructions::interrupts;
// main loop for the shell task
pub async fn run() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(ScancodeSet1::new(),
        layouts::Us104Key, HandleControl::Ignore);
    print_banner();
    print_prompt();
    let mut line_buffer = String::new();
    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => {
                        match character {
                            '\u{8}' => { 
                                if !line_buffer.is_empty() {
                                    line_buffer.pop();
                                    delete_char();
                                }
                            }
                            '\n' => { 
                                print!("\n");
                                execute_command(&line_buffer).await;
                                line_buffer.clear();
                                print_prompt();
                            }
                            c => {
                                if !c.is_ascii_control() {
                                    print!("{}", c);
                                    line_buffer.push(c);
                                }
                            }
                        }
                    }
                    DecodedKey::RawKey(_) => {}
                }
            }
        }
    }
}
// prints the toyos ascii art
fn print_banner() {
    println!(r#"
  _   _                 _______                      ____   _____ 
 | \ | |               |__   __|                    / __ \ / ____|
 |  \| | _____      __    | | _____      ___ __    | |  | | (___  
 | . ` |/ _ \ \ /\ / /    | |/ _ \ \ /\ / / '_ \   | |  | |\___ \ 
 | |\  |  __/\ V  V /     | | (_) \ V  V /| | | |  | |__| |____) |
 |_| \_|\___| \_/\_/      |_|\___/ \_/\_/ |_| |_|   \____/|_____/ 
    "#);
    println!("Welcome to NewTownOS Shell!");
    println!("Type 'help' to see available commands.\n");
}
// prints the shell prompt
fn print_prompt() {
    print!("NewTownOS> ");
}
// removes the last char from screen
fn delete_char() {
    interrupts::without_interrupts(|| {
        WRITER.lock().backspace();
    });
}
// executes the user entered command
async fn execute_command(command: &str) {
    let mut parts = command.trim().split_whitespace();
    let cmd = match parts.next() {
        Some(s) => s,
        None => return, 
    };
    match cmd {
        "help" => {
            println!("Available commands:");
            println!("  help       - Show this help message");
            println!("  echo <txt> - Print back text");
            println!("  clear      - Clear the screen");
            println!("  shutdown   - Exit QEMU");
            println!("  heap       - Show heap memory info");
            println!("  alloc_test - Test heap allocation");
            println!("  snake      - Play Snake game!");
            println!("  panic      - Trigger a kernel panic");
            println!("  ps         - List active tasks");
            println!("  kill <id>  - Request a task to stop");
            println!("  sleep <n>  - Sleep for n timer ticks");
        }
        "echo" => {
            let rest: String = parts.collect::<Vec<&str>>().join(" ");
            println!("{}", rest);
        }
        "clear" => {
            interrupts::without_interrupts(|| {
                WRITER.lock().clear_screen();
            });
        }
        "shutdown" => {
            println!("Shutting down...");
            crate::exit_qemu(crate::QemuExitCode::Success);
        }
        "heap" => {
             println!("Heap Start: 0x{:x}", crate::allocator::HEAP_START);
             println!("Heap Size:  {} bytes", crate::allocator::HEAP_SIZE);
        }
        "alloc_test" => {
            let mut vec = Vec::new();
            println!("Allocating vector...");
            for i in 0..1000 {
                vec.push(i);
            }
            println!("Vector allocated at {:p}, size: {}", vec.as_slice(), vec.len());
            println!("Testing value at index 500: {}", vec[500]);
            println!("Dropping vector (freeing memory)...");
        }
        "snake" => {
             println!("Starting Snake Game... (Press 'q' or Enter to exit)");
             crate::task::snake::run().await;
        }
        "panic" => {
            panic!("Manual panic triggered by user!");
        }
        "ps" => {
            let tasks = crate::task::executor::snapshot_tasks();
            if tasks.is_empty() {
                println!("No active tasks.");
                return;
            }

            println!("ID   STATE          POLLS NAME");
            for task in tasks {
                println!(
                    "{:>2}   {:<13} {:>5} {}",
                    task.id,
                    task.state.as_str(),
                    task.poll_count,
                    task.name
                );
            }
        }
        "kill" => {
            let Some(raw_id) = parts.next() else {
                println!("Usage: kill <task_id>");
                return;
            };

            let task_id = match raw_id.parse::<u64>() {
                Ok(id) => id,
                Err(_) => {
                    println!("Invalid task id: '{}'", raw_id);
                    return;
                }
            };

            match crate::task::executor::request_kill(task_id) {
                crate::task::executor::KillRequestResult::Queued => {
                    println!("Kill requested for task {}.", task_id);
                }
                crate::task::executor::KillRequestResult::AlreadyQueued => {
                    println!("Task {} is already waiting to be killed.", task_id);
                }
                crate::task::executor::KillRequestResult::NotFound => {
                    println!("Task {} not found.", task_id);
                }
            }
        }
        "sleep" => {
            let Some(raw_ticks) = parts.next() else {
                println!("Usage: sleep <ticks>");
                return;
            };

            let ticks = match raw_ticks.parse::<usize>() {
                Ok(n) if n > 0 => n,
                Ok(_) => {
                    println!("Ticks must be > 0.");
                    return;
                }
                Err(_) => {
                    println!("Invalid tick count: '{}'", raw_ticks);
                    return;
                }
            };

            println!("Sleeping for {} ticks...", ticks);
            sleep_ticks(ticks).await;
            println!("Awake.");
        }
        _ => {
            println!("Unknown command: '{}'", cmd);
            println!("Type 'help' to list commands.");
        }
    }
}

async fn sleep_ticks(ticks: usize) {
    let mut ticker = TickStream::new();
    for _ in 0..ticks {
        ticker.next().await;
    }
}
