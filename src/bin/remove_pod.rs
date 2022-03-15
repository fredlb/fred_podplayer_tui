extern crate diesel;
extern crate fred_podplayer_tui;

use fred_podplayer_tui::db::delete_pod;
use fred_podplayer_tui::db::establish_connection;
use fred_podplayer_tui::db::models::*;

use diesel::prelude::*;
use std::io::{stdin, Read};

fn main() {
    let connection = establish_connection();

    println!("Name of pod?");
    let mut title = String::new();
    stdin().read_line(&mut title).unwrap();
    let title = &title[..(title.len() - 1)];

    let _ = delete_pod(&connection, title);
    println!("\nDeleted pod {}", title);
}
