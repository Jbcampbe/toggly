use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use uuid::Uuid;

use toggly_core::models::*;

use crate::client::ApiClient;
use crate::ui;

// ---------------------------------------------------------------------------
// Input mode for text input dialogs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    /// Editing a text input field; (field_label, current_buffer)
    Editing,
}

// ---------------------------------------------------------------------------
// Application screens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Screen {
    ProjectList,
    ProjectDetail {
        project: Project,
    },
    FlagDetail {
        project: Project,
        flag_with_states: FlagWithStates,
    },
}

// ---------------------------------------------------------------------------
// Dialog types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Dialog {
    CreateProject {
        name: String,
    },
    CreateEnvironment {
        project_id: Uuid,
        name: String,
    },
    CreateFlag {
        project_id: Uuid,
        key: String,
        name: String,
        /// 0 = key field, 1 = name field
        active_field: usize,
    },
    ConfirmDelete {
        message: String,
        action: DeleteAction,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone)]
pub enum DeleteAction {
    Project(Uuid),
    Environment { project_id: Uuid, env_id: Uuid },
    Flag { project_id: Uuid, key: String },
}

// ---------------------------------------------------------------------------
// Focus pane on ProjectDetail screen
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Environments,
    Flags,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

pub struct App {
    pub screen: Screen,
    pub dialog: Option<Dialog>,
    pub input_mode: InputMode,
    pub should_quit: bool,
    pub status_message: String,

    // Project list
    pub projects: Vec<Project>,
    pub project_index: usize,

    // Project detail
    pub environments: Vec<Environment>,
    pub env_index: usize,
    pub flags: Vec<Flag>,
    pub flag_index: usize,
    pub active_pane: Pane,

    // Flag detail
    pub flag_env_index: usize,

    pub client: ApiClient,
}

impl App {
    pub fn new(client: ApiClient) -> Self {
        Self {
            screen: Screen::ProjectList,
            dialog: None,
            input_mode: InputMode::Normal,
            should_quit: false,
            status_message: String::new(),
            projects: Vec::new(),
            project_index: 0,
            environments: Vec::new(),
            env_index: 0,
            flags: Vec::new(),
            flag_index: 0,
            active_pane: Pane::Flags,
            flag_env_index: 0,
            client,
        }
    }

    pub fn load_projects(&mut self) {
        match self.client.list_projects() {
            Ok(projects) => {
                self.projects = projects;
                if self.project_index >= self.projects.len() && !self.projects.is_empty() {
                    self.project_index = self.projects.len() - 1;
                }
            }
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }

    pub fn load_project_detail(&mut self, project: &Project) {
        match self.client.list_environments(project.id) {
            Ok(envs) => self.environments = envs,
            Err(e) => self.status_message = format!("Error loading environments: {e}"),
        }
        match self.client.list_flags(project.id) {
            Ok(flags) => self.flags = flags,
            Err(e) => self.status_message = format!("Error loading flags: {e}"),
        }
        self.env_index = 0;
        self.flag_index = 0;
    }

    pub fn load_flag_detail(&mut self, project_id: Uuid, key: &str) {
        match self.client.get_flag(project_id, key) {
            Ok(fws) => {
                self.screen = Screen::FlagDetail {
                    project: match &self.screen {
                        Screen::ProjectDetail { project } => project.clone(),
                        Screen::FlagDetail { project, .. } => project.clone(),
                        _ => return,
                    },
                    flag_with_states: fws,
                };
                self.flag_env_index = 0;
            }
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Main run loop
// ---------------------------------------------------------------------------

pub fn run(client: ApiClient) -> anyhow::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(client);
    app.load_projects();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_input(&mut app, key.code);
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Input handling
// ---------------------------------------------------------------------------

fn handle_input(app: &mut App, key: KeyCode) {
    // If a dialog is open, route input there
    if app.dialog.is_some() {
        handle_dialog_input(app, key);
        return;
    }

    match &app.screen.clone() {
        Screen::ProjectList => handle_project_list_input(app, key),
        Screen::ProjectDetail { project } => {
            let project = project.clone();
            handle_project_detail_input(app, key, &project);
        }
        Screen::FlagDetail {
            project,
            flag_with_states,
        } => {
            let project = project.clone();
            let fws = flag_with_states.clone();
            handle_flag_detail_input(app, key, &project, &fws);
        }
    }
}

fn handle_project_list_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.projects.is_empty() {
                app.project_index = (app.project_index + 1).min(app.projects.len() - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.project_index > 0 {
                app.project_index -= 1;
            }
        }
        KeyCode::Enter => {
            if let Some(project) = app.projects.get(app.project_index).cloned() {
                app.load_project_detail(&project);
                app.screen = Screen::ProjectDetail { project };
            }
        }
        KeyCode::Char('c') => {
            app.dialog = Some(Dialog::CreateProject {
                name: String::new(),
            });
            app.input_mode = InputMode::Editing;
        }
        KeyCode::Char('d') => {
            if let Some(project) = app.projects.get(app.project_index).cloned() {
                app.dialog = Some(Dialog::ConfirmDelete {
                    message: format!("Delete project '{}'?", project.name),
                    action: DeleteAction::Project(project.id),
                });
            }
        }
        KeyCode::Char('r') => app.load_projects(),
        _ => {}
    }
}

fn handle_project_detail_input(app: &mut App, key: KeyCode, project: &Project) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::ProjectList;
            app.load_projects();
        }
        KeyCode::Tab => {
            app.active_pane = match app.active_pane {
                Pane::Environments => Pane::Flags,
                Pane::Flags => Pane::Environments,
            };
        }
        KeyCode::Char('j') | KeyCode::Down => match app.active_pane {
            Pane::Environments => {
                if !app.environments.is_empty() {
                    app.env_index = (app.env_index + 1).min(app.environments.len() - 1);
                }
            }
            Pane::Flags => {
                if !app.flags.is_empty() {
                    app.flag_index = (app.flag_index + 1).min(app.flags.len() - 1);
                }
            }
        },
        KeyCode::Char('k') | KeyCode::Up => match app.active_pane {
            Pane::Environments => {
                if app.env_index > 0 {
                    app.env_index -= 1;
                }
            }
            Pane::Flags => {
                if app.flag_index > 0 {
                    app.flag_index -= 1;
                }
            }
        },
        KeyCode::Enter => {
            if app.active_pane == Pane::Flags {
                if let Some(flag) = app.flags.get(app.flag_index).cloned() {
                    app.load_flag_detail(project.id, &flag.key);
                }
            }
        }
        KeyCode::Char('c') => match app.active_pane {
            Pane::Environments => {
                app.dialog = Some(Dialog::CreateEnvironment {
                    project_id: project.id,
                    name: String::new(),
                });
                app.input_mode = InputMode::Editing;
            }
            Pane::Flags => {
                app.dialog = Some(Dialog::CreateFlag {
                    project_id: project.id,
                    key: String::new(),
                    name: String::new(),
                    active_field: 0,
                });
                app.input_mode = InputMode::Editing;
            }
        },
        KeyCode::Char('d') => match app.active_pane {
            Pane::Environments => {
                if let Some(env) = app.environments.get(app.env_index).cloned() {
                    app.dialog = Some(Dialog::ConfirmDelete {
                        message: format!("Delete environment '{}'?", env.name),
                        action: DeleteAction::Environment {
                            project_id: project.id,
                            env_id: env.id,
                        },
                    });
                }
            }
            Pane::Flags => {
                if let Some(flag) = app.flags.get(app.flag_index).cloned() {
                    app.dialog = Some(Dialog::ConfirmDelete {
                        message: format!("Delete flag '{}'?", flag.key),
                        action: DeleteAction::Flag {
                            project_id: project.id,
                            key: flag.key.clone(),
                        },
                    });
                }
            }
        },
        KeyCode::Char('r') => app.load_project_detail(project),
        _ => {}
    }
}

fn handle_flag_detail_input(
    app: &mut App,
    key: KeyCode,
    project: &Project,
    fws: &FlagWithStates,
) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.load_project_detail(project);
            app.screen = Screen::ProjectDetail {
                project: project.clone(),
            };
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !fws.environments.is_empty() {
                app.flag_env_index =
                    (app.flag_env_index + 1).min(fws.environments.len() - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.flag_env_index > 0 {
                app.flag_env_index -= 1;
            }
        }
        KeyCode::Char('t') => {
            // Toggle the selected environment state
            if let Some(state) = fws.environments.get(app.flag_env_index) {
                let req = UpdateFlagStateRequest {
                    enabled: Some(!state.enabled),
                    default_value: None,
                    off_value: None,
                };
                match app.client.update_flag_state(
                    project.id,
                    &fws.flag.key,
                    state.environment_id,
                    &req,
                ) {
                    Ok(_) => {
                        app.status_message = format!(
                            "Flag '{}' {} in {}",
                            fws.flag.key,
                            if !state.enabled { "enabled" } else { "disabled" },
                            state.environment_name,
                        );
                        app.load_flag_detail(project.id, &fws.flag.key);
                    }
                    Err(e) => app.status_message = format!("Error: {e}"),
                }
            }
        }
        KeyCode::Char('r') => {
            app.load_flag_detail(project.id, &fws.flag.key);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Dialog input
// ---------------------------------------------------------------------------

fn handle_dialog_input(app: &mut App, key: KeyCode) {
    let dialog = app.dialog.clone().unwrap();

    match dialog {
        Dialog::CreateProject { mut name } => match key {
            KeyCode::Esc => {
                app.dialog = None;
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                if !name.trim().is_empty() {
                    let req = CreateProjectRequest {
                        name: name.trim().to_string(),
                        description: String::new(),
                    };
                    match app.client.create_project(&req) {
                        Ok(_) => {
                            app.status_message = format!("Project '{}' created", req.name);
                            app.load_projects();
                        }
                        Err(e) => app.status_message = format!("Error: {e}"),
                    }
                }
                app.dialog = None;
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                name.pop();
                app.dialog = Some(Dialog::CreateProject { name });
            }
            KeyCode::Char(c) => {
                name.push(c);
                app.dialog = Some(Dialog::CreateProject { name });
            }
            _ => {}
        },

        Dialog::CreateEnvironment {
            project_id,
            mut name,
        } => match key {
            KeyCode::Esc => {
                app.dialog = None;
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => {
                if !name.trim().is_empty() {
                    let req = CreateEnvironmentRequest {
                        name: name.trim().to_string(),
                    };
                    match app.client.create_environment(project_id, &req) {
                        Ok(_) => {
                            app.status_message = format!("Environment '{}' created", req.name);
                            if let Screen::ProjectDetail { project } = app.screen.clone() {
                                app.load_project_detail(&project);
                            }
                        }
                        Err(e) => app.status_message = format!("Error: {e}"),
                    }
                }
                app.dialog = None;
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                name.pop();
                app.dialog = Some(Dialog::CreateEnvironment { project_id, name });
            }
            KeyCode::Char(c) => {
                name.push(c);
                app.dialog = Some(Dialog::CreateEnvironment { project_id, name });
            }
            _ => {}
        },

        Dialog::CreateFlag {
            project_id,
            key: mut flag_key,
            mut name,
            active_field,
        } => match key {
            KeyCode::Esc => {
                app.dialog = None;
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Tab => {
                app.dialog = Some(Dialog::CreateFlag {
                    project_id,
                    key: flag_key,
                    name,
                    active_field: if active_field == 0 { 1 } else { 0 },
                });
            }
            KeyCode::Enter => {
                if !flag_key.trim().is_empty() && !name.trim().is_empty() {
                    let req = CreateFlagRequest {
                        key: flag_key.trim().to_string(),
                        name: name.trim().to_string(),
                        description: String::new(),
                        flag_type: FlagType::Boolean,
                        tags: Vec::new(),
                    };
                    match app.client.create_flag(project_id, &req) {
                        Ok(_) => {
                            app.status_message = format!("Flag '{}' created", req.key);
                            if let Screen::ProjectDetail { project } = app.screen.clone() {
                                app.load_project_detail(&project);
                            }
                        }
                        Err(e) => app.status_message = format!("Error: {e}"),
                    }
                }
                app.dialog = None;
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                if active_field == 0 {
                    flag_key.pop();
                } else {
                    name.pop();
                }
                app.dialog = Some(Dialog::CreateFlag {
                    project_id,
                    key: flag_key,
                    name,
                    active_field,
                });
            }
            KeyCode::Char(c) => {
                if active_field == 0 {
                    flag_key.push(c);
                } else {
                    name.push(c);
                }
                app.dialog = Some(Dialog::CreateFlag {
                    project_id,
                    key: flag_key,
                    name,
                    active_field,
                });
            }
            _ => {}
        },

        Dialog::ConfirmDelete { message: _, action } => match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                match &action {
                    DeleteAction::Project(id) => match app.client.delete_project(*id) {
                        Ok(_) => {
                            app.status_message = "Project deleted".into();
                            app.load_projects();
                        }
                        Err(e) => app.status_message = format!("Error: {e}"),
                    },
                    DeleteAction::Environment {
                        project_id,
                        env_id,
                    } => match app.client.delete_environment(*project_id, *env_id) {
                        Ok(_) => {
                            app.status_message = "Environment deleted".into();
                            if let Screen::ProjectDetail { project } = app.screen.clone() {
                                app.load_project_detail(&project);
                            }
                        }
                        Err(e) => app.status_message = format!("Error: {e}"),
                    },
                    DeleteAction::Flag { project_id, key } => {
                        match app.client.delete_flag(*project_id, key) {
                            Ok(_) => {
                                app.status_message = "Flag deleted".into();
                                if let Screen::ProjectDetail { project } = app.screen.clone() {
                                    app.load_project_detail(&project);
                                }
                            }
                            Err(e) => app.status_message = format!("Error: {e}"),
                        }
                    }
                }
                app.dialog = None;
                app.input_mode = InputMode::Normal;
            }
            _ => {
                app.dialog = None;
                app.input_mode = InputMode::Normal;
            }
        },

        Dialog::Error { .. } => {
            app.dialog = None;
            app.input_mode = InputMode::Normal;
        }
    }
}
