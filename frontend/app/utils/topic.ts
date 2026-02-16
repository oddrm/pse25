export interface Topic {
    id: number;
    entry_id: number;
    topic_name: string;
    topic_type: string | null;
    message_count: number;
    frequency: number | null;
    created_at: string;
    updated_at: string;
}

export interface TopicWeb {
    topic_name: string;
    topic_type: string | null;
    message_count: number;
    frequency: number | null;
}
