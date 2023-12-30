use std::path::PathBuf;

use rand::prelude::*;

use crate::{Db, DbId, Event, Task};

use std::sync::mpsc::*;

use std::io::Write;
use std::fs::File;

/// Db writer thread
pub fn db_executor(db_path: PathBuf, event_rx: Receiver<Event>) {
    let db: Db;
    // in tests it will always be created from scratch here
    if !db_path.exists() {
        db = Db::open(&db_path);
        db.create_tables();
    }
    else {
        db = Db::open(&db_path);
    }
    loop {
        let event = event_rx.recv().unwrap();
        match event {
            Event::New(task) => {
                let now = chrono::Utc::now();
                db.insert_task(task, now);
            },
            Event::Complete(id) => { db.complete_task(id); },
        }
    }
}

/// Get new tasks from the db that are due and runs them.
/// Then report back to the db when a task is finished via the mpsc sender.
/// This could also be async but it's just simpler to do it sync.
pub fn task_executor(db_path: PathBuf, finished_tx: Sender<Event>) {
    let output_path = db_path.parent().unwrap().join("output.txt");
    let mut output = File::create(&output_path).unwrap();
    let db = Db::open(&db_path); // open but do not create tables, db must exist.
    loop {
        // fetch and execute new tasks
        let now = chrono::Utc::now();
        let tasks = db.fetch_pending_tasks_due_by(now);
        for (id, task) in tasks {
            match task {
                Task::Foo => {exec_foo(id, &mut output)},
                Task::Bar => {exec_bar(id, &mut output)},
                Task::Baz => {exec_baz(id, &mut output)},
            }

            finished_tx.send(Event::Complete(id)).unwrap();
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn exec_foo(task_id: DbId, output: &mut File) {
    std::thread::sleep(std::time::Duration::from_secs(3));

    let text = format!("foo {}", task_id.0);
    println!("{}", text);
    writeln!(output, "{}", text).unwrap();
}

fn exec_bar(_task_id: DbId, output: &mut File) {
    let resp = reqwest::blocking::get("https://example.com/").unwrap();
    
    let text = format!("bar {}", resp.status());
    println!("{}", text);
    writeln!(output, "{}", text).unwrap();
}

fn exec_baz(_task_id: DbId, output: &mut File) {
    let mut rng = thread_rng();
    let n: u32 = rng.gen_range(0..344);

    let text = format!("baz {}", n);
    println!("{}", text);
    writeln!(output, "{}", text).unwrap();
}

// TODO
//#[cfg(test)]
//mod test;
