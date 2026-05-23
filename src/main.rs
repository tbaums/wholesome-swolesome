mod app;
mod coach;
mod components;
mod csv_utils;
mod library;
mod models;
mod storage;
mod sync;

use app::App;

fn main() {
    leptos::mount::mount_to_body(App);
}
