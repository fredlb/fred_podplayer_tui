extern crate dotenv;

pub mod models;
pub mod schema;

use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

use models::{Episode, NewEpisode, NewPod, Pod};

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("database url not set");
    SqliteConnection::establish(&database_url).unwrap_or_else(|_| panic!("failed to connect to db"))
}

pub fn get_pods(conn: &SqliteConnection) -> Vec<Pod> {
    use schema::pods::dsl::pods;
    pods.load::<Pod>(conn).expect("failed to load pods")
}

pub fn create_pod(conn: &SqliteConnection, title: &str, url: &str) -> usize {
    use schema::pods;
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

pub fn mark_pod_as_downloaded(conn: &SqliteConnection, pod_id: i32) {
    use schema::pods;
    use schema::pods::dsl::*;
    let _ = diesel::update(pods.find(pod_id))
        .set(pods::downloaded.eq(true))
        .execute(conn);
}

pub fn create_episode(
    conn: &SqliteConnection,
    uid: &str,
    pod_id: i32,
    title: &str,
    url: &str,
    audio_url: &str,
    description: &str,
    downloaded: bool,
) -> usize {
    use schema::episodes;
    let new_episode = NewEpisode {uid, pod_id, title, url, audio_url, description, downloaded, audio_filepath: None };

    diesel::insert_into(episodes::table)
        .values(&new_episode)
        .execute(conn)
        .expect("error saving episode")
}
