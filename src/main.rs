use std::time::Duration;

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
    layout::Alignment,
    style::Style,
    text::Line,
    widgets::{Block, Borders, List, ListState, Paragraph, Wrap},
};

#[derive(Debug, Default)]
struct Model {
    running_state: RunningState,
    tasks: Vec<Task>,
    list_state: ListState,
    app_view: AppView,
}

#[derive(Debug, Clone)]
struct Task {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Debug, Default)]
enum AppView {
    #[default]
    ListView,
    CreateTask,
}

#[derive(PartialEq)]
enum Message {
    Increment,
    Decrement,
    Quit,
    CreateTask,
    CancelCreateTask,
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Increment => {
            model.list_state.select_next();
        }
        Message::Decrement => {
            model.list_state.select_previous();
        }
        Message::Quit => {
            // You can handle cleanup and exit here
            model.running_state = RunningState::Done;
        }
        Message::CreateTask => {
            model.app_view = AppView::CreateTask;
        }
        Message::CancelCreateTask => {
            model.app_view = AppView::ListView;
        }
    };
    None
}

fn main() -> color_eyre::Result<()> {
    tui::install_panic_hook();
    let mut terminal = tui::init_terminal()?;
    let mut model = Model::default();
    model.app_view = AppView::ListView;
    model.tasks.push(Task {
        name: "Task 1".to_string(),
        description: Some("description".to_string()),
    });

    model.tasks.push(Task {
        name: "Task 2".to_string(),
        description: Some("description".to_string()),
    });

    model.tasks.push(Task {
        name: "Task 3".to_string(),
        description: Some("description".to_string()),
    });

    model.list_state.select_first();

    while model.running_state != RunningState::Done {
        // Render the current view
        terminal.draw(|f| view(&mut model, f))?;

        // Handle events and map to a Message
        let mut current_msg = handle_event(&model)?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap());
        }
    }

    tui::restore_terminal()?;
    Ok(())
}

fn view(model: &mut Model, frame: &mut Frame) {
    match model.app_view {
        AppView::CreateTask => {
            let p = Paragraph::new("Hello!")
                .block(Block::bordered().title("Paragraph"))
                .style(Style::new().white().on_black())
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });

            frame.render_widget(p, frame.area());
        }
        AppView::ListView => {
            let task_names: Vec<&str> = model.tasks.iter().map(|task| task.name.as_str()).collect();

            let b = Block::default()
                .title(Line::from("Press \"q\" to exit").left_aligned())
                .title(Line::from("Clok").centered())
                .title(Line::from("Press \"c\" to create new task").right_aligned())
                .borders(Borders::ALL);

            let list = List::new(task_names)
                .block(b)
                .style(Style::new().white())
                .highlight_style(Style::new().red().italic())
                .highlight_symbol("> ")
                .repeat_highlight_symbol(true);

            frame.render_stateful_widget(list, frame.area(), &mut model.list_state);
        }
    }
}

fn handle_event(model: &Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))?
        && let Event::Key(key) = event::read()?
        && key.kind == event::KeyEventKind::Press
    {
        return Ok(handle_key(key, model));
    }
    Ok(None)
}

fn handle_key(key: event::KeyEvent, model: &Model) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::Quit),
        _ => match model.app_view {
            AppView::CreateTask => match key.code {
                KeyCode::Esc => Some(Message::CancelCreateTask),
                _ => None,
            },
            AppView::ListView => match key.code {
                KeyCode::Char('j') => Some(Message::Increment),
                KeyCode::Char('k') => Some(Message::Decrement),
                KeyCode::Char('c') => Some(Message::CreateTask),
                _ => None,
            },
        },
    }
}

mod tui {
    use ratatui::{
        Terminal,
        backend::{Backend, CrosstermBackend},
        crossterm::{
            ExecutableCommand,
            terminal::{
                EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
            },
        },
    };
    use std::{
        io::{self, stdout},
        panic,
    };

    pub fn init_terminal() -> color_eyre::Result<Terminal<impl Backend<Error = io::Error>>> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        Ok(terminal)
    }

    pub fn restore_terminal() -> color_eyre::Result<()> {
        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    pub fn install_panic_hook() {
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            stdout().execute(LeaveAlternateScreen).unwrap();
            disable_raw_mode().unwrap();
            original_hook(panic_info);
        }));
    }
}
