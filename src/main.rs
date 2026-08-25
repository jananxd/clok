use std::time::Duration;

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style, Stylize},
    text::{Line, ToSpan},
    widgets::{Block, Borders, List, ListState, Paragraph, Wrap},
};

use tui_input::{Input, backend::crossterm::EventHandler};

#[derive(Debug, Default)]
struct InputState {
    input: Input,
}

#[derive(Debug, Default)]
struct SelectedInputState {
    name: InputState,
    description: InputState,
}

#[derive(Debug, Default)]
enum InputElementState {
    #[default]
    Idle,
    Editing,
}

#[derive(Debug, Default)]
struct Model {
    running_state: RunningState,
    tasks: Vec<Task>,
    list_state: ListState,
    app_view: AppView,
    // TODO: create a separate struct to manage the CreateTaskState.
    inputs_state: SelectedInputState,
    create_task_selected_element: i32,
    create_task_input_element_state: InputElementState,
}

impl Model {
    fn is_create_task_editing(&self) -> bool {
        return match self.create_task_input_element_state {
            InputElementState::Editing => true,
            InputElementState::Idle => false,
        };
    }
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
    CreateTaskSelectUpwardElement,
    CreateTaskSelectDownwardElement,
    CreateTaskCancelEditing,
    CreateTaskStartEditing,
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
        Message::CreateTaskSelectDownwardElement => {
            if model.is_create_task_editing() {
                return None;
            }
            model.create_task_selected_element = (model.create_task_selected_element + 1).min(3);
        }
        Message::CreateTaskSelectUpwardElement => {
            if model.is_create_task_editing() {
                return None;
            }
            model.create_task_selected_element = (model.create_task_selected_element - 1).max(0);
        }
        Message::CreateTaskCancelEditing => {
            model.create_task_input_element_state = InputElementState::Idle;
        }
        Message::CreateTaskStartEditing => {
            model.create_task_input_element_state = InputElementState::Editing;
        }
    };
    None
}

fn main() -> color_eyre::Result<()> {
    tui::install_panic_hook();
    let mut terminal = tui::init_terminal()?;
    let mut model = Model::default();
    model.app_view = AppView::ListView;
    model.create_task_selected_element = 0;
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
        let mut current_msg = handle_event(&mut model)?;

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
            let [header_area, name_input_area, description_input_area] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .areas(frame.area());

            let help_message = Line::from_iter(if model.is_create_task_editing() {
                [
                    "Press ".to_span(),
                    "Esc".bold(),
                    " to stop editing, ".to_span(),
                    "Enter".bold(),
                    " to record the message".to_span(),
                ]
            } else {
                [
                    "Press ".to_span(),
                    "q".bold(),
                    " to exit, ".to_span(),
                    "e".bold(),
                    " to start editing.".to_span(),
                ]
            });

            frame.render_widget(help_message, header_area);

            // keep 2 for borders and 1 for cursor
            let width = name_input_area.width.max(3) - 3;
            let scroll = model.inputs_state.name.input.visual_scroll(width as usize);

            let name_style = if model.create_task_selected_element == 0 {
                Color::Red.into()
            } else {
                Style::default()
            };

            let name_input = Paragraph::new(model.inputs_state.name.input.value())
                .style(name_style)
                .scroll((0, scroll as u16))
                .block(Block::bordered().title("Name"));

            frame.render_widget(name_input, name_input_area);

            let description_style = if model.create_task_selected_element == 1 {
                Color::Red.into()
            } else {
                Style::default()
            };

            let description_input = Paragraph::new(model.inputs_state.description.input.value())
                .style(description_style)
                .scroll((0, scroll as u16))
                .block(Block::bordered().title("Description"));

            frame.render_widget(description_input, description_input_area);

            let selected_input: &Input = if model.create_task_selected_element == 0 {
                &model.inputs_state.name.input
            } else {
                &model.inputs_state.description.input
            };

            let selected_area = if model.create_task_selected_element == 0 {
                name_input_area
            } else {
                description_input_area
            };

            if let InputElementState::Editing = model.create_task_input_element_state {
                // Ratatui hides the cursor unless it's explicitly set. Position the  cursor past the
                // end of the input text and one line down from the border to the input line
                let x = selected_input.visual_cursor().max(scroll) - scroll + 1;
                frame.set_cursor_position((selected_area.x + x as u16, selected_area.y + 1))
            }
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

fn handle_event(model: &mut Model) -> color_eyre::Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        let event = event::read()?;
        if let Event::Key(key) = event
            && key.kind == event::KeyEventKind::Press
        {
            return Ok(handle_key(key, model, event));
        }
    }
    Ok(None)
}

fn handle_key(key: event::KeyEvent, model: &mut Model, event: Event) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::Quit),
        _ => match model.app_view {
            AppView::CreateTask => match model.create_task_input_element_state {
                InputElementState::Editing => match key.code {
                    KeyCode::Esc => Some(Message::CreateTaskCancelEditing),
                    _ => {
                        let selected_input: &mut Input = if model.create_task_selected_element == 0
                        {
                            &mut model.inputs_state.name.input
                        } else {
                            &mut model.inputs_state.description.input
                        };
                        selected_input.handle_event(&event);
                        return None;
                    }
                },
                InputElementState::Idle => match key.code {
                    KeyCode::Char('j') => Some(Message::CreateTaskSelectDownwardElement),
                    KeyCode::Char('k') => Some(Message::CreateTaskSelectUpwardElement),
                    KeyCode::Char('e') => Some(Message::CreateTaskStartEditing),
                    _ => None,
                },
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
