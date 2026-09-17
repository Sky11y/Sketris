# Sketris

A Tetris for the terminal. Written in Rust.

## About the project

This project is for me (Sky11y) to learn and better understand Rust. Every line in this project is written by me. I have been using AI only for ideation and in cases where I was stuck with the language and rust error messages didn't help me forward.

### Tools

- [Ratatui](https://docs.rs/ratatui/): TUI
- [Crossterm](https://docs.rs/crossterm/): Input
- [Color_eyre](https://docs.rs/color-eyre/): Error handling

### Restrictions

For following the keyboard state unambiguously this project uses [kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/). Follow the link to to learn more and see if your terminal supports the protocol.

While the game is running no signals are sent to the terminal. **Note**. Though not a signal ctrl+c will exit the game.

This game has currently only been tested on Fedora 44 and alacritty, so no guarantees for it to work smoothly (or at all) on your computer.

### Features

- SRS (Super Rotation System) for wallkicks
- Soft drop
- Hard drop
- Shadow piece
- Lock down 0.5 s with move reset (15 moves/rotations)
- Next piece preview
- One bag filled and randomly shuffled when empty
- Points:
    - 1 line    => 40 x level
    - 2 lines   => 100 x level
    - 3 lines   => 300 x level
    - Tetris    => 1200 x level

**Note**. Levels will be implemented soon.

### Controls


| Key | Action |
| --- | ---- |
| Left | Move piece left |
| Right | Move piece right |
| Z | Rotate counterclockwise |
| X | Rotate clockwise |
| Down | Soft drop |
| Space | Hard drop |
| Escape | Quit |
| Q | Quit |
| Ctrl-c | Quit |

**Note**. While game is running no signals are sent forward.

## Getting started

### Dependencies

- Should support all the usual operating systems: Linux, macOS and Windows.
- A terminal that supports *kitty keyboard protocol* (see [[#Restrictions]])
- Rust compiler (and preferably Cargo)

### Running the game

- Clone this repository to your computer.
- Cd to the repository
- Execute `cargo run --release` to start the game immediately (after build)

**Note**. You could also build the program with *rustc* but you have to build and link the external crates (rand, ratatui, crossterm, color-eyre) manually. Do yourself a favor and build it with Cargo.
**Note**. The binary will be found in the *repository-root/target/release/* directory under name ***sketris***.

## Roadmap

- v. 0.1
    - [ ] Levels
    - [ ] Clean up the code and separate to modules
    - [ ] Game menu
    - [ ] Better Game over state
    - [ ] Local leaderboard
    - [ ] Make horizontal movement smoother
    - [ ] Better error handling and last clean up

### Optional improvements

- [ ] Add extra points for clearing the whole area
- [ ] I-piece wall kicks might be broken
    - I'm not an expert in Tetris but I think it is not working properly in small caves.
- [ ] Render the layout only once (should be possible with ratatui)
    - Layout (boxes and text) is static, why render it on every frame?
- [ ] Render one line at a time instead of per block/cell
    - Gaming area is small and there's so much to render that this hardly is a bottleneck.

## Authors

[Sky11y](https://github.com/Sky11y/)

## License

- MIT


