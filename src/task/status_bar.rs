// top status bar task
use crate::println;
use crate::vga_buffer::{WRITER, Color, ColorCode};
use crate::task::time::TickStream;
use futures_util::stream::StreamExt;
use x86_64::instructions::interrupts;
use core::fmt::Write;
// main loop for the status bar task
pub async fn run() {
    let mut ticker = TickStream::new();
    let chars = ['|', '/', '-', '\\'];
    let mut i = 0;
    
    
    draw_bar(0);
    while let Some(count) = ticker.next().await {
        let spinner = chars[i % 4];
        i += 1;
        draw_status(count, spinner);
    }
}
// draws the initial background of the bar
fn draw_bar(initial_count: usize) {
     interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let color = ColorCode::new(Color::Black, Color::LightGray);
        
        for col in 0..80 {
            writer.write_at(0, col, b' ', color);
        }
        
        let label = "NewTownOS Multitasking Environment";
        for (j, byte) in label.bytes().enumerate() {
             writer.write_at(0, j + 1, byte, color);
        }
    });
    draw_status(initial_count, '|');
}
// updates the dynamic status info
fn draw_status(count: usize, spinner: char) {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let color = ColorCode::new(Color::Black, Color::LightGray);
        
        
        use core::fmt::Write;
        struct StringWriter {
            buf: [u8; 32],
            len: usize,
        }
        impl Write for StringWriter {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for b in s.bytes() {
                    if self.len < self.buf.len() {
                        self.buf[self.len] = b;
                        self.len += 1;
                    }
                }
                Ok(())
            }
        }
        let mut sw = StringWriter { buf: [0; 32], len: 0 };
        write!(sw, "Ticks: {} {}", count, spinner).ok();
        
        let start_col = 80 - sw.len - 1;
        for (j, &byte) in sw.buf[..sw.len].iter().enumerate() {
            writer.write_at(0, start_col + j, byte, color);
        }
    });
}
