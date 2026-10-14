pub mod repository;

use std::collections::HashMap;
use std::time::Duration;

use chrono::{Local, TimeDelta, Utc};

use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::{Line, ToSpan},
    widgets::{Block, Borders, Clear, List, ListState, Paragraph},
};

use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::repository::{NewTask, Repository, StartTimerPayload, Task, TimeEntry};
use tui_input::{Input, backend::crossterm::EventHandler};

const API_URL: &str = "http://localhost:3000";

#[derive(Deserialize)]
struct AuthInitResponseBody {
    url: String,
}

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
    // used to determine whether there's a task that the user wants to delete and currently in progress and waiting for confirmation.
    selected_delete_task_id: Option<i32>,
    repository: Repository,
    active_timers: HashMap<i32, TimeEntry>,
    is_logging_in: bool,
    error_message: String, // whether the API request is still in flight.
}

impl Model {
    fn is_create_task_editing(&self) -> bool {
        return match self.create_task_input_element_state {
            InputElementState::Editing => true,
            InputElementState::Idle => false,
        };
    }

    fn get_in_cursor_task(&self) -> Option<&Task> {
        if let Some(selected_index) = self.list_state.selected() {
            if let Some(selected_task) = self
                .tasks
                .iter()
                .enumerate()
                .find(|&(index, _)| index == selected_index)
            {
                let (_, task) = selected_task;
                return Some(task);
            }
        }

        None
    }
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
    LoginView,
}

#[derive(PartialEq)]
enum Message {
    Increment,
    Decrement,
    Quit,
    OpenCreateTask,
    CancelCreateTask,
    CreateTaskSelectUpwardElement,
    CreateTaskSelectDownwardElement,
    CreateTaskCancelEditing,
    CreateTaskStartEditing,
    CreateTask,
    DeleteTask,
    CancelDeleteTask,
    ConfirmDeleteTask,
    ToggleTimer,
    InitiateLogin,
    CancelLogin,
    CancelErrorMessage,
}

const ONE_HOUR_IN_SECONDS: i64 = 3600;

fn format_duration(delta: &TimeDelta) -> String {
    let total_seconds = delta.num_seconds();

    let hours = total_seconds / ONE_HOUR_IN_SECONDS;
    let minutes = (total_seconds % ONE_HOUR_IN_SECONDS) / 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

async fn update(model: &mut Model, msg: Message) -> Option<Message> {
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
        Message::OpenCreateTask => {
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
        Message::CreateTask => {
            let new_task = NewTask {
                name: model.inputs_state.name.input.to_string(),
                description: Some(model.inputs_state.description.input.to_string()),
            };
            let add_task_result = model.repository.add_task(&new_task);

            match add_task_result {
                Ok(task) => {
                    model.tasks.push(task);
                    model.inputs_state.name.input.reset();
                    model.inputs_state.description.input.reset();
                    model.app_view = AppView::ListView;
                    model.list_state.select_first();
                }
                Err(_) => {}
            }
        }
        Message::DeleteTask => {
            if let Some(selected_index) = model.list_state.selected() {
                if let Some(selected_task) = model
                    .tasks
                    .iter()
                    .enumerate()
                    .find(|&(index, _)| index == selected_index)
                {
                    let (_, task) = selected_task;
                    model.selected_delete_task_id = Some(task.id());
                }
            }
        }
        Message::CancelDeleteTask => {
            model.selected_delete_task_id = None;
        }
        Message::ConfirmDeleteTask => {
            if let Some(task_id) = model.selected_delete_task_id
                && model.repository.delete_task(&task_id).is_ok()
            {
                model.selected_delete_task_id = None;
                model.tasks.retain(|task| task.id() != task_id);
            }
        }
        Message::ToggleTimer => {
            if let Some(task) = model.get_in_cursor_task() {
                match model.active_timers.get(&task.id()) {
                    Some(time_entry) => match model.repository.stop_timer(time_entry.id()) {
                        Ok(_) => {
                            model.active_timers.remove(&time_entry.id());
                        }
                        Err(_) => {
                            // TODO: handle the error later, we should display an error with Clear and ask the user to retry the action.
                        }
                    },
                    None => {
                        let start_timer_payload = StartTimerPayload {
                            start_time: Utc::now(),
                            task_id: task.id(),
                        };
                        match model.repository.start_timer(&start_timer_payload) {
                            Ok(time_entry) => {
                                model.active_timers.insert(task.id(), time_entry);
                            }
                            Err(_) => {
                                // TODO: show an error here, and ask the user to retry it.
                            }
                        }
                    }
                }
            }
        }
        Message::InitiateLogin => {
            model.app_view = AppView::LoginView;
            model.is_logging_in = true;
            let client = reqwest::Client::new();
            match client.post(format!("{}/auth/init", API_URL)).send().await {
                Ok(response) => match response.json::<AuthInitResponseBody>().await {
                    Ok(parsed_response) => {
                        println!("I'm here");
                        if webbrowser::open(parsed_response.url.as_str()).is_ok() {
                            // ...
                        }
                    }
                    Err(error) => {
                        println!("I have an error {}", error);
                    }
                },
                Err(error) => {
                    println!("I have an error {}", error);
                }
            }
        }
        Message::CancelLogin => {
            model.app_view = AppView::ListView;
        }
        Message::CancelErrorMessage => {
            model.error_message = String::from("");
        }
    };
    None
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    tui::install_panic_hook();
    let mut terminal = tui::init_terminal()?;
    let mut model = Model::default();
    model.app_view = AppView::ListView;
    model.create_task_selected_element = 0;
    model
        .repository
        .init()
        .expect("cannot proceed with the application without creating the tables.");
    model.tasks = model
        .repository
        .get_tasks()
        .expect("we don't expect this fail, please take a look at the implementation.");
    model.list_state.select_first();

    for time_entry in model
        .repository
        .get_active_timers(model.tasks.iter().map(|task| task.id()).collect())
        .expect("we don't expect this to fail, especially on start up")
    {
        model.active_timers.insert(time_entry.task_id, time_entry);
    }

    while model.running_state != RunningState::Done {
        // Render the current view
        terminal.draw(|f| view(&mut model, f))?;

        // Handle events and map to a Message
        let mut current_msg = handle_event(&mut model).await?;

        // Process updates as long as they return a non-None message
        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap()).await;
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

            let help_message = if model.is_create_task_editing() {
                Line::from_iter([
                    "Press ".to_span(),
                    "Esc".bold(),
                    " to stop editing, ".to_span(),
                ])
            } else {
                Line::from_iter([
                    "Press ".to_span(),
                    "Esc".bold(),
                    " to cancel create task, ".to_span(),
                    "i".bold(),
                    " to start editing, ".to_span(),
                    "Enter".bold(),
                    " to create the task.".to_span(),
                ])
            };

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
            let task_names: Vec<String> = model
                .tasks
                .iter()
                .map(|task| {
                    if let Some(selected_task) = model.selected_delete_task_id
                        && selected_task == task.id()
                    {
                        return format!(
                            "Please press \"y\" to continue or \"n\" to cancel deletion of: {}",
                            task.name()
                        );
                    }

                    // we wanna check if the curren task in the iteration have an active timer, if yes, then we display a different message.
                    if let Some(time_entry) = model.active_timers.get(&task.id()) {
                        let now = Local::now();
                        let local_start_time = time_entry.start_time.with_timezone(&Local);

                        let current_duration = now - local_start_time;
                        return format!(
                            "▶ {} ⏱ {}",
                            task.name(),
                            format_duration(&current_duration)
                        );
                    }

                    task.name().to_string()
                })
                .collect();

            let b = Block::default()
                .title(Line::from("Press \"q\" to exit").left_aligned())
                .title(Line::from("Clok").centered())
                .title(Line::from("Press \"c\" to create new task").right_aligned())
                // TODO: we want to have a different state, when they press something, we want to render a Clear(), and show another screen
                // saying the instructions that they need like how they do it on Claude Code.
                .title_bottom(Line::from("Press \"l\" to login").left_aligned())
                .borders(Borders::ALL);

            let list = List::new(task_names)
                .block(b)
                .style(Style::new().white())
                .highlight_style(Style::new().red().italic())
                .highlight_symbol("> ")
                .repeat_highlight_symbol(true);

            frame.render_stateful_widget(list, frame.area(), &mut model.list_state);
        }
        AppView::LoginView => {
            let popup_block = Block::bordered().title("Popup");
            let paragraph = Paragraph::new(if model.is_logging_in {
                "Opening the browser..."
            } else {
                "If the browser didn't open, please open this link: to continue with the login"
            })
            .block(popup_block);
            frame.render_widget(paragraph, frame.area());
        }
    }

    if !model.error_message.is_empty() {
        let popup_block = Block::bordered().title("Error!");
        let centered_area = frame
            .area()
            .centered(Constraint::Percentage(60), Constraint::Percentage(60));
        frame.render_widget(Clear, centered_area);
        let paragraph = Paragraph::new(model.error_message.to_string()).block(popup_block);
        frame.render_widget(paragraph, centered_area);
    }
}

async fn handle_event(model: &mut Model) -> color_eyre::Result<Option<Message>> {
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
    if !model.error_message.is_empty() {
        return match key.code {
            KeyCode::Esc => Some(Message::CancelErrorMessage),
            _ => None,
        };
    }

    match key.code {
        KeyCode::Char('q') => Some(Message::Quit),
        _ => match model.app_view {
            AppView::CreateTask => match model.create_task_input_element_state {
                InputElementState::Editing => match key.code {
                    KeyCode::Esc => Some(Message::CreateTaskCancelEditing),
                    _ => {
                        // TODO: this looks ugly, please improve it.
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
                    KeyCode::Char('i') => Some(Message::CreateTaskStartEditing),
                    KeyCode::Enter => Some(Message::CreateTask),
                    KeyCode::Esc => Some(Message::CancelCreateTask),
                    _ => None,
                },
            },
            AppView::ListView => match key.code {
                KeyCode::Char('j') => Some(Message::Increment),
                KeyCode::Char('k') => Some(Message::Decrement),
                KeyCode::Char('c') => Some(Message::OpenCreateTask),
                KeyCode::Char('d') => Some(Message::DeleteTask),
                KeyCode::Char('y') => Some(Message::ConfirmDeleteTask),
                KeyCode::Char('n') => Some(Message::CancelDeleteTask),
                KeyCode::Char('l') => Some(Message::InitiateLogin),
                KeyCode::Char(' ') => Some(Message::ToggleTimer),
                _ => None,
            },
            AppView::LoginView => match key.code {
                KeyCode::Esc => Some(Message::CancelLogin),
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
