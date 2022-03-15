table! {
    episodes (id) {
        id -> Integer,
        uid -> Text,
        pod_id -> Integer,
        title -> Text,
        url -> Text,
        audio_url -> Text,
        description -> Text,
        audio_filepath -> Nullable<Text>,
        downloaded -> Bool,
        played -> Bool,
        timestamp -> Float,
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
