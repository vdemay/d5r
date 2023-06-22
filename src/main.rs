#![forbid(unsafe_code)]
#![warn(
    clippy::expect_used,
    clippy::nursery,
    clippy::pedantic,
    clippy::todo,
    clippy::unused_async,
    clippy::unwrap_used
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::doc_markdown,
    clippy::similar_names
)]
// Only allow when debugging
// #![allow(unused)]

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use bollard::Docker;
use parking_lot::Mutex;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{info, Level};

use app_data::AppData;
use app_error::AppError;
use docker_data::DockerData;
use input_handler::InputMessages;
use parse_args::CliArgs;
use ui::{GuiState, Status, Ui};

use crate::docker_data::DockerMessage;

mod app_data;
mod app_error;
mod docker_data;
mod input_handler;
mod parse_args;
mod ui;

/// This is the entry point when running as a Docker Container, and is used, in conjunction with the `CONTAINER_ENV` ENV, to check if we are running as a Docker Container
const ENTRY_POINT: &str = "/app/oxker";
const ENV_KEY: &str = "OXKER_RUNTIME";
const ENV_VALUE: &str = "container";

/// Enable tracing, only really used in debug mode, for now
/// write to file if `-g` is set?
fn setup_tracing() {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
}

/// An ENV is set in the ./containerised/Dockerfile, if this is ENV found, then sleep for 250ms, else the container, for as yet unknown reasons, will close immediately
/// returns a bool, so that the `update_all_containers()` won't bother to check the entry point unless running via a container
fn check_if_containerised() -> bool {
    if std::env::vars().any(|x| x == (ENV_KEY.into(), ENV_VALUE.into())) {
        std::thread::sleep(std::time::Duration::from_millis(250));
        true
    } else {
        false
    }
}

/// Create docker daemon handler, and only spawn up the docker data handler if a ping returns non-error
async fn docker_init(
    app_data: &Arc<Mutex<AppData>>,
    containerised: bool,
    docker_rx: Receiver<DockerMessage>,
    gui_state: &Arc<Mutex<GuiState>>,
    is_running: &Arc<AtomicBool>,
) {
    if let Ok(docker) = Docker::connect_with_socket_defaults() {
        if docker.ping().await.is_ok() {
            let app_data = Arc::clone(app_data);
            let gui_state = Arc::clone(gui_state);
            let is_running = Arc::clone(is_running);
            tokio::spawn(DockerData::init(
                app_data,
                containerised,
                docker,
                docker_rx,
                gui_state,
                is_running,
            ));
        } else {
            app_data.lock().set_error(AppError::DockerConnect);
            gui_state.lock().status_push(Status::DockerConnect);
        }
    } else {
        app_data.lock().set_error(AppError::DockerConnect);
        gui_state.lock().status_push(Status::DockerConnect);
    }
}

/// Create data for, and then spawn a tokio thread, for the input handler
fn handler_init(
    app_data: &Arc<Mutex<AppData>>,
    docker_sx: &Sender<DockerMessage>,
    gui_state: &Arc<Mutex<GuiState>>,
    input_rx: Receiver<InputMessages>,
    is_running: &Arc<AtomicBool>,
) {
    let input_app_data = Arc::clone(app_data);
    let input_gui_state = Arc::clone(gui_state);
    let input_is_running = Arc::clone(is_running);
    tokio::spawn(input_handler::InputHandler::init(
        input_app_data,
        input_rx,
        docker_sx.clone(),
        input_gui_state,
        input_is_running,
    ));
}

#[tokio::main]
async fn main() {
    let containerised = check_if_containerised();

    setup_tracing();

    let args = CliArgs::new();
    let app_data = Arc::new(Mutex::new(AppData::default(args)));
    let gui_state = Arc::new(Mutex::new(GuiState::default()));
    let is_running = Arc::new(AtomicBool::new(true));
    let (docker_sx, docker_rx) = tokio::sync::mpsc::channel(32);
    let (input_sx, input_rx) = tokio::sync::mpsc::channel(32);

    docker_init(&app_data, containerised, docker_rx, &gui_state, &is_running).await;

    handler_init(&app_data, &docker_sx, &gui_state, input_rx, &is_running);

    if args.gui {
        Ui::create(app_data, docker_sx, gui_state, is_running, input_sx).await;
    } else {
        info!("in debug mode");
        while is_running.load(Ordering::SeqCst) {
            // Debug mode for testing, mostly pointless, doesn't take terminal
            loop {
                docker_sx.send(DockerMessage::Update).await.ok();
                tokio::time::sleep(std::time::Duration::from_millis(u64::from(
                    args.docker_interval,
                )))
                .await;
            }
        }
    }
}
