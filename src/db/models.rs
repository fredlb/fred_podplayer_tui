use super::schema::pods;

#[derive(Clone, Queryable)]
pub struct Pod {
    pub id: i32,
    pub title: String,
    pub url: String,
}

#[derive(Insertable)]
#[table_name="pods"]
pub struct NewPod<'a> {
    pub title: &'a str,
    pub url: &'a str,
}
