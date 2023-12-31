mod fragments;

// extern crate maud
pub use maud::*;

use astra::{Body, ConnectionInfo, Request, Response, ResponseBuilder, Server};
use fragments::{edit_todo, filter_bar, page, todo_item};
use serde_json::json;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex, MutexGuard, RwLock,
};
use url::form_urlencoded::parse;

#[derive(Debug, Clone, PartialEq)]
struct Todo {
    id: u32,
    task: String,
    done: bool,
    editing: bool,
}

impl Todo {
    // helper method to create a new instance with a calculated ID
    // the counter can only go up, which good enough for in-memory indexing
    // if we storing the data elsewhere this might not be the case anymore
    fn new_id(task: String, done: bool, editing: bool, counter: &Arc<AtomicU32>) -> Todo {
        let id = counter.fetch_add(1, Ordering::Relaxed); // increment the counter for the next ID
        Todo {
            id: id.into(),
            task,
            done,
            editing,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    url: &'static str,
    name: &'static str,
    selected: bool,
}

// use trait to make generic on implementation
trait UpdateSelected {
    type Item;
    fn update_selected_by_property(
        value: String,
        filters: &Arc<RwLock<Vec<Self::Item>>>,
        property: fn(&Self::Item) -> &str,
    );
}

impl UpdateSelected for Filter {
    type Item = Filter;
    fn update_selected_by_property(
        value: String,
        filters: &Arc<RwLock<Vec<Filter>>>,
        property: fn(&Filter) -> &str,
    ) {
        let mut filters_write = filters.write().unwrap();
        for filter in &mut *filters_write {
            filter.selected = property(filter) == value;
        }
    }
}

// utility function to extract a parameter from the query string
fn extract_query_param(query: &str, param_name: &str) -> Option<String> {
    for (key, value) in parse(query.as_bytes()) {
        if key == param_name {
            return Some(value.into_owned());
        }
    }
    None
}

fn response(status: u16, mk: PreEscaped<String>) -> Response {
    let mk_str = mk.into_string();
    ResponseBuilder::new()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::new(mk_str))
        .unwrap()
}

// use generic so we can use this for different template
fn build_str_vector<F, T>(template_frag: F, vector: &Arc<RwLock<Vec<T>>>) -> PreEscaped<String>
where
    F: FnOnce(&[T]) -> Markup,
    T: Clone + PartialEq,
{
    let vector_read = vector.read().unwrap();
    let mk = template_frag(&vector_read);
    mk
}

// use generic so we can use this for different template
fn build_str_struct<F, T>(template_frag: F, obj: &T) -> PreEscaped<String>
where
    F: FnOnce(&T) -> Markup,
    T: Clone + PartialEq,
{
    let mk = template_frag(&obj);
    mk
}

fn count_not_done(todos: &MutexGuard<'_, Vec<Todo>>) -> usize {
    todos.iter().filter(|&todo| !todo.done).count()
}

fn def_checked(todos: &MutexGuard<'_, Vec<Todo>>) -> bool {
    let uncompleted_count = count_not_done(todos);
    let default_checked = uncompleted_count == 0 && !todos.is_empty();
    default_checked
}

fn has_complete_task(todos: &MutexGuard<'_, Vec<Todo>>) -> bool {
    for todo in todos.iter() {
        if todo.done {
            return true;
        }
    }
    false
}

fn update_counts(todos: &MutexGuard<'_, Vec<Todo>>) -> String {
    let uncompleted_count = count_not_done(todos);
    let plural = if uncompleted_count != 1 { "s" } else { "" };

    format!("<strong>{} item{} left</strong>", uncompleted_count, plural)
}

fn handle_request(
    _req: Request,
    _info: ConnectionInfo,
    id_counter: Arc<AtomicU32>,
    todos: Arc<Mutex<Vec<Todo>>>,
    filters: Arc<RwLock<Vec<Filter>>>,
) -> Response {
    // acquire the lock to access and modify the todos vector
    let mut todos_lock = todos.lock().unwrap();

    match _req.uri().path() {
        "/" => {
            // acquire a read to access the filters array
            let filters_read = filters.read().unwrap();
            let checked = def_checked(&todos_lock);
            let mk = page("HTMX • TodoMVC", &filters_read, &todos_lock, checked);
            let mk_str = mk.into_string();
            Response::new(Body::new(mk_str))
        }
        "/get-hash" => {
            let filter_name = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "name"));
            let filter_hash = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "hash"));
            let vector_response;
            if let Some(name) = filter_name {
                // call to update_selected
                Filter::update_selected_by_property(name, &filters, |f| &f.name);
            } else {
                if let Some(hash) = filter_hash {
                    if !hash.is_empty() {
                        Filter::update_selected_by_property(hash, &filters, |f| &f.name);
                    } else {
                        Filter::update_selected_by_property("all".to_string(), &filters, |f| {
                            &f.name
                        });
                    }
                }
            }
            vector_response = build_str_vector(|filters| filter_bar(filters), &filters);
            response(200, vector_response)
        }
        "/add-todo" => {
            let todo_task = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "task"));
            let struct_response;
            if let Some(task) = todo_task {
                if !task.trim().is_empty() {
                    let todo = Todo::new_id(task, false, false, &id_counter);
                    todos_lock.push(todo.clone());
                    struct_response = build_str_struct(|todo| todo_item(todo), &todo);
                    return response(200, struct_response);
                } else {
                    return response(200, PreEscaped(String::new()));
                }
            }
            response(400, PreEscaped(String::new()))
        }
        "/toggle-todo" => {
            let todo_id = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "id"));
            if let Some(todo_id_str) = todo_id {
                if let Ok(todo_id) = todo_id_str.parse::<u32>() {
                    if let Some(todo) = todos_lock.iter_mut().find(|t| t.id == todo_id) {
                        todo.done = !todo.done;
                        let struct_response = build_str_struct(|todo| todo_item(todo), todo);
                        return response(200, struct_response);
                    }
                }
            }
            response(400, PreEscaped(String::new()))
        }
        "/edit-todo" => {
            let todo_id = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "id"));
            if let Some(todo_id_str) = todo_id {
                if let Ok(todo_id) = todo_id_str.parse::<u32>() {
                    if let Some(todo) = todos_lock.iter_mut().find(|t| t.id == todo_id) {
                        // clone the todo and update editing to true
                        let mut clone_todo = todo.clone();
                        clone_todo.editing = true;
                        let struct_response =
                            build_str_struct(|clone_todo| edit_todo(&clone_todo), &clone_todo);
                        return response(200, struct_response);
                    }
                }
            }
            response(400, PreEscaped(String::new()))
        }
        "/update-todo" => {
            let todo_id = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "id"));
            let todo_task = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "task"));
            let task = todo_task.unwrap_or_else(|| String::new());
            if let Some(todo_id_str) = todo_id {
                if let Ok(todo_id) = todo_id_str.parse::<u32>() {
                    if let Some(todo) = todos_lock.iter_mut().find(|t| t.id == todo_id) {
                        if !task.trim().is_empty() {
                            todo.task = task;
                        } else {
                            // behave same as remove if user send empty task
                            todos_lock.retain(|t| t.id != todo_id);
                            return response(200, PreEscaped(String::new()));
                        }
                        let struct_response = build_str_struct(|todo| todo_item(todo), todo);
                        return response(200, struct_response);
                    }
                }
            }
            response(400, PreEscaped(String::new()))
        }
        "/remove-todo" => {
            let todo_id = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "id"));
            if let Some(todo_id_str) = todo_id {
                if let Ok(todo_id) = todo_id_str.parse::<u32>() {
                    todos_lock.retain(|t| t.id != todo_id);
                    return response(200, PreEscaped(String::new()));
                }
            }
            response(400, PreEscaped(String::new()))
        }
        "/completed" => {
            let mut display_style = "none";
            let todo_incomplete = has_complete_task(&todos_lock);
            if todo_incomplete {
                display_style = "block"
            }
            let struct_response = PreEscaped(display_style.to_string());
            response(200, struct_response)
        }
        "/toggle-all" => {
            let checked = def_checked(&todos_lock);
            let struct_response = PreEscaped(checked.to_string());
            response(200, struct_response)
        }
        "/update-counts" => {
            let update_counts_str = update_counts(&todos_lock);
            let struct_response = PreEscaped(update_counts_str);
            response(200, struct_response)
        }
        "/learn.json" => {
            let json_str = PreEscaped(serde_json::to_string(&json!({})).unwrap());
            response(200, json_str)
        }
        _ => {
            let struct_response = PreEscaped("404 Not Found".to_string());
            response(404, struct_response)
        }
    }
}
fn main() {
    // choose 3 different ownership concepts with Atomic, Mutex and RwLock
    // wrap all in Arc
    // use Atomic for the id_counter
    let id_counter = Arc::new(AtomicU32::new(0));
    // initialize the todos vector, use Mutex, lock for any operations ensure
    // the atomic counter always sync when the length of the vector goes up
    let todos = Arc::new(Mutex::new(Vec::new()));
    // initialize the filters vector, use RwLock
    // the filters will never change in length with the only changes is for updating
    // the select parameters, so we do not need to lock with Mutex
    let filters = Arc::new(RwLock::new(vec![
        Filter {
            url: "#/",
            name: "all",
            selected: true,
        },
        Filter {
            url: "#/active",
            name: "active",
            selected: false,
        },
        Filter {
            url: "#/completed",
            name: "completed",
            selected: false,
        },
    ]));

    Server::bind("localhost:8000")
        .serve(move |_req, _info| {
            handle_request(
                _req,
                _info,
                Arc::clone(&id_counter),
                Arc::clone(&todos),
                Arc::clone(&filters),
            )
        })
        .expect("serve failed");
}
