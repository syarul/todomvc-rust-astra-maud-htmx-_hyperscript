use crate::{Filter, Todo};
use maud::{html, Markup, PreEscaped, DOCTYPE};

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
                on click send toggle to <input.toggle/>" {}
    }
}

pub fn filter_bar(filters: &[Filter]) -> Markup {
    html! {
        ul class="filters" {
            @for filter in filters {
                li {
                    a
                        class={ @if filter.selected { "selected" } }
                        href={ (filter.url) }
                        hx-get={ "/get-hash?name="(filter.name) }
                        hx-trigger="click"
                        hx-target=".filters"
                        hx-swap="outerHTML"
                        _="on htmx:afterRequest send show to <li.todo/>"
                        { (filter.name) }
                }
            }
        }
    }
}

fn todo_check(todo: &Todo) -> Markup {
    let toggle = todo.done == true;
    html! {
        input
            class="toggle"
            type="checkbox"
            checked[toggle]
            hx-patch={ "/toggle-todo?id="(todo.id) }
            hx-target="closest <li/>"
            hx-swap="outerHTML"
            _="
            on toggle
                if $toggleAll.checked and my.checked === false
                my.click()
                else if $toggleAll.checked === false and my.checked
                my.click()" {}
    }
}

pub fn edit_todo(todo: &Todo) -> Markup {
    let value = if todo.editing { &todo.task } else { "" };
    html! {
        input
            class="edit"
            name="task"
            value={ (value) }
            hx-trigger="keyup[keyCode==13], task"
            hx-get={ "/update-todo?id="(todo.id) }
            hx-target="closest <li/>"
            hx-swap="outerHTML"
            autofocus
            _="
                on keyup[keyCode==27] remove .editing from closest <li/>
                on htmx:afterRequest send focus to <input.new-todo/>" {}
    }
}

pub fn todo_item(todo: &Todo) -> Markup {
    html! {
        li
            id={ (todo.id) }
            class={
                "todo "
                @if todo.done { "completed " }
                @if todo.editing { "editing" }
            }
            _="
            on destroy my.querySelector('button').click()
            on show wait 20ms
                if window.location.hash === '#/active' and my.classList.contains('completed')
                set my.style.display to 'none'
                else if window.location.hash === '#/completed' and my.classList.contains('completed') === false
                set my.style.display to 'none'
                else
                set my.style.display to 'block'" {
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
                        set $el.selectionStart to $el.value.length" { 
                    (todo.task)
                }
                button
                    class="destroy"
                    hx-delete={ "/remove-todo?id="(todo.id) }
                    hx-trigger="click"
                    hx-target="closest <li/>"
                    _="
                    on htmx:afterRequest 
                        send toggleDisplayClearCompleted to <button.clear-completed/>
                        send todoCount to <span.todo-count/>
                        send toggleAll to <input.toggle-all/>
                        send footerToggleDisplay to <footer.footer/>
                        send labelToggleAll to <label/>
                        send focus to <input.new-todo/>" {}
            }
            (edit_todo(todo))
        }
    }
}

fn todoapp(filters: &[Filter], todos: &[Todo], checked: bool) -> Markup {
    html! {
        body {
            section
                class="todoapp"
                hx-get="/get-hash"
                hx-vals="js:{hash: window.location.hash}"
                hx-trigger="load"
                hx-target=".filters"
                hx-swap="outerHTML"
                {
                    header class="header" {
                        h1 { "todos" }
                        input
                            id="add-todo"
                            name="task"
                            class="new-todo"
                            placeholder="What needs to be done?"
                            hx-get="/add-todo"
                            hx-trigger="keyup[keyCode==13], task"
                            hx-target=".todo-list"
                            hx-swap="beforeend"
                            autofocus
                            _="
                                on htmx:afterRequest set my value to ''
                                on focus my.focus()" {}
                    }
                    section
                        class="main" {
                            { (toggle_all(checked)) }
                            label
                                for="toggle-all"
                                _="
                                on load send labelToggleAll to me
                                on labelToggleAll debounced at 100ms
                                    if $todo.hasChildNodes() set my.style.display to 'flex'
                                    else set my.style.display to 'none'"
                                style="display:none;" {
                                "Mark all as complete"
                            }
                        }
                    ul
                        class="todo-list"
                        _="
                            on load debounced at 10ms 
                            set $todo to me
                            send toggleDisplayClearCompleted to <button.clear-completed/>
                            send footerToggleDisplay to <footer.footer/>
                            send todoCount to <span.todo-count/>
                            send toggleAll to <input.toggle-all/>
                            send footerToggleDisplay to <footer.footer/>
                            send labelToggleAll to <label/>
                            send show to <li.todo/>" {
                        @for todo in todos {
                            { (todo_item(todo)) }
                        }
                    }
                    footer
                        class="footer"
                        _="
                            on load send footerToggleDisplay to me
                            on footerToggleDisplay debounced at 100ms
                            if $todo.hasChildNodes() set my.style.display to 'block'
                            else set my.style.display to 'none'
                            send focus to <input.new-todo/>"
                        style="display:none;" {
                        span
                            class="todo-count"
                            hx-trigger="load"
                            _="
                                on load send todoCount to me
                                on todoCount debounced at 100ms
                                fetch /update-counts then put the result into me" {}
                        (filter_bar(filters))
                        button
                            class="clear-completed"
                            _="
                            on load send toggleDisplayClearCompleted to me
                            on toggleDisplayClearCompleted debounced at 100ms
                                fetch /completed then
                                set my.style.display to it
                            end
                            on click send destroy to <li.completed/>" { 
                            "Clear Complete"
                        }
                    }
                }
        }
    }
}

fn footer() -> Markup {
    html! {
        footer class="info" _="on load debounced at 100ms call startMeUp()" {
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
    "#,
    );
    html! {
        script src="https://unpkg.com/todomvc-common@1.0.5/base.js" {}
        script src="https://unpkg.com/htmx.org@1.9.10" {}
        script src="https://unpkg.com/hyperscript.org/dist/_hyperscript.js" {}
        script type="text/hyperscript" { (start_me_up) }
    }
}

pub fn page(title: &str, filters: &[Filter], todos: &[Todo], checked: bool) -> Markup {
    html! {
        (header(title))
        body {
            (todoapp(filters, todos, checked))
            (footer())
            (scripts())
        }
    }
}
