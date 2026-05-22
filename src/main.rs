mod app;
mod components;
mod csv_utils;
mod models;
mod seed;
mod storage;
mod sync;

use app::App;

fn main() {
    leptos::mount::mount_to_body(App);
}
