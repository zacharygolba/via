table! {
    posts (id) {
        id -> Int4,
        body -> Text,
        title -> Varchar,
        user_id -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        published_at -> Nullable<Timestamp>,
    }
}

table! {
    users (id) {
        id -> Int4,
        username -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

joinable!(posts -> users (user_id));
allow_tables_to_appear_in_same_query!(posts, users,);
