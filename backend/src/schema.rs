diesel::table! {
    files (path) {
        path -> Varchar,
        is_mcap -> Bool,
        is_custom_metadata -> Bool,
    }
}
diesel::table! {
    entries (id) {
        id -> BigInt,
        name -> Varchar,
        path -> Varchar,
        platform -> Varchar,
        size -> BigInt,
        start_time_ns -> Nullable<BigInt>,
        duration_ns -> Nullable<BigInt>,
        total_message_count -> Nullable<BigInt>,
        storage_identifier -> Nullable<Varchar>,
        compression_format -> Nullable<Varchar>,
        compression_mode -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
diesel::table! {
    topics (id) {
        id -> BigInt,
        entry_id -> BigInt,
        topic_name -> Varchar,
        message_count -> BigInt,
        type_ -> Nullable<Varchar>,
        type_description_hash -> Nullable<Varchar>,
        serialization_format -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    tags (id) {
        id -> BigInt,
        entry_id -> BigInt,
        name -> Varchar,
        created_at -> Timestamptz,
    }
}
diesel::table! {
    sequences (id) {
        id -> BigInt,
        entry_id -> BigInt,
        description -> Text,
        start_timestamp -> BigInt,
        end_timestamp -> BigInt,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    metadata (entry_id) {
        entry_id -> BigInt,
        metadata_json -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(tags -> entries (entry_id));
diesel::joinable!(metadata -> entries (entry_id));
diesel::joinable!(sequences -> entries (entry_id));
diesel::joinable!(topics -> entries (entry_id));

diesel::allow_tables_to_appear_in_same_query!(entries, metadata, sequences, tags, topics,);
