use std::net::SocketAddr;

use std::path::PathBuf;

use std::thread;
use std::sync::mpsc::*;

use axum::{
    extract::State,
    routing::post,
    Router,
};

mod db;
pub use db::*;

mod task_exec;
pub use task_exec::*;

#[derive(Debug, Clone, Copy)]
pub enum Task {
    Foo,
    Bar,
    Baz
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    New(Task),
    Complete(DbId),
}

impl Task {
    pub fn from_str(s: &str) -> Task {
        match s {
            "foo" => Task::Foo,
            "bar" => Task::Bar,
            "baz" => Task::Baz,
            _ => panic!("unidentified task type {}", s),
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Task::Foo => "foo",
            Task::Bar => "bar",
            Task::Baz => "baz",
        }
    }
}

async fn handle_foo(State(db_tx): State<Sender<Event>>) -> &'static str {
    db_tx.send(Event::New(Task::Foo)).unwrap();
    "ok"
}
async fn handle_bar(State(db_tx): State<Sender<Event>>) -> &'static str {
    db_tx.send(Event::New(Task::Bar)).unwrap();
    "ok"
}
async fn handle_baz(State(db_tx): State<Sender<Event>>) -> &'static str {
    db_tx.send(Event::New(Task::Baz)).unwrap();
    "ok"
}

pub struct SvixServer {
    db_path: PathBuf,
    addr: SocketAddr,
}

impl SvixServer {
    pub fn new(db_path: PathBuf, addr: SocketAddr) -> SvixServer {
        SvixServer { db_path, addr }
    }

    pub fn start(self) {
        let (db_tx, db_rx) = channel();

        // Start db thread
        let path = self.db_path.clone();
        let _db_thread = thread::spawn(move || {
            db_executor(path, db_rx);
        });

        // TODO: use a separate mpsc to signal db is ready
        thread::sleep(std::time::Duration::from_secs(1));

	// Start executor thread
        let path = self.db_path.clone();
        let finished_tx = db_tx.clone();
        let _event_thread = thread::spawn(move || {
            task_executor(path, finished_tx);
        });

        // Start axum server
        let app = Router::new()
            .route("/foo", post(handle_foo))
            .route("/bar", post(handle_bar))
            .route("/baz", post(handle_baz))
            .with_state(db_tx.clone());
	tokio::runtime::Builder::new_multi_thread()
	    .enable_all()
	    .build()
	    .unwrap()
	    .block_on(async {

		let addr = self.addr;
		println!("listening on {addr}");
		let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
		axum::serve(listener, app).await.unwrap();
	});
    }
}
