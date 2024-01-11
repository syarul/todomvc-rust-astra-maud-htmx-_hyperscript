use crate::{Filter, Todo};
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub fn clear_completed(has_completed: bool) -> Markup {
    html! {
        @if has_completed {
            button
                class="clear-completed"
                _="
                    on load set $clearCompleted to me
                    on click send destroy to <li.completed/>
                " { 
                "Clear completed"
            }
        }
    }
}

fn toggle_all(toggle: bool) -> Markup {
    html! {
        input
            id="toggle-all"
            class="toggle-all"
            type="checkbox"
            checked[toggle]
            _="
                on load set $toggleAll to me
                on toggleAll debounced at 100ms
                    fetch /toggle-all then
                        if it === 'true' and my.checked === false then
                            set my.checked to true
                        else
                            if my.checked === true and it === 'false' then set my.checked to false
                        end
                end
                on click send toggle to <input.toggle/>
            " {}
    }
}

pub fn filter_bar(filters: &[Filter]) -> Markup {
    html! {
        ul class="filters" _="on load set $filter to me" {
            @for filter in filters {
                li {
                    a
                        class={ @if filter.selected { "selected" } }
                        href={ (filter.url) }
                        _="on click add .selected to me"
                        { (filter.name) }
                }
            }
        }
    }
}

// blur event handle both keyup ESC and blur
// where only blur should trigger update call while ESC is not
pub fn edit_todo(todo: &Todo) -> Markup {
    let value = if todo.editing { &todo.task } else { "" };
    html! {
        input
            class="edit"
            name="task"
            value={ (value) }
            _="
                on load
                    my.focus()
                on keyup[keyCode==27]
                    set $keyup to 'esc'
                    remove .editing from closest <li/>
                on keyup[keyCode==13]
                    set $keyup to 'enter'
                    htmx.ajax('GET', `/update-todo?id=${my.parentNode.id.slice(5)}&task=${my.value}`, {target: closest <li/>, swap:'outerHTML'})
                on blur debounced at 10ms
                    if $keyup === 'enter'
                        set $keyup to 'none'
                    else if $keyup === 'esc'
                        set $keyup to 'none'
                    else
                    htmx.ajax('GET', `/update-todo?id=${my.parentNode.id.slice(5)}&task=${my.value}`, {target: closest <li/>, swap:'outerHTML'})
                end
                send toggleMain to <section.todoapp/>
                send toggleFooter to <section.todoapp/>
            " {}
    }
}

fn todo_check(todo: &Todo) -> Markup {
    let toggle = todo.done == true;
    html! {
        input
            class="toggle"
            type="checkbox"
            checked[toggle]
            hx-patch={ "/toggle-todo?id="(todo.id)"&done="(todo.done) }
            hx-target="closest <li/>"
            hx-swap="outerHTML"
            _="
            on htmx:afterRequest
                send toggleAll to <input.toggle-all/>
                send toggleClearCompleted to <footer.footer/>
            on toggle
                send toggleClearCompleted to <footer.footer/>
                if $toggleAll.checked and my.checked === false
                    my.click()
                else if $toggleAll.checked === false and my.checked
                    my.click()
            " {}
    }
}

pub fn todo_item(todo: &Todo, filter_name: &str) -> Markup {
    let should_render = !todo.done && filter_name == "Active"
        || todo.done && filter_name == "Completed"
        || filter_name == "All";
    html! {
        @if should_render {
            li
                id={ "todo-"(todo.id) }
                class={
                    "todo "
                    @if todo.done { "completed " }
                    @if todo.editing { "editing" }
                }
                _="on destroy my.querySelector('button').click()" {
                div class="view" {
                    (todo_check(todo))
                    label
                        hx-trigger="dblclick"
                        hx-patch={ "/edit-todo?id="(todo.id) }
                        hx-target="next input"
                        hx-swap="outerHTML"
                        _="
                        on dblclick add .editing to the closest <li/>
                        on htmx:afterRequest
                            set $el to my.parentNode.nextSibling
                            set $el.selectionStart to $el.value.length
                        " { (todo.task) }
                    button
                        class="destroy"
                        hx-delete={ "/remove-todo?id="(todo.id) }
                        hx-trigger="click"
                        hx-target="closest <li/>"
                        hx-swap="outerHTML"
                        _="
                            on htmx:afterRequest debounced at 5ms
                                send toggleMain to <section.todoapp/>
                                send toggleFooter to <section.todoapp/>
                                send focus to <input.new-todo/>
                                if $todo
                                    send toggleClearCompleted to <footer.footer/>
                        " {}
                }
                (edit_todo(todo))
            }
        }
    }
}

pub fn toggle_main(todos: &[Todo], checked: bool) -> Markup {
    let has_length = todos.len() != 0;
    html! {
        @if has_length {
            section
                class="main" _="on load set $sectionMain to me" {
                    { (toggle_all(checked)) }
                    label for="toggle-all" {
                        "Mark all as complete"
                    }
                }
        }
    }
}

pub fn footer(todos: &[Todo], filters: &[Filter], has_completed: bool) -> Markup {
    let has_todos = todos.len() != 0;
    html! {
        @if has_todos {
            footer
                class="footer"
                _="
                    on load set $footerFooter to me
                    on toggleClearCompleted debounced at 20ms
                        if $clearCompleted === undefined
                            htmx.ajax('GET', '/completed', {target:'.filters', swap:'afterend'})
                        else
                            // need to first set to undefined in case the fetch may return empty which
                            // will indiscriminately leave it in incorrect state
                            set $clearCompleted to undefined
                            htmx.ajax('GET', '/completed', {target:'.clear-completed', swap:'outerHTML'})
                    send toggleFooter to <section.todoapp/>
                " {
                    span
                        class="todo-count"
                        hx-trigger="load"
                        _="
                            on load send todoCount to me
                            on todoCount debounced at 100ms
                            fetch /update-counts then put the result into me
                        " {}
                    (filter_bar(filters))
                    (clear_completed(has_completed))
                }
        }
    }
}

pub fn todo_list(todos: &[Todo], filter_name: &str) -> Markup {
    let has_todos = todos.len() != 0;
    html! {
        @if has_todos {
            ul
                class="todo-list"
                _="on load set $todo to me" {
                @for todo in todos {
                    { (todo_item(todo, filter_name)) }
                }
            }
        }
    }
}

fn todoapp(
    filters: &[Filter],
    todos: &[Todo],
    checked: bool,
    has_completed: bool,
    filter_name: &str,
) -> Markup {
    html! {
        body {
            section
                class="todoapp"
                _="
                    on toggleMain debounced at 20ms
                        // log 'toggleMain'
                        if $sectionMain
                            set $sectionMain to undefined
                            htmx.ajax('GET', '/toggle-main', {target:'section.main', swap:'outerHTML'})
                        else
                            htmx.ajax('GET', '/toggle-main', {target:'.todo-list', swap:'beforebegin'})
                        end
                    on toggleFooter debounced at 20ms
                        // log 'toggleFooter'
                        if $footerFooter
                            fetch /todo-json as json then
                                if $todo.hasChildNodes() === false and it.length === 0
                                    remove $footerFooter
                                    set $footerFooter to undefined
                                end
                            // set-hash already update the hash on the server
                            // this reassign the filter class selected base on user interaction
                            // or location hash changes
                            for filter in $filter.children
                                if filter.textContent === 'All' and `${$initial}${$after}` === ''
                                    add .selected to filter.firstChild
                                else if filter.textContent !== `${$initial}${$after}`
                                    remove .selected from filter.firstChild
                                end
                            end
                            // update counts
                            fetch /update-counts then put the result into <span.todo-count/>
                        else
                            htmx.ajax('GET', '/footer', {target:'.header', swap:'beforeend'})
                        end
                    on show wait 20ms
                        log 'fetch show'
                        // this is the DOM tree diffing of the todo-list, fetch only the needed
                        // to render and remove accordingly base on route All/Active/Completed
                        fetch /todo-json as json then
                            if window.location.hash === '#/active'
                                for todo in it
                                    if todo.done
                                        document.getElementById(`todo-${todo.id}`) then if it remove it end
                                    else
                                        document.getElementById(`todo-${todo.id}`) then
                                            if it === null
                                                htmx.ajax('GET', `/todo-item?id=${todo.id}`, {target:'.todo-list', swap:'beforeend'})
                                            end
                                    end
                                end
                            else if window.location.hash === '#/completed'
                                for todo in it
                                    if todo.done
                                        document.getElementById(`todo-${todo.id}`) then
                                            if it === null
                                                htmx.ajax('GET', `/todo-item?id=${todo.id}`, {target:'.todo-list', swap:'beforeend'})
                                            end
                                    else
                                        document.getElementById(`todo-${todo.id}`) then if it remove it end
                                    end
                                end
                            else
                                // loop through the JSON
                                for todo in it
                                    // check if the element exist in the current DOM, add if none
                                    // placement is decided according to order if there's an element
                                    // with higher than the current todo swap as 'beforebegin'
                    
                                    for el in $todo.children
                                        if parseInt(el.id.slice(5)) > todo.id and document.getElementById(`todo-${todo.id}`) === null
                                        htmx.ajax('GET', `/todo-item?id=${todo.id}`, {target: `#${el.id}`, swap:'beforebegin'})
                                        end
                                    end
                
                                    // do reverse lookup for lower than the current todo swap as 'afterend'
                                    for el in Array.from($todo.children).reverse()
                                        if parseInt(el.id.slice(5)) < todo.id and document.getElementById(`todo-${todo.id}`) === null
                                        htmx.ajax('GET', `/todo-item?id=${todo.id}`, {target: `#${el.id}`, swap:'afterend'})
                                        end
                                    end
                
                                    // if todo is empty initially recursively add all of it
                                    if $todo.children.length === 0
                                        htmx.ajax('GET', `/todo-item?id=${todo.id}`, {target:'.todo-list', swap:'beforeend'})
                                    end
                                end
                "
                {
                    header class="header" {
                        h1 { "todos" }
                        input
                            id="add-todo"
                            name="task"
                            class="new-todo"
                            placeholder="What needs to be done?"
                            _="
                                on load send focus to me
                                on focus
                                    if $focus === undefined
                                        my.focus()
                                        set $isFocus to 'true'
                                    end
                                on blur set $isFocus to undefined
                                on keyup[keyCode==13]
                                    if $todo
                                        htmx.ajax('GET', `/add-todo?task=${my.value}`, {target:'.todo-list', swap:'beforeend'})
                                        set my value to ''
                                    else
                                        htmx.ajax('GET', `/add-todo?task=${my.value}`, {target:'.header', swap:'beforeend'})
                                        set my value to ''
                                    end
                                        send toggleMain to <section.todoapp/>
                                        send toggleFooter to <section.todoapp/>
                            " {}
                    }
                    { (toggle_main(todos, checked)) }
                    { (todo_list(todos, filter_name))}
                    { (footer(todos, filters, has_completed)) }
                }
        }
    }
}

fn header(page_title: &str) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" data-framework="htmx";
        head {
            meta charset="utf-8";
            title { (page_title) }
            link rel="stylesheet" type="text/css" href="https://unpkg.com/todomvc-common@1.0.5/base.css";
            link rel="stylesheet" type="text/css" href="https://unpkg.com/todomvc-app-css/index.css";
        }
    }
}

fn info() -> Markup {
    html! {
        footer
            class="info"
            _="
                on load debounced at 10ms
                    call startMeUp()
                    hashCache()
            " {
            p { "Double-click to edit a todo" }
            p { "Created by " a href="http://github.com/syarul/" { "syarul" } }
            p { "Part of " a href="http://todomvc.com" { "TodoMVC" } }
            img src="https://htmx.org/img/createdwith.jpeg" width="250" height="auto" {}
        }
    }
}

fn scripts() -> Markup {
    let start_me_up = PreEscaped(
        r#"
        def startMeUp()
            log "
             ooooo   ooooo ooooooooooooo ooo        ooooo ooooooo  ooooo 
             `888'   `888' 8'   888   `8 `88.       .888'  `8888    d8'  
              888     888       888       888b     d'888     Y888..8P    
              888ooooo888       888       8 Y88. .P  888      `8888'     
              888     888       888       8  `888'   888     .8PY888.    
              888     888       888       8    Y     888    d8'  `888b   
             o888o   o888o     o888o     o8o        o888o o888o  o88888o
             ===========================================================
                  Build with RUST, ASTRA, MAUD, HTMX & _HYPERSCRIPT
            _   _                 _       _     _                         
           | |_| |__   ___   _ __(_) __ _| |__ | |_  __      ____ _ _   _ 
           | __| '_ \\ / _ \\ | '__| |/ _\` | '_ \\| __| \\ \\ /\\ / / _\` | | | |
           | |_| | | |  __/ | |  | | (_| | | | | |_   \\ V  V / (_| | |_| |
            \\__|_| |_|\\___| |_|  |_|\\__, |_| |_|\\__|   \\_/\\_/ \\__,_|\\__, |
                                    |___/                           |___/ 
                            by http://github.com/syarul/"
        end
        def hashCache()
            // this is done to get current location hash then update todo-list and footer
            set $initial to window.location.hash.slice(2).charAt(0).toUpperCase()
            set $after to window.location.hash.slice(3)
            fetch `/set-hash?name=${$initial}${$after}` then
              send show to <section.todoapp/>
              send toggleFooter to <section.todoapp/>
        end
        // this to handle popstate event such as back/forward button
        // where it will automatically calling hashCache _hyperscript function
        js
            window.addEventListener('popstate', function(){
                hashCache();
            });
        end
    "#,
    );
    html! {
        script src="https://unpkg.com/todomvc-common@1.0.5/base.js" {}
        script src="https://unpkg.com/htmx.org@1.9.10" {}
        script src="https://unpkg.com/hyperscript.org/dist/_hyperscript.js" {}
        script type="text/hyperscript" { (start_me_up) }
    }
}

pub fn page(
    title: &str,
    filters: &[Filter],
    todos: &[Todo],
    checked: bool,
    has_completed: bool,
    filter_name: &str,
) -> Markup {
    html! {
        (header(title))
        body {
            (todoapp(filters, todos, checked, has_completed, filter_name))
            (info())
            (scripts())
        }
    }
}
