// @generated automatically by Diesel CLI.

diesel::table! {
    messages (id) {
        id -> Uuid,
        body -> Text,
        author_id -> Uuid,
        thread_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    reactions (id) {
        id -> Uuid,
        #[max_length = 16]
        emoji -> Varchar,
        message_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    threads (id) {
        id -> Uuid,
        name -> Text,
        owner_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        email -> Text,
        username -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(messages -> threads (thread_id));
diesel::joinable!(messages -> users (author_id));
diesel::joinable!(reactions -> messages (message_id));
diesel::joinable!(reactions -> users (user_id));
diesel::joinable!(threads -> users (owner_id));

diesel::allow_tables_to_appear_in_same_query!(messages, reactions, threads, users,);
