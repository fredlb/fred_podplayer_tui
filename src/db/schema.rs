// @generated automatically by Diesel CLI.

diesel::table! {
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
        pub_timestamp -> Integer,
        duration -> Nullable<Integer>,
    }
}

diesel::table! {
    pods (id) {
        id -> Integer,
        title -> Text,
        url -> Text,
        downloaded -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    episodes,
    pods,
);
