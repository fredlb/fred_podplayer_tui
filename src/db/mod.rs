extern crate dotenv;

pub mod models;
pub mod schema;

use diesel::prelude::*;

use models::{Episode, NewEpisode, NewPod, Pod};

pub fn establish_connection() -> SqliteConnection {
    let database_url = "poddb.db";
    SqliteConnection::establish(&database_url).unwrap_or_else(|_| panic!("failed to connect to db"))
}

pub fn get_pods(conn: &mut SqliteConnection) -> Vec<Pod> {
    use schema::pods::dsl::pods;
    pods.load::<Pod>(conn).expect("failed to load pods")
}

pub fn get_pod(conn: &mut SqliteConnection, pod_id: i32) -> Pod {
    use schema::pods::dsl::*;
    let pod: Pod = pods
        .find(pod_id)
        .first(conn)
        .unwrap_or_else(|_| panic!("aaaaa"));
    return pod;
}

pub fn create_pod(conn: &mut SqliteConnection, title: &str, url: &str) -> usize {
    use schema::pods;
    let new_pod = NewPod { title, url };

    diesel::insert_into(pods::table)
        .values(&new_pod)
        .execute(conn)
        .expect("error saving pod")
}

pub fn mark_pod_as_downloaded(conn: &mut SqliteConnection, pod_id: i32) {
    use schema::pods;
    use schema::pods::dsl::*;
    let _ = diesel::update(pods.find(pod_id))
        .set(pods::downloaded.eq(true))
        .execute(conn);
}

pub fn create_episode(
    conn: &mut SqliteConnection,
    uid: &str,
    pod_id: i32,
    title: &str,
    url: &str,
    audio_url: &str,
    description: &str,
    pub_timestamp: i32,
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
        pub_timestamp,
        duration: None,
    };

    diesel::insert_into(episodes::table)
        .values(&new_episode)
        .execute(conn)
        .expect("error saving episode")
}

pub fn get_episodes_for_pod(conn: &mut SqliteConnection, pod_id_x: i32) -> Vec<Episode> {
    use schema::episodes::dsl::*;
    let eps = episodes
        .filter(pod_id.eq(pod_id_x))
        .order(pub_timestamp.desc())
        .load::<Episode>(conn)
        .expect("failed to fetch episodes");
    return eps;
}

pub fn get_episode(conn: &mut SqliteConnection, ep_id: i32) -> Episode {
    use schema::episodes::dsl::*;
    let ep: Episode = episodes
        .find(ep_id)
        .first(conn)
        .unwrap_or_else(|_| panic!("aaaaa"));
    return ep;
}

pub fn mark_episode_as_downloaded(
    conn: &mut SqliteConnection,
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

pub fn set_timestamp_on_episode(conn: &mut SqliteConnection, episode_id: i32, ts: f32) -> Episode {
    use schema::episodes;
    use schema::episodes::dsl::*;
    let _ = diesel::update(episodes.find(episode_id))
        .set(episodes::timestamp.eq(ts))
        .execute(conn);
    return episodes
        .find(episode_id)
        .first(conn)
        .unwrap_or_else(|_| panic!("aaaaa"));
}

pub fn delete_pod(conn: &mut SqliteConnection, pod_id_to_delete: i32) {
    use schema::episodes::dsl::*;
    use schema::pods::dsl::*;
    let _ = diesel::delete(episodes.filter(pod_id.eq(pod_id_to_delete))).execute(conn);
    let _ = diesel::delete(pods.find(pod_id_to_delete)).execute(conn);
}
