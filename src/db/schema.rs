table! {
    episodes (id) {
        id -> Integer,
        uid -> Text,
        title -> Text,
        url -> Text,
        audio_url -> Text,
        description -> Text,
        audio_filepath -> Text,
        downloaded -> Bool,
        pod_id -> Integer,
    }
}

table! {
    pods (id) {
        id -> Integer,
        title -> Text,
        url -> Text,
        downloaded -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    episodes,
    pods,
);
