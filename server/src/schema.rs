table! {
    games (id) {
        id -> Int8,
        name -> Text,
        replay -> Nullable<Bytea>,
        owner -> Nullable<Int8>,
    }
}

table! {
    users (id) {
        id -> Int8,
        auth_token -> Text,
        nick -> Nullable<Text>,
    }
}

joinable!(games -> users (owner));

allow_tables_to_appear_in_same_query!(games, users,);
