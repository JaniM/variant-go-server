table! {
    games (id) {
        id -> Int8,
        name -> Text,
        replay -> Nullable<Bytea>,
    }
}

table! {
    users (id) {
        id -> Int8,
        auth_token -> Text,
        nick -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(
    games,
    users,
);
