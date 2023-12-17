// @generated automatically by Diesel CLI.

diesel::table! {
    games (id) {
        id -> Int8,
        name -> Text,
        replay -> Nullable<Bytea>,
        owner -> Nullable<Int8>,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        auth_token -> Text,
        nick -> Nullable<Text>,
        has_integration_access -> Bool,
    }
}

diesel::joinable!(games -> users (owner));

diesel::allow_tables_to_appear_in_same_query!(
    games,
    users,
);
