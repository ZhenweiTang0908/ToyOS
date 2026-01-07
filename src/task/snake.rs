// snake game implementation
use crate::vga_buffer::{WRITER, Color, ColorCode};
use crate::task::time::TickStream;
use crate::task::keyboard; 
use futures_util::stream::StreamExt;
use alloc::collections::vec_deque::VecDeque;
use x86_64::instructions::interrupts;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
const WIDTH: usize = 80;
const HEIGHT: usize = 25;
const PLAY_TOP: usize = 2; 
#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Clone, Copy, PartialEq)]
struct Point {
    x: usize,
    y: usize,
}
struct Snake {
    body: VecDeque<Point>,
    direction: Direction,
}
// main game loop for snake
pub async fn run() {
    
    
    clear_play_area();
    
    let mut rng = Random::new(crate::interrupts::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed));
    
    let mut snake = Snake {
        body: VecDeque::new(),
        direction: Direction::Right,
    };
    snake.body.push_back(Point { x: 10, y: 10 });
    snake.body.push_back(Point { x: 9, y: 10 });
    snake.body.push_back(Point { x: 8, y: 10 });
    let mut food = spawn_food(&snake, &mut rng);
    
    let mut keyboard_decoder = Keyboard::new(ScancodeSet1::new(), layouts::Us104Key, HandleControl::Ignore);
    
    let mut ticker = TickStream::new();
    
    
    draw_border();
    draw_food(food);
    draw_snake(&snake);
    
    let mut game_over = false;
    let mut score = 0;
    
    while let Some(_) = ticker.next().await {
        
        
        
        
        
        
        
        while let Some(scancode) = keyboard::pop_scancode() {
             if let Ok(Some(key_event)) = keyboard_decoder.add_byte(scancode) {
                if let Some(key) = keyboard_decoder.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(c) => match c {
                            'w' => if snake.direction != Direction::Down { snake.direction = Direction::Up; },
                            's' => if snake.direction != Direction::Up { snake.direction = Direction::Down; },
                            'a' => if snake.direction != Direction::Right { snake.direction = Direction::Left; },
                            'd' => if snake.direction != Direction::Left { snake.direction = Direction::Right; },
                            'q' => { 
                                game_over = true;
                            }, 
                            _ => {},
                        },
                        DecodedKey::RawKey(k) => match k {
                             pc_keyboard::KeyCode::ArrowUp => if snake.direction != Direction::Down { snake.direction = Direction::Up; },
                             pc_keyboard::KeyCode::ArrowDown => if snake.direction != Direction::Up { snake.direction = Direction::Down; },
                             pc_keyboard::KeyCode::ArrowLeft => if snake.direction != Direction::Right { snake.direction = Direction::Left; },
                             pc_keyboard::KeyCode::ArrowRight => if snake.direction != Direction::Left { snake.direction = Direction::Right; },
                             _ => {},
                        }
                    }
                }
             }
        }
        
        if game_over { break; }
        
        
        
        
        draw_score(score);
        static mut TICK_ACC: usize = 0;
        unsafe {
            TICK_ACC += 1;
            if TICK_ACC < 2 { continue; } 
            TICK_ACC = 0;
        }
        let head = *snake.body.front().unwrap();
        let new_head = match snake.direction {
            Direction::Up => Point { x: head.x, y: head.y.wrapping_sub(1) },
            Direction::Down => Point { x: head.x, y: head.y + 1 },
            Direction::Left => Point { x: head.x.wrapping_sub(1), y: head.y },
            Direction::Right => Point { x: head.x + 1, y: head.y },
        };
        
        if new_head.x == 0 || new_head.x >= WIDTH || new_head.y < PLAY_TOP || new_head.y >= HEIGHT {
            game_over = true;
            break;
        }
        
        for part in &snake.body {
            if part.x == new_head.x && part.y == new_head.y {
                game_over = true;
                break;
            }
        }
        if game_over { break; }
        snake.body.push_front(new_head);
        
        
        if new_head.x == food.x && new_head.y == food.y {
            score += 10;
            food = spawn_food(&snake, &mut rng);
            draw_food(food);
        } else {
            let tail = snake.body.pop_back().unwrap();
            draw_point(tail, b' '); 
        }
        
        draw_point(new_head, b'O'); 
        draw_point(head, b'o');     
    }
    
    
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let color = ColorCode::new(Color::Red, Color::Black);
        let msg = "GAME OVER";
        let score_msg = alloc::format!("Score: {}", score);
        
        let center_x = (WIDTH - msg.len()) / 2;
        let center_y = HEIGHT / 2;
        
        for (i, b) in msg.bytes().enumerate() {
            writer.write_at(center_y, center_x + i, b, color);
        }
        for (i, b) in score_msg.bytes().enumerate() {
            writer.write_at(center_y + 1, (WIDTH - score_msg.len()) / 2 + i, b, color);
        }
    });
    
    loop {
        if let Some(s) = keyboard::pop_scancode() {
             if s == 0x1C { break; } 
        }
        
        ticker.next().await;
    }
    
    
    clear_play_area();
}
// clears the game play area
fn clear_play_area() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let blank = ColorCode::new(Color::White, Color::Black);
        for row in PLAY_TOP..HEIGHT {
            for col in 0..WIDTH {
                writer.write_at(row, col, b' ', blank);
            }
        }
    });
}
// draws the static border around the play area
fn draw_border() {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let color = ColorCode::new(Color::Blue, Color::Black);
        for col in 0..WIDTH {
            writer.write_at(PLAY_TOP - 1, col, b'#', color);
            writer.write_at(HEIGHT - 1, col, b'#', color);
        }
        for row in PLAY_TOP..HEIGHT {
            writer.write_at(row, 0, b'#', color);
            writer.write_at(row, WIDTH - 1, b'#', color);
        }
    });
}
// draws a single character at a point
fn draw_point(p: Point, c: u8) {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let color = ColorCode::new(Color::Green, Color::Black);
        writer.write_at(p.y, p.x, c, color);
    });
}
// draws the food item
fn draw_food(p: Point) {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        let color = ColorCode::new(Color::Red, Color::Black);
        writer.write_at(p.y, p.x, b'*', color);
    });
}
// generates a new random food position
fn spawn_food(snake: &Snake, rng: &mut Random) -> Point {
    loop {
        let x = (rng.next() as usize % (WIDTH - 2)) + 1;
        let y = (rng.next() as usize % (HEIGHT - PLAY_TOP - 2)) + PLAY_TOP + 1;
        
        let mut collision = false;
        for part in &snake.body {
            if part.x == x && part.y == y {
                collision = true;
                break;
            }
        }
        if !collision {
             return Point { x, y };
        }
    }
}
struct Random {
    state: usize,
}
impl Random {
    fn new(seed: usize) -> Self {
        Random { state: seed }
    }
    
    fn next(&mut self) -> usize {
        
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }
}
fn draw_snake(snake: &Snake) {
    for (i, &p) in snake.body.iter().enumerate() {
        let c = if i == 0 { b'O' } else { b'o' };
        draw_point(p, c);
    }
}
// displays the current score
fn draw_score(score: usize) {
    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        
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
        write!(sw, "Score: {}", score).ok();
        
        
        
        let border_row = PLAY_TOP - 1; 
        
        let start_col = WIDTH - sw.len - 2;
        for (j, &byte) in sw.buf[..sw.len].iter().enumerate() {
            
            writer.write_at(border_row, start_col + j, byte, ColorCode::new(Color::Yellow, Color::Blue));
        }
    });
}
