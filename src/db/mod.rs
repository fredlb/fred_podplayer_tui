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

pub fn get_pod(conn: &SqliteConnection, pod_id: i32) -> Pod {
    use schema::pods::dsl::*;
    let pod: Pod = pods
        .find(pod_id)
        .first(conn)
        .unwrap_or_else(|_| panic!("aaaaa"));
    return pod;
}

pub fn create_pod(conn: &SqliteConnection, title: &str, url: &str) -> usize {
    use schema::pods;
    let new_pod = NewPod { title, url };

    diesel::insert_into(pods::table)
        .values(&new_pod)
        .execute(conn)
        .expect("error saving pod")
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
    let new_episode = NewEpisode {
        uid,
        pod_id,
        title,
        url,
        audio_url,
        description,
        downloaded,
        audio_filepath: None,
        played: false,
        timestamp: 0.0,
        duration: None,
    };

    diesel::insert_into(episodes::table)
        .values(&new_episode)
        .execute(conn)
        .expect("error saving episode")
}

pub fn get_episodes_for_pod(conn: &SqliteConnection, pod_id_x: i32) -> Vec<Episode> {
    use schema::episodes::dsl::*;
    let eps = episodes
        .filter(pod_id.eq(pod_id_x))
        .load::<Episode>(conn)
        .expect("failed to fetch episodes");
    return eps;
}

pub fn get_episode(conn: &SqliteConnection, ep_id: i32) -> Episode {
    use schema::episodes::dsl::*;
    let ep: Episode = episodes
        .find(ep_id)
        .first(conn)
        .unwrap_or_else(|_| panic!("aaaaa"));
    return ep;
}

pub fn mark_episode_as_downloaded(
    conn: &SqliteConnection,
    episode: &Episode,
    filepath: &String,
    ep_duration: i32,
) -> Episode {
    use schema::episodes;
    use schema::episodes::dsl::*;
    let _ = diesel::update(episodes.find(episode.id))
        .set((
            episodes::downloaded.eq(true),
            episodes::audio_filepath.eq(filepath),
            episodes::duration.eq(ep_duration),
        ))
        .execute(conn);
    let updated_ep: Episode = episodes
        .find(episode.id)
        .first(conn)
        .unwrap_or_else(|_| panic!("aaaaa"));
    return updated_ep;
}

pub fn set_timestamp_on_episode(conn: &SqliteConnection, episode_id: i32, ts: f32) {
    use schema::episodes;
    use schema::episodes::dsl::*;
    let _ = diesel::update(episodes.find(episode_id))
        .set(episodes::timestamp.eq(ts))
        .execute(conn);
}
