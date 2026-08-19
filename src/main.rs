use crossterm::event::{self, KeyCode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListDirection, ListState};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let mut list_state = ListState::default().with_selected(Some(0));
    ratatui::run(|terminal| {
        loop {
            terminal.draw(|frame| render(frame, &mut list_state))?;
            if let Some(key) = event::read()?.as_key_press_event() {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => list_state.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => list_state.select_previous(),
                    KeyCode::Char('c') => println!("pressing C!!"),
                    KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                    _ => {}
                }
            }
        }
    })
}

/// Render the UI with various lists.
fn render(frame: &mut Frame, list_state: &mut ListState) {
    let constraints = [
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ];
    let layout = Layout::vertical(constraints).spacing(1);
    let [top, first, _] = frame.area().layout(&layout);

    let title = Line::from_iter([
        Span::from("Clok").bold(),
        Span::from(" (Press 'q' to quit and arrow keys to navigate)"),
    ]);

    frame.render_widget(title.centered(), top);

    render_list(frame, first, list_state);
}

/// Render a list.
pub fn render_list(frame: &mut Frame, area: Rect, list_state: &mut ListState) {
    let items = ["Item 1", "Item 2", "Item 3", "Item 4"];
    let list = List::new(items)
        .style(Color::White)
        .highlight_style(Modifier::REVERSED)
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, list_state);
}
