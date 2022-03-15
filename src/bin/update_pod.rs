
extern crate diesel;
extern crate fred_podplayer_tui;

use fred_podplayer_tui::db::create_pod;
use fred_podplayer_tui::db::get_pods;
use fred_podplayer_tui::db::mark_pod_as_downloaded;
use fred_podplayer_tui::db::establish_connection;
use fred_podplayer_tui::db::models::*;

use diesel::prelude::*;
use std::io::{stdin, Read};

fn main() {
    let connection = establish_connection();
    let podddds = get_pods(&connection);
    for pod in &podddds {
        println!("{} {} {}", pod.title, pod.url, pod.downloaded);
    }
    mark_pod_as_downloaded(&connection, podddds.first().unwrap().id);
    let podddds2 = get_pods(&connection);
    for pod in podddds2 {
        println!("{} {} {}", pod.title, pod.url, pod.downloaded);
    }
}
