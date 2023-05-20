#![allow(unused_imports)]
use std::{io::{self, Write}, thread, time::{Duration, Instant}, sync::mpsc, error::Error};
use std::ops::Not;
use std::thread::current;
use tui::{
    style::{Style, Color, Modifier},
    backend::CrosstermBackend,
    widgets::{List, ListItem,  Block, Borders, Paragraph, BorderType},
    layout::{Layout, Constraint, Direction, Alignment, self, Rect}, 
    Terminal,
    text::Text
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode,  KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::widgets::ListState;

enum MyEvent<I> {
    Input(I),
    Tick,
}

// enum MenuItem {
//     Split,
//     SetCounterAndTimer,
// }
//
// impl From<MenuItem> for usize {
//     fn from(input: MenuItem) -> usize {
//         match input {
//             MenuItem::Split => 0,
//             MenuItem::SetCounterAndTimer => 1,
//         }
//     }
// }

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // set up a multiproducer, single consumer (mpsc) channel to communicate between input handler
    // and the rendering loop
    // spawn new thread, so input polling doesn't block the main thread for rendering
    let (tx, rx) = mpsc::channel();
    let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(timeout).expect("poll works") {
                if let Event::Key(key) = event::read().expect("can read events") {
                    tx.send(MyEvent::Input(key)).expect("can send events")
                }
            }

            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(MyEvent::Tick) {
                    last_tick = Instant::now();
                }
            }
        }
    });

    let mut split_list_state = ListState::default();
    split_list_state.select(Some(0));
    let mut counter: u32 = 0;
    let mut now = Instant::now();
    let mut paused = true;

    loop {
        terminal.draw(|f| {

            // configuring the ui
            let size = f.size();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
                .split(size);

            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(25),
                        Constraint::Percentage(25),
                        Constraint::Min(0)
                    ].as_ref()
                )
                .split(chunks[1]);

            let sub_body_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(50),
                        Constraint::Min(0)
                    ].as_ref()
                )
                .split(body_chunks[2]);

            // creating widgets for rendering
            let title = Paragraph::new("Workout Tool Version 0.1.0")
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Plain),
                );

            let counter_block = Paragraph::new(counter.to_string())
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title("Set Counter")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Plain),
                );

            let timer_block = Paragraph::new(now.elapsed().as_secs().to_string())
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title("Timer")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Plain),
                );

            let exercises_in_split: [&str; 4] = [
                "Chin-Ups\nRows\nBicep Curls\nHanging",
                "Dips\nPush-Ups\nLateral Raises",
                "Squats\nNordic Curls\nCalf Raises\nLeg Raises",
                "Romanian Deadlifts\nExternal Rotation\nResting Deep-Squat",
            ];
            let exercise_list =
                Paragraph::new(exercises_in_split[split_list_state.selected().unwrap()])
                    .block(Block::default().title("Exercises").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White));

            let items = [
                ListItem::new("Pull"),
                ListItem::new("Push"),
                ListItem::new("Legs"),
                ListItem::new("Accs")
            ];
            let split_list = List::new(items)
                .block(Block::default().title("Split").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                );

            // render
            f.render_widget(title, chunks[0]);
            f.render_stateful_widget(split_list, body_chunks[0], &mut split_list_state);
            f.render_widget(exercise_list, body_chunks[1]);
            f.render_widget(counter_block, sub_body_chunks[0]);
            f.render_widget(timer_block, sub_body_chunks[1]);
        })?;

        // do some action after input
        match rx.recv()? {
            MyEvent::Input(event) => match event.code {
                KeyCode::Esc => {
                    // restore terminal
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;

                    // break loop
                    break;
                }
                KeyCode::Up => {
                    if let Some(current_selection) = split_list_state.selected() {
                        // wrap around if current item is 0 or somehow under
                        if current_selection <= 0 {
                            split_list_state.select(Some(3));
                        } else {
                            split_list_state.select(Some(current_selection - 1));
                        }
                    }
                }
                KeyCode::Down => {
                    if let Some(current_selection) = split_list_state.selected() {
                        // wrap around if item at end of list
                        if current_selection >= 3 {
                            split_list_state.select(Some(0));
                        } else {
                            split_list_state.select(Some(current_selection + 1));
                        }
                    }
                }
                KeyCode::Char('w') => {
                    counter += 1;
                }
                KeyCode::Char('s') => {
                    if counter > 0 {
                        counter -= 1;
                    }
                }
                KeyCode::Char(' ') => {
                    paused = paused.not();
                    now = Instant::now();
                }
                _ => {}
            }
            MyEvent::Tick => {
                if paused {
                    now = Instant::now();
                }
            }
        }
    }

    // let thread_join_handle = thread::Builder::new().name("timer".to_string()).spawn( || {
    //     let now = Instant::now();
    //     let mut elapsed_time = now.elapsed().as_millis();
    //     while elapsed_time < 5000 {
    //         elapsed_time = now.elapsed().as_millis();
    //         println!("{:0>2}:{:0>3}", elapsed_time/1000, elapsed_time%1000);
    //     }
    // });

    // _ = thread_join_handle.unwrap().join();

    Ok(())
}