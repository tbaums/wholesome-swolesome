mod app;
mod components;
mod csv_utils;
mod models;
mod seed;
mod storage;

use app::App;

fn main() {
    leptos::mount::mount_to_body(App);
}
