
     ooooo   ooooo ooooooooooooo ooo        ooooo ooooooo  ooooo 
     `888'   `888' 8'   888   `8 `88.       .888'  `8888    d8'  
      888     888       888       888b     d'888     Y888..8P    
      888ooooo888       888       8 Y88. .P  888      `8888'     
      888     888       888       8  `888'   888     .8PY888.    
      888     888       888       8    Y     888    d8'  `888b   
     o888o   o888o     o888o     o8o        o888o o888o  o88888o
    ===========================================================
         Build with RUST, ASTRA, MAUD, HTMX & _HYPERSCRIPT

[![Rust](https://github.com/syarul/todomvc-rust-astra-maud-htmx-_hyperscript/actions/workflows/rust.yml/badge.svg)](https://github.com/syarul/todomvc-rust-astra-maud-htmx-_hyperscript/actions/workflows/rust.yml)
### Use
- [Rust](https://www.rust-lang.org/) - check out why is the most [loved](https://survey.stackoverflow.co/2023/#programming-scripting-and-markup-languages) language.
- [astra](https://github.com/ibraheemdev/astra) - A blocking HTTP server built on top of [hyper](https://github.com/hyperium/hyper)
- [maud](https://github.com/lambda-fairy/maud) - Maud is an HTML template engine for Rust
- [htmx](https://htmx.org/) - HATEOS
- [_hyperscript](https://hyperscript.org/) - Why you do not need to code front-end anymore

### Usage
- install `Rust` if you don't have
- run `cargo build`
- run `cargo run`
- visit [http://localhost:8888/](http://localhost:8888/)
- if you need to run the e2e testing make sure to have nodejs installed
- run in the root folder since the Rust server will pick a static asset needed for covered test
- do `git clone https://github.com/cypress-io/cypress-example-todomvc`
- `cd cypress-example-todomvc`
- `npm install`
- if you need to see the test in browser run `npm run cypress:open`
- for headless test `npm run cypress:run`

https://github.com/syarul/todomvc-rust-astra-maud-htmx-_hyperscript/assets/2774594/932b7475-2ddd-48e8-8224-79be69775cac

### Concept
3 different ownership concepts with **Atomic**, **Mutex** and **RwLock** wrap with **Arc** to show how to operate handling the todos in multi threads
- `Atomic` for the counter, specifically `AtomicU32` unassigned 32-bit integer. the counter will goes up when new todo is inserted to the todos vector.
- `Mutex` is use to store the todos, with the locking mechanism in place ensure the changes to the todos will be handled correctly on multi thread ops.
- `RwLock` is used to handle the filter (tab link #/all #/active #/completed), since the length is never changed with only selected parameter changed when pages is click, it save to do read/write operations.

### HTMX
Visit [https://github.com/rajasegar/awesome-htmx](https://github.com/rajasegar/awesome-htmx) to look for HTMX curated infos

### Todo
- Perf test (consolidate with other langs rust, zig, odin, ocaml, etc+)
