# Design

I'm just using sqlite as the database. Horizontal scaling may be handled by having a database per-instance, but see below discussion about tradeoffs. sqlite is used in WAL mode to allow multiple readers (with a single writer).

The system contains: An axum webserver (defined in `src/lib.rs`), a database `src/db.rs`, and the task execution code `src/task_exec.rs`. `task_exec.rs` contains a database thread function and a task execution thread function, which are called in `src/lib.rs` inside `SvixServer::start`.

See `tests/basic_functionality.rs` and `SvixServer::start` in `src/lib.rs` for how everything is set up.

When a request comes in to the web server, it gets added to a queue (a `std::sync::mpsc::channel`) which is read by the database thread. The database thread stores the events in the database, initially with a status of new.

The executor thread reads (but does not write to) the database in a loop and retrieves all events due prior to the current time. For each task, it executes the given action. The instructions said to run in "the worker" but I was planning to run each task in a separately-spawned thread before I noticed it, so currently all tasks run sequentially. When a task finishes, it notifies the database to mark that task as complete via the same mpsc channel. After all tasks are completed, the executor thread sleeps for 1 second.

The external CRUD API just reads from the database directly except in the case of writes, where it sends an event to the database thread. (Note: I ran out of time and didn't actually implement the crud ops in the external api)

This design may seem overly complicated with the separate database thread but it's something that I've implemented previously in some example code for a library I wrote (where I wanted the database code separate from the http code) which is why I went for it.

## Issues

I've discussed some issues that I would fix, or tradeoffs that I made here, besides the obvious things like using unwrap instead of proper error types.

## Missing functionality

I ran out of time and didn't implement the external API's list/show/delete endpoints. The create endpoints also just return "ok" and don't give you the task id. I would have implemented that by either adding an async response send/receive channel from the handler to the db thread, or by simply getting rid of teh db thread and moving the db operations into the handlers - but the rusqlite client isn't async so that adds a tiny complication (you just need to await a blocking future via `spawn_blocking`)

### Durability vs Only-once
If the server is shut down when a "complete" event is on the events queue but not processed by the db thread, it will try to re-execute the event upon restart.

### Database and scaling
When scaling horizontally with multiple databases, the logic to query and filter all tasks is more complicated because you have to merge the results from each instance at some higher level. This is a tradeoff - sometimes this operation is rare and it's fine for it to take more time or be more expensive, and other times this is the main operation. You can often shard per-customer which makes some query operations simpler.

For an event queue (e.g. for serving webhooks) I think this tradeoff is perfectly fine since most of the time you're just adding stuff and not removing or listing all the items. If it isn't okay, the current code's database thread could be replaced with a postgres connection but you'd have to do some work to make sure two different instances don't try to get the same task - you can't just "get everything and then execute them all", you'd have to do something like start a transaction, turn the tasks you want to execute to pending, and then make sure you didn't get any write conflicts by e.g. setting a "current executing task id" column in your transaction and checking it matches or explicitly using row-level locking in your SQL. 

Database ids are used in the public interface for convenience but this is not ideal in production.

### Sync task execution
Originally I wanted to do the task execution such that each task runs in its own thread, but then I noticed the instructions implied they should all be run in a single thread or executor. It's very likely that an async setup would be more performant vs either one, especially when doing lots of IO operations that could be pushed into the kernel and awaited in the executor's pool of threads rather than spawning a new one for each task.

The other issue with the task executor is that it doesn't really care that much about being late on delivery - it runs all the tasks, waits 1 second, and then starts over to look for tasks that are past due. It doesn't actively plan to say "the next task isn't due for an hour so let's sleep until then". This makes it easier to handle restarts with unexecuted tasks and new tasks being added that are due before the ones we currently have.

### Testing

Typically I'd write tests for each layer of the system, in this case the database and event executor layers, in addition to the top layer API. Due to the time constraints I only wrote a single test that just exercises the HTTP API directly.

# Timeline
I got the project Wednesday night, thought about it Thursday but was otherwise busy, and finished it Friday night. I spent about 3 and a half hours on it.
