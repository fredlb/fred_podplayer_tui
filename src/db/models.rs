use diesel::prelude::*;

#[derive(Identifiable, Queryable, PartialEq, Debug, Clone)]
#[diesel(table_name = crate::db::schema::pods)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Pod {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub downloaded: bool,
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::pods)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPod<'a> {
    pub title: &'a str,
    pub url: &'a str,
}

#[derive(Clone, Queryable, Identifiable, Associations, PartialEq, Debug)]
#[diesel(belongs_to(Pod))]
#[diesel(table_name = crate::db::schema::episodes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
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
#[diesel(table_name = crate::db::schema::episodes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
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
