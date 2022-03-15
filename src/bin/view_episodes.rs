extern crate diesel;
extern crate fred_podplayer_tui;

use fred_podplayer_tui::db::create_pod;
use fred_podplayer_tui::db::get_episodes;
use fred_podplayer_tui::db::establish_connection;
use fred_podplayer_tui::db::models::*;

use diesel::prelude::*;
use std::io::{stdin, Read};

fn main() {
    let connection = establish_connection();
    let episodes = get_episodes(&connection);
    for ep in episodes {
        println!("{:?}", ep);
    }
}
