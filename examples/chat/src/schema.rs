// @generated automatically by Diesel CLI.

diesel::table! {
    messages (id) {
        id -> Uuid,
        author_id -> Uuid,
        thread_id -> Uuid,
        body -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        reactions_count -> Int8,
    }
}

diesel::table! {
    reactions (id) {
        id -> Uuid,
        #[max_length = 16]
        emoji -> Varchar,
        message_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    subscriptions (id) {
        id -> Uuid,
        user_id -> Uuid,
        thread_id -> Uuid,
        claims -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    threads (id) {
        id -> Uuid,
        name -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        email -> Text,
        username -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(messages -> threads (thread_id));
diesel::joinable!(messages -> users (author_id));
diesel::joinable!(reactions -> messages (message_id));
diesel::joinable!(reactions -> users (user_id));
diesel::joinable!(subscriptions -> threads (thread_id));
diesel::joinable!(subscriptions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(messages, reactions, subscriptions, threads, users,);
