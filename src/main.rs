use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use log;
use session::Session;
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

mod content;
mod session;
#[cfg(test)]
mod tests;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[clap(long, default_value = "127.0.0.1:9000")]
    addr: String,

    #[clap(long, short, default_value = "0")]
    max_sessions: usize,

    #[clap(long, default_value = "/tmp/chrome_server")]
    data_root: String,

    #[clap(long, default_value = "/")]
    prefix: String,

    #[clap(long, default_value = "info")]
    log_level: String,

    #[clap(long, default_value = "false", help = "enable private ip access")]
    enable_private_ip: bool,
}

fn init_log(level: String, is_test: bool) {
    let _ = env_logger::builder()
        .is_test(is_test)
        .format(|buf, record| {
            let short_file_name = record
                .file()
                .unwrap_or("unknown")
                .split('/')
                .last()
                .unwrap_or("unknown");

            writeln!(
                buf,
                "{} [{}] {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                short_file_name,
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .format_timestamp(None)
        .filter_level(level.parse().unwrap())
        .try_init();
}

#[derive(Clone)]
pub struct AppState {
    sessions: Arc<Mutex<Vec<Session>>>,
    max_sessions: usize,
    data_root: String,
    enable_private_ip: bool,
}

impl AppState {
    pub fn new(data_root: String, max_sessions: usize) -> Self {
        AppState {
            sessions: Arc::new(Mutex::new(Vec::new())),
            max_sessions,
            data_root,
            enable_private_ip: false,
        }
    }
    pub fn is_full(&self) -> bool {
        if self.max_sessions <= 0 {
            return false;
        }
        self.sessions.lock().unwrap().len() >= self.max_sessions
    }
}
type StateRef = Arc<AppState>;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Cli::parse();
    let addr = args.addr;
    let prefix = args.prefix;

    init_log(args.log_level, false);

    let state = Arc::new(AppState {
        data_root: args.data_root,
        max_sessions: args.max_sessions,
        sessions: Arc::new(Mutex::new(Vec::new())),
        enable_private_ip: args.enable_private_ip,
    });
    let router = Router::new()
        .route("/", get(session::create_session))
        .route("/list", get(session::list_session))
        .route("/kill/:session_id", post(session::kill_session))
        .route("/kill_all", post(session::killall_session))
        .route("/pdf", get(content::render_pdf))
        .route("/screenshot", get(content::render_screenshot))
        .route("/text", get(content::dump_text))
        .route("/html", get(content::dump_html))
        .with_state(state);

    let app = Router::new().nest(&prefix, router);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    log::warn!("Starting server on {} -> {}", addr, prefix);
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
