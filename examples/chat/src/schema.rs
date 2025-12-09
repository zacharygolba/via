// @generated automatically by Diesel CLI.

diesel::table! {
    channels (id) {
        id -> Uuid,
        name -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    conversations (id) {
        id -> Uuid,
        channel_id -> Uuid,
        thread_id -> Nullable<Uuid>,
        user_id -> Uuid,
        body -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        total_reactions -> Int8,
        total_replies -> Int8,
    }
}

diesel::table! {
    reactions (id) {
        id -> Uuid,
        #[max_length = 16]
        emoji -> Varchar,
        conversation_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    subscriptions (id) {
        id -> Uuid,
        channel_id -> Uuid,
        user_id -> Uuid,
        claims -> Int4,
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

diesel::joinable!(conversations -> channels (channel_id));
diesel::joinable!(conversations -> users (user_id));
diesel::joinable!(reactions -> conversations (conversation_id));
diesel::joinable!(reactions -> users (user_id));
diesel::joinable!(subscriptions -> channels (channel_id));
diesel::joinable!(subscriptions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    channels,
    conversations,
    reactions,
    subscriptions,
    users,
);
