use svix_server::*;

use chrono::Duration;

use reqwest::blocking as req;
use std::thread;

/// Make a request to the server with an expiration time offset from the current time.
fn make_req(action: &str, delay: i64) -> (String, reqwest::StatusCode) {
    let t = chrono::Utc::now() + Duration::seconds(delay);
    let ts = t.timestamp().to_string();

    let client = req::Client::new();
    let resp = client.post(format!("http://127.0.0.1:8000/{action}?t={ts}")).send().unwrap();
    let status = resp.status();
    let body = resp.text().unwrap();

    (body, status)
}

fn foo_req() -> (String, reqwest::StatusCode) {
    make_req("foo", 1)
}

fn bar_req() -> (String, reqwest::StatusCode) {
    make_req("bar", 4)
}

fn baz_req() -> (String, reqwest::StatusCode) {
    make_req("baz", 7)
}

#[test]
fn test_basic_functionality() {
    let tempdir = tempfile::TempDir::new_in("run/").unwrap();

    let db_path = tempdir.path().join("dbsvix.sqlite");
    let db_path_ = db_path.clone();
    let _server = thread::spawn(move || {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8000));
        let server = SvixServer::new(db_path_, addr);
        server.start();
    });
   
    println!("started server");
    // wait for server to start
    thread::sleep(std::time::Duration::from_secs(1));

    println!("making reqs");
    println!("baz1");
    let _ = make_req("baz", -10);
    println!("foo1");
    let _ = foo_req();
    println!("bar1");
    let _ = bar_req();
    println!("baz2");
    let _ = baz_req();

    println!("waiting 10 seconds for tasks to finish...");
    // wait for all tasks to be finished
    thread::sleep(std::time::Duration::from_secs(10));

    // check db directly

    let db = Db::open(&db_path);
    let tasks = db.fetch_all_tasks();
    for task in tasks {
        assert!(task.2 == 1, "task not completed: {:?}", task);
    }

    // check output
    let output_file = tempdir.path().join("output.txt");
    let output: Vec<String> = std::fs::read_to_string(output_file).unwrap().split("\n").map(String::from).collect();
    assert!(output[0].starts_with("baz "));
    assert!(output[1].starts_with("foo "));
    assert!(output[2].starts_with("bar 200 OK"));
    assert!(output[3].starts_with("baz "));
    // read results from output file
    // should be:
    // baz <number>
    // foo <id>
    // bar <status>
    // baz <number>
}
