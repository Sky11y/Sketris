use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use rand::seq::SliceRandom;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;

use ratatui::{
    DefaultTerminal, Terminal,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Text,
    widgets::{Block, BorderType, Borders, Widget},
};

mod lookup;
use crate::lookup::{I_WALL_KICK_TABLE, OFFSET_TABLE, WALL_KICK_TABLE};

pub fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let mut stdout = std::io::stdout();
        let _ = execute!(stdout, PopKeyboardEnhancementFlags, LeaveAlternateScreen);
        disable_raw_mode().unwrap();
        hook(panic_info);
    }))
}

pub fn init_terminal() -> color_eyre::Result<DefaultTerminal> {
    set_panic_hook();
    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    execute!(
        stdout,
        EnterAlternateScreen,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    Ok(terminal)
}

pub fn restore_terminal() -> color_eyre::Result<()> {
    let mut stdout = std::io::stdout();
    execute!(stdout, PopKeyboardEnhancementFlags, LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = init_terminal()?;
    let app_result = Sketris::new().run(&mut terminal);

    restore_terminal()?;
    app_result
}

#[derive(Debug)]
pub struct Sketris {
    exit: bool,
    play_area_cells: [[bool; 10]; 20],
    bag: Bag,
    current_piece: Piece,
    test_piece: TestPiece,
    next_piece: Option<Piece>,
    gravity_interval: Duration,
    last_gravity: Instant,
    soft_drop: bool,
    hard_drop: bool,
    rotation_pressed: bool,
    horizontal_move: i32,
    lock_down_timer: Duration,
    lock_down_counter: u8,
    last_lock_down: Instant,
    lines: usize,
    points: usize,
    state: State,
}

#[derive(Debug, Clone)]
struct TestPiece {
    origin: Position,
    offsets: &'static [(i32, i32); 4],
    wallkicks: &'static [(i32, i32); 5],
    orientation_idx: u8,
}

#[derive(Debug, Clone, Copy)]
struct Position {
    x: i32,
    y: i32,
}

impl Position {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl TestPiece {
    pub fn new(piece: &Piece) -> Self {
        let origin = piece.origin;
        let offsets = piece.offsets;
        // just a default reference to wallkick table. Changed to correct when rotating a piece.
        let wallkicks = &WALL_KICK_TABLE[0][0];
        let orientation_idx = piece.orientation_idx;
        Self {
            origin,
            offsets,
            wallkicks,
            orientation_idx,
        }
    }

    fn rotate_test_piece(&mut self, shape: Kind, orientation: u8, direction: Dir) {
        let shape_idx = get_shape_idx(shape);
        let mut test_idx: usize = 0;
        if direction == Dir::ClockWise {
            self.orientation_idx = (orientation + 1) % 4;
        } else {
            self.orientation_idx = (orientation + 3) % 4;
            test_idx = 1;
        };

        self.offsets = &OFFSET_TABLE[shape_idx][self.orientation_idx as usize];
        if shape == Kind::I {
            self.wallkicks = &I_WALL_KICK_TABLE[orientation as usize][test_idx];
        } else if shape != Kind::O {
            self.wallkicks = &WALL_KICK_TABLE[orientation as usize][test_idx];
        }
    }
}

#[derive(Debug, Clone)]
struct Piece {
    origin: Position,
    offsets: &'static [(i32, i32); 4],
    shape: Kind,
    color: Color,
    orientation_idx: u8,
}

impl Piece {
    pub fn new(shape: Kind) -> Self {
        let origin = Position::new(4, -1);
        let offsets = get_offset(shape, 0);
        Self {
            origin,
            offsets,
            color: get_piece_color(shape),
            shape,
            orientation_idx: 0,
        }
    }

    pub fn update_origin(&mut self, new_origin: Position) {
        self.origin = new_origin;
    }

    pub fn update_orientation(&mut self, piece: &TestPiece) {
        self.orientation_idx = piece.orientation_idx;
        self.offsets = piece.offsets;
    }
}

#[derive(Debug)]
struct Bag {
    pieces: [Option<Piece>; 7],
    idx: usize,
}

impl Bag {
    fn new() -> Self {
        let mut pieces = std::array::from_fn(|i| Some(Piece::new(get_shape(i as u8))));
        let mut rng = rand::rng();
        pieces.shuffle(&mut rng);
        Self { pieces, idx: 0 }
    }

    fn fill(&mut self) {
        let mut pieces = std::array::from_fn(|i| Some(Piece::new(get_shape(i as u8))));
        let mut rng = rand::rng();
        pieces.shuffle(&mut rng);
        self.pieces = pieces;
    }

    fn piece_owned(&mut self) -> Option<Piece> {
        if let Some(piece) = self.pieces[self.idx].take() {
            self.idx += 1;
            if self.idx > 6 {
                self.fill();
                self.idx = 0;
            }
            Some(piece)
        } else {
            None
        }
    }
}

impl Sketris {
    fn new() -> Self {
        let mut bag = Bag::new();
        let current_piece = bag.piece_owned().unwrap();
        let next_piece = bag.piece_owned();
        let test_piece = TestPiece::new(&current_piece);
        Self {
            exit: false,
            play_area_cells: [[false; 10]; 20],
            bag,
            current_piece,
            next_piece,
            test_piece,
            gravity_interval: Duration::from_millis(500),
            last_gravity: Instant::now(),
            soft_drop: false,
            hard_drop: false,
            rotation_pressed: false,
            horizontal_move: 0,
            lock_down_timer: Duration::from_millis(500),
            lock_down_counter: 0,
            last_lock_down: Instant::now(),
            lines: 0,
            points: 0,
            state: State::Playing,
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let tick_rate = Duration::from_millis(16);
        let mut last_tick = Instant::now();
        while !self.exit {
            self.update(last_tick);
            terminal.draw(|frame| {
                let [header_area, play_area, footer_area] =
                    frame.area().layout(&Layout::vertical([
                        Constraint::Length(1),
                        Constraint::Length(22),
                        Constraint::Length(1),
                    ]));

                frame.render_widget(Text::from("Sketris".bold()).centered(), header_area);
                frame.render_widget(Text::from("<q> Quit | <hjkl> Move").centered(), footer_area);
                frame.render_widget(&self, play_area);
            })?;
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if !event::poll(timeout)? {
                last_tick = Instant::now();
                continue;
            }
            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key)
            };
            last_tick = Instant::now();
        }
        Ok(())
    }

    fn update(&mut self, now: Instant) {
        let mut spawn_new_piece = false;
        if let State::Playing = self.state {
            if self.hard_drop {
                let mut new_origin = self.new_position();
                while self.validate_new_position(&new_origin) {
                    self.current_piece.origin = new_origin;
                    new_origin = self.new_position();
                }
                self.hard_drop = false;
                self.last_gravity = now;
                spawn_new_piece = true;
            } else {
                let gravity_interval = if self.soft_drop {
                    Duration::from_millis(50)
                } else {
                    self.gravity_interval
                };
                if self.lock_down_counter > 0 {
                    let lock_down_timer = now - self.last_lock_down;
                    if lock_down_timer >= self.lock_down_timer || self.lock_down_counter > 15 {
                        spawn_new_piece = true;
                        self.lock_down_counter = 0;
                    }
                }
                if !spawn_new_piece {
                    let elapsed = now - self.last_gravity;
                    if elapsed >= gravity_interval {
                        let new_origin = self.new_position();
                        if self.validate_new_position(&new_origin) {
                            self.current_piece.origin = new_origin;
                            self.lock_down_counter = 0;
                        } else if self.lock_down_counter == 0 {
                            self.lock_down_counter = 1;
                            self.last_lock_down = Instant::now();
                        }
                        self.last_gravity = now;
                    }
                }
            }
            if spawn_new_piece {
                self.update_play_area_cells();
                self.current_piece = self.next_piece.take().unwrap();
                self.next_piece = self.bag.piece_owned();
            }
            self.test_piece = TestPiece::new(&self.current_piece);
        }
    }

    fn update_play_area_cells(&mut self) {
        let offsets = self.current_piece.offsets;
        let origin = self.current_piece.origin;
        for offset in offsets {
            let x = origin.x + offset.0;
            let y = origin.y + offset.1;

            // I'm sceptical if this is the correct way to check the spawning piece. Though seems to work.
            if y < 0 {
                self.state = State::GameOver;
                return;
            }
            self.play_area_cells[y as usize][x as usize] = true;
        }

        let mut removed = Vec::with_capacity(4);
        for y in 0..20 {
            let mut full = true;
            for x in 0..10 {
                if !self.play_area_cells[y][x] {
                    full = false;
                    break;
                }
            }
            if full {
                for x in 0..10 {
                    self.play_area_cells[y][x] = false;
                }
                removed.push(y);
            }
        }

        self.lines += removed.len();
        match removed.len() {
            1 => self.points += 40,
            2 => self.points += 100,
            3 => self.points += 300,
            4 => self.points += 1200,
            _ => {}
        }

        /*
         * TODO:
         *     We need a lookup table for levels. "What level is the player on".
         *     Level up => gravity_interval down
         */

        for line in removed {
            for y in (1..=line).rev() {
                let previous_idx = y - 1;
                self.play_area_cells[y] = self.play_area_cells[previous_idx];
            }
        }
    }

    fn new_position(&self) -> Position {
        Position {
            x: self.current_piece.origin.x,
            y: self.current_piece.origin.y + 1,
        }
    }

    fn validate_new_position(&self, pos: &Position) -> bool {
        let offsets = self.current_piece.offsets;
        for offset in offsets {
            let x = pos.x + offset.0;
            let y = pos.y + offset.1;
            if y < 0 {
                if (0..10).contains(&x) {
                    continue;
                } else {
                    return false;
                }
            }
            if y > 19 || self.play_area_cells[y as usize][x as usize] {
                return false;
            }
        }
        true
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        let mut is_rotating = false;
        match key.kind {
            KeyEventKind::Press => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.exit = true,
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                        self.exit = true
                    }
                    KeyCode::Char('j') | KeyCode::Down => self.soft_drop = true,
                    KeyCode::Char(' ') => self.hard_drop = true,
                    KeyCode::Char('z') if !self.rotation_pressed => {
                        self.test_piece.rotate_test_piece(
                            self.current_piece.shape,
                            self.current_piece.orientation_idx,
                            Dir::CounterClockWise,
                        );
                        self.rotation_pressed = true;
                        is_rotating = true;
                    }
                    KeyCode::Char('x') if !self.rotation_pressed => {
                        self.test_piece.rotate_test_piece(
                            self.current_piece.shape,
                            self.current_piece.orientation_idx,
                            Dir::ClockWise,
                        );
                        self.rotation_pressed = true;
                        is_rotating = true;
                    }
                    KeyCode::Char('l') | KeyCode::Right => self.horizontal_move = 1,
                    KeyCode::Char('h') | KeyCode::Left => self.horizontal_move = -1,
                    // TODO: Just for testing purposes. Remove later!
                    KeyCode::Char('k') | KeyCode::Up => self.test_piece.origin.y += -1,
                    _ => {}
                }
            }
            KeyEventKind::Release => match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    self.soft_drop = false;
                }
                KeyCode::Char('z') => self.rotation_pressed = false,
                KeyCode::Char('x') => self.rotation_pressed = false,
                KeyCode::Char('l') | KeyCode::Right => self.horizontal_move = 0,
                KeyCode::Char('h') | KeyCode::Left => self.horizontal_move = 0,
                _ => {}
            },
            KeyEventKind::Repeat => match key.code {
                // TODO: Just for testing purposes. Remove later!
                KeyCode::Char('k') | KeyCode::Up => self.test_piece.origin.y += -1,
                _ => {}
            },
        }
        self.test_piece.origin.x += self.horizontal_move;
        if let Some(new_origin) = self.validate_test_piece(is_rotating) {
            self.current_piece.update_origin(new_origin);
            self.current_piece.update_orientation(&self.test_piece);
            if self.lock_down_counter > 0 {
                self.lock_down_counter += 1;
                self.last_lock_down = Instant::now();
            }
        }
    }

    fn validate_test_piece(&self, is_rotating: bool) -> Option<Position> {
        // no wall kick tests left/right movement or for O-piece
        if !is_rotating || self.current_piece.shape == Kind::O {
            let pos = self.test_piece.origin;
            for offset in self.test_piece.offsets {
                let test_x = pos.x + offset.0;
                let test_y = pos.y + offset.1;
                // Allow negative y if it is in horizontal bounds
                if test_y < 0 {
                    if (0..10).contains(&test_x) {
                        continue;
                    } else {
                        return None;
                    }
                }
                // test out of bounds
                if !(0..10).contains(&test_x) || test_y > 19 {
                    return None;
                }
                // test static blocks
                if self.play_area_cells[test_y as usize][test_x as usize] {
                    return None;
                }
            }
            return Some(pos);
        }

        let tests = 5;
        for test in 0..tests {
            let mut success = true;
            let x = self.test_piece.origin.x + self.test_piece.wallkicks[test].0;
            let y = self.test_piece.origin.y + self.test_piece.wallkicks[test].1;
            for offset in self.test_piece.offsets {
                let test_x = x + offset.0;
                let test_y = y + offset.1;
                // Allow negative y if it is in horizontal bounds.
                if test_y < 0 {
                    if (0..10).contains(&test_x) {
                        continue;
                    } else {
                        return None;
                    }
                }
                // test out of bounds
                if !(0..10).contains(&test_x) || test_y > 19 {
                    success = false;
                    break;
                }
                // test static blocks
                if self.play_area_cells[test_y as usize][test_x as usize] {
                    success = false;
                    break;
                }
            }
            if success {
                return Some(Position::new(x, y));
            }
        }
        None
    }

    fn find_shadow_piece_origin(&self, mut pos: Position) -> Position {
        loop {
            for offset in self.current_piece.offsets {
                let test_x = pos.x + offset.0;
                let test_y = pos.y + offset.1 + 1;
                if test_y > 19
                    || (test_y > 0 && self.play_area_cells[test_y as usize][test_x as usize])
                {
                    return Position { y: pos.y, x: pos.x };
                }
            }
            pos.y += 1;
        }
    }
}

#[derive(PartialEq)]
pub enum Dir {
    Left,
    Right,
    Down,
    CounterClockWise,
    ClockWise,
}

impl Widget for &Sketris {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [outer_play_area, side_area] = area.layout(
            &Layout::horizontal([Constraint::Length(22), Constraint::Length(12)])
                .flex(Flex::Center),
        );
        let [outer_next_area, empty] = side_area.layout(&Layout::vertical([
            Constraint::Length(6),
            Constraint::Fill(1),
        ]));

        let play_area_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let next_area_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .title("Next");

        let play_area = play_area_block.inner(outer_play_area);
        let next_area = next_area_block.inner(outer_next_area);

        // Draw points and lines
        buf.set_string(
            empty.x + 1,
            empty.y + 1,
            format!("LINES {}", self.lines),
            Style::default().fg(Color::Magenta),
        );
        buf.set_string(
            empty.x + 1,
            empty.y + 2,
            format!("POINTS {}", self.points),
            Style::default().fg(Color::Magenta),
        );
        // draw static pieces
        for y in 0..20 {
            for x in 0..10 {
                if self.play_area_cells[y][x] {
                    buf.set_string(
                        play_area.x + 2 * x as u16,
                        play_area.y + y as u16,
                        "██",
                        Style::default().fg(Color::Gray),
                    );
                }
            }
        }

        if let State::Playing = self.state {
            // draw shadow piece
            let play_area_x = play_area.x as i32;
            let play_area_y = play_area.y as i32;
            let shadow_offsets = self.current_piece.offsets;
            let mut shadow_origin = self.current_piece.origin;
            shadow_origin = self.find_shadow_piece_origin(shadow_origin);
            for offset in shadow_offsets.iter() {
                let x = play_area_x + 2 * (shadow_origin.x + offset.0);
                let y = play_area_y + shadow_origin.y + offset.1;
                buf.set_string(
                    x as u16,
                    y as u16,
                    "[]",
                    Style::default().fg(self.current_piece.color),
                );
            }

            // draw falling piece
            let current_piece = &self.current_piece;
            let origin = current_piece.origin;
            for offset in current_piece.offsets.iter() {
                let x = play_area_x + 2 * (origin.x + offset.0);
                let y = play_area_y + origin.y + offset.1;
                // don't draw if above the area
                if y < play_area_y {
                    continue;
                }
                buf.set_string(
                    x as u16,
                    y as u16,
                    "██",
                    Style::default().fg(current_piece.color),
                );
            }
        }

        // draw next piece
        // Never panics: There's always Some in self.next_piece.
        let next_piece = self.next_piece.as_ref().unwrap();
        for offset in next_piece.offsets.iter() {
            let x = 2 + offset.0;
            let y = 2 + offset.1;
            // No need to check the borders. Piece not moving.
            buf.set_string(
                next_area.x + 2 * x as u16,
                next_area.y + y as u16,
                "██",
                Style::default().fg(next_piece.color),
            );
        }
        play_area_block.render(outer_play_area, buf);
        next_area_block.render(outer_next_area, buf);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    I,
    L,
    J,
    S,
    Z,
    O,
    T,
}

#[derive(Debug)]
pub enum State {
    GameOver,
    Playing,
}

fn get_offset(shape: Kind, orientation: u8) -> &'static [(i32, i32); 4] {
    let shape_idx = get_shape_idx(shape);

    &OFFSET_TABLE[shape_idx][orientation as usize]
}

fn get_shape_idx(shape: Kind) -> usize {
    match shape {
        Kind::I => 0,
        Kind::L => 1,
        Kind::J => 2,
        Kind::S => 3,
        Kind::Z => 4,
        Kind::O => 5,
        Kind::T => 6,
    }
}

fn get_shape(idx: u8) -> Kind {
    match idx {
        0 => Kind::I,
        1 => Kind::L,
        2 => Kind::J,
        3 => Kind::S,
        4 => Kind::Z,
        5 => Kind::O,
        6 => Kind::T,
        _ => unreachable!(),
    }
}

fn get_piece_color(piece: Kind) -> Color {
    match piece {
        Kind::I => Color::LightBlue,
        Kind::L => Color::Rgb(255, 165, 0),
        Kind::J => Color::Blue,
        Kind::S => Color::Green,
        Kind::Z => Color::Red,
        Kind::O => Color::Yellow,
        Kind::T => Color::Magenta,
    }
}
