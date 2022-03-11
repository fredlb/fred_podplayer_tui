extern crate dotenv;

pub mod models;
pub mod schema;

use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

use models::{NewPod, Pod};
use schema::pods;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("database url not set");
    SqliteConnection::establish(&database_url).unwrap_or_else(|_| panic!("failed to connect to db"))
}

pub fn get_pods(conn: &SqliteConnection) -> Vec<Pod> {
    use schema::pods::dsl::*;
    pods.load::<Pod>(conn).expect("failed to load pods")
}

pub fn create_pod(conn: &SqliteConnection, title: &str, url: &str) -> usize {
    let new_pod = NewPod { title, url };

    diesel::insert_into(pods::table)
        .values(&new_pod)
        .execute(conn)
        .expect("error saving pod")
}

pub fn delete_pod(conn: &SqliteConnection, title_to_delete: &str) {
    use schema::pods::dsl::*;
    let _ = diesel::delete(pods.filter(title.like(format!("%{}%", title_to_delete))))
        .execute(conn)
        .expect(format!("failed to delete pod(s): {}", title_to_delete).as_str());
}
