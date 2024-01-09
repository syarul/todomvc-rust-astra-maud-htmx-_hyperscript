mod fragments;

// extern crate maud
pub use maud::*;

use astra::{Body, ConnectionInfo, Request, Response, ResponseBuilder, Server};
use fragments::{
    clear_completed, edit_todo, footer, /* filter_bar, */ page, todo_item, todo_list,
    toggle_main,
};
use serde::Serialize;
// use serde::ser::{SerializeStruct, Serializer};
use serde_json::json;
// use std::fmt;
// use cookie::{Cookie, SameSite};
// use headers::{HeaderMap, HeaderMapExt, HeaderValue};
use chrono::DateTime;
use std::{
    fmt::Debug,
    fs::read_to_string,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, MutexGuard, RwLock,
    },
    time::SystemTime,
};
// use time::{Duration, OffsetDateTime};
use url::form_urlencoded::parse;
use rand::distributions::Alphanumeric;
use rand::{thread_rng,Rng};

#[derive(Debug, Clone, PartialEq, Serialize)]
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

    let response_builder = ResponseBuilder::new();

    response_builder
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::new(mk_str))
        .unwrap()
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

fn selected_filter(filters: Arc<RwLock<Vec<Filter>>>) -> String {
    let filters_read = filters.read().unwrap();
    for filter in filters_read.iter() {
        if filter.selected {
            return filter.name.to_string();
        }
    }
    "All".to_string()
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
        "/set-hash" => {
            let filter_name = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "name"));

            if let Some(name) = filter_name {
                if !name.is_empty() {
                    Filter::update_selected_by_property("All".to_string(), &filters, |f| &f.name);
                } else {
                    // call to update_selected
                    Filter::update_selected_by_property(name, &filters, |f| &f.name);
                }
            }
            response(200, PreEscaped(String::new()))
        }
        "/learn.json" => {
            let json_str = PreEscaped(serde_json::to_string(&json!({})).unwrap());
            response(200, json_str)
        }
        "/update-counts" => {
            let update_counts_str = update_counts(&todos_lock);
            let struct_response = PreEscaped(update_counts_str);
            response(200, struct_response)
        }
        "/toggle-all" => {
            let checked = def_checked(&todos_lock);
            let struct_response = PreEscaped(checked.to_string());
            response(200, struct_response)
        }
        "/completed" => {
            let todo_incomplete = has_complete_task(&todos_lock);
            if todo_incomplete {
                let struct_response = clear_completed(todo_incomplete);
                return response(200, struct_response);
            }
            response(200, PreEscaped(String::new()))
        }
        "/footer" => {
            let filters_read = filters.read().unwrap();
            let struct_response =
                footer(&todos_lock, &filters_read, has_complete_task(&todos_lock));
            response(200, struct_response)
        }
        "/" => {
            let cookies = _req.headers().get("Cookie");
            let mut is_reset = false;
            for cookie in cookies.iter() {
                let cookie_result = cookie.to_str();
                if let Ok(cookie_value) = cookie_result {
                    let expires_str = cookie_value.split('=').last().unwrap_or_default();
                    // the cookie string needed to be formatted by removing 
                    // the nano seconds before parsing using crono DateTime
                    let expiration_time_str = &expires_str[..19];
                    if let Ok(parsed_datetime) = DateTime::parse_from_str(
                        &(expiration_time_str.to_owned() + " +00:00"),
                        "%Y-%m-%d %H:%M:%S %z",
                    ) {
                        let system_time_from_datetime = SystemTime::from(parsed_datetime);
                        if SystemTime::now() > system_time_from_datetime {
                            is_reset = true;
                        }
                    }
                }
            }

            // clone borrow checker on next line
            let filter_name = selected_filter(filters.clone());
            // acquire a read to access the filters array
            let filters_read = filters.read().unwrap();
            let checked = def_checked(&todos_lock);
            let mk = page(
                "HTMX • TodoMVC",
                &filters_read,
                &todos_lock,
                checked,
                has_complete_task(&todos_lock),
                &filter_name,
            );
            let mk_str = mk.into_string();

            if is_reset || cookies.is_none() {
                // reset data for client when cookie expired
                todos_lock.clear();
                id_counter.store(0, Ordering::Relaxed);

                // generate randomId string
                let mut rng = thread_rng();
                let random_session_id: String = (&mut rng).sample_iter(Alphanumeric)
                    .take(128)
                    .map(char::from)
                    .collect();

                let cookie_value = format!(
                    "sessionId={}; Max-Age={}; HttpOnly",
                    random_session_id,
                    600
                );
                return ResponseBuilder::new()
                    .header("Set-Cookie", cookie_value)
                    .body(Body::new(mk_str))
                    .unwrap();
            }
            ResponseBuilder::new().body(Body::new(mk_str)).unwrap()
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
                    if todos_lock.len() == 0 {
                        todos_lock.push(todo);
                        struct_response = todo_list(&todos_lock, &selected_filter(filters))
                    } else {
                        todos_lock.push(todo.clone());
                        struct_response = build_str_struct(
                            |todo| todo_item(todo, &selected_filter(filters)),
                            &todo,
                        );
                    }
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
                        let struct_response = build_str_struct(
                            |todo| todo_item(todo, &selected_filter(filters)),
                            todo,
                        );
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
                        let struct_response = build_str_struct(
                            |todo| todo_item(todo, &selected_filter(filters)),
                            todo,
                        );
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
        "/toggle-main" => {
            let struct_response = toggle_main(&todos_lock, def_checked(&todos_lock));
            response(200, struct_response)
        }
        "/toggle-footer" => {
            let filters_read = filters.read().unwrap();
            let struct_response =
                footer(&todos_lock, &filters_read, has_complete_task(&todos_lock));
            response(200, struct_response)
        }
        "/todo-list" => {
            println!("called me!");
            let struct_response = todo_list(&todos_lock, &selected_filter(filters));
            response(200, struct_response)
        }
        "/todo-json" => response(
            200,
            PreEscaped(serde_json::to_string(&*todos_lock).unwrap()),
        ),
        "/todo-item" => {
            let todo_id = _req
                .uri()
                .query()
                .and_then(|query| extract_query_param(query, "id"));
            if let Some(todo_id_str) = todo_id {
                if let Ok(todo_id) = todo_id_str.parse::<u32>() {
                    if let Some(todo) = todos_lock.iter_mut().find(|t| t.id == todo_id) {
                        let struct_response = build_str_struct(
                            |todo| todo_item(todo, &selected_filter(filters)),
                            todo,
                        );
                        return response(200, struct_response);
                    }
                }
            }
            response(400, PreEscaped(String::new()))
        }
        // serve axe-core for cypress testing
        "/node_modules/axe-core/axe.min.js" => {
            if let Ok(js_content) = read_to_string("node_modules/axe-core/axe.min.js") {
                let struct_response = PreEscaped(js_content);
                response(200, struct_response)
            } else {
                let struct_response = PreEscaped("500 Internal Server Error".to_string());
                response(500, struct_response)
            }
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
            name: "All",
            selected: true,
        },
        Filter {
            url: "#/active",
            name: "Active",
            selected: false,
        },
        Filter {
            url: "#/completed",
            name: "Completed",
            selected: false,
        },
    ]));

    Server::bind("localhost:8888")
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
