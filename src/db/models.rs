use super::schema::{episodes, pods};

#[derive(Identifiable, Queryable, PartialEq, Debug, Clone)]
#[table_name = "pods"]
pub struct Pod {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub downloaded: bool,
}

#[derive(Insertable)]
#[table_name = "pods"]
pub struct NewPod<'a> {
    pub title: &'a str,
    pub url: &'a str,
}

#[derive(Clone, Queryable, Identifiable, Associations, PartialEq, Debug)]
#[belongs_to(Pod)]
#[table_name = "episodes"]
pub struct Episode {
    pub id: i32,
    pub uid: String,
    pub pod_id: i32,
    pub title: String,
    pub url: String,
    pub audio_url: String,
    pub description: String,
    pub audio_filepath: Option<String>,
    pub downloaded: bool,
    pub played: bool,
    pub timestamp: f32,
    pub pub_timestamp: i32,
    pub duration: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "episodes"]
pub struct NewEpisode<'a> {
    pub uid: &'a str,
    pub pod_id: i32,
    pub title: &'a str,
    pub url: &'a str,
    pub audio_url: &'a str,
    pub description: &'a str,
    pub audio_filepath: Option<&'a str>,
    pub downloaded: bool,
    pub played: bool,
    pub timestamp: f32,
    pub pub_timestamp: i32,
    pub duration: Option<i32>,
}
