use std::{
    collections::VecDeque,
    io::{stdout, Write},
    time::{Duration, Instant},
};

use futures::{future::FutureExt, select, StreamExt};
use futures_timer::Delay;

use crossterm::{
    cursor,
    event::{Event, EventStream, KeyCode},
    execute,
    queue,
    terminal
};

enum Direction {
    Up, Right, Down, Left
}

impl Direction {
    pub fn get_delta(&self) -> (i16, i16) {
        match self {
            Direction::Up => (0, -1),
            Direction::Right => (1, 0),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0)
        }
    }
}

struct Point {
    x: u16,
    y: u16
}

#[derive(Debug, Clone)]
struct OutOfBounds;

struct Snake {
    body: VecDeque<Point>,
    direction: Direction
}

impl Snake {
    pub fn new(room_w: u16, room_h: u16) -> Snake {
        let room_mid = room_w / 2;

        let mut body = VecDeque::new();
        body.push_front(Point{ x: room_mid, y: room_h - 2 });
        body.push_front(Point{ x: room_mid, y: room_h - 3 });
        body.push_front(Point{ x: room_mid, y: room_h - 4 });

        let direction = Direction::Up;

        Snake {
            body,
            direction
        }
    }

    pub fn turn(&mut self, dir: Direction) {
        self.direction = dir;
    }

    pub fn draw(&self) {
        let mut stdout = stdout();
        for point in self.body.iter() {
            let Point{x, y} = point;
            queue!(stdout, cursor::MoveTo(*x, *y)).unwrap();
            print!("x");
            stdout.flush().unwrap();
        }
    }

    pub fn step(&mut self) -> Result<(), OutOfBounds> {
        match self.get_next_point() {
            Some(point) => {
                let _removed_point = self.body.pop_back().unwrap();
                self.body.push_front(point);
                Ok(())
            },
            None => Err(OutOfBounds)
        }
    }

    fn get_next_point(&self) -> Option<Point> {
        let head = self.body.front().unwrap();
        let (dx, dy) = self.direction.get_delta();

        if head.x == 0 && dx < 0 { return None }
        if head.y == 0 && dy < 0 { return None }

        return Some(Point {
            x: (head.x as i16 + dx) as u16,
            y: (head.y as i16 + dy) as u16
        })
    }
}

async fn game_loop() {
    let mut reader = EventStream::new();

    let (w, h) = terminal::size().unwrap();

    let mut snake = Snake::new(w, h);

    let target_delay = 1_000;
    let mut planned_delay: u64 = target_delay;

    loop {
        let mut delay = Delay::new(Duration::from_millis(planned_delay)).fuse();
        let mut event = reader.next().fuse();
        let cycle_started_at = Instant::now();

        select! {
            _ = delay => {
                let mut stdout = stdout();
                queue!(stdout, terminal::Clear(terminal::ClearType::All)).unwrap();
                match snake.step() {
                    Err(_) => { break; },
                    _ => {}
                };
                snake.draw();
                planned_delay = target_delay;
            },
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {
                        match event {
                            Event::Key(key) => match key.code {
                                KeyCode::Esc => break,
                                KeyCode::Char(char) => match char {
                                    'u' => snake.turn(Direction::Up),
                                    'n' => snake.turn(Direction::Left),
                                    'i' => snake.turn(Direction::Right),
                                    'e' => snake.turn(Direction::Down),
                                    _ => {}
                                },
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    Some(Err(e)) => println!("Error: {:?}\r", e),
                    None => break,
                }
                let event_happened_at = Instant::now();
                let time_passed = event_happened_at - cycle_started_at;
                planned_delay = target_delay - time_passed.as_millis() as u64;
            }
        };
    }
}

fn main() -> crossterm::Result<()> {
    terminal::enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, terminal::Clear(terminal::ClearType::All))?;
    execute!(stdout, cursor::Hide)?;

    async_std::task::block_on(game_loop());

    execute!(stdout, cursor::Show)?;

    terminal::disable_raw_mode()
}
