use tantivy::schema::*;

/// Holds field handles for the engram Tantivy schema.
pub struct EngramSchema {
    pub schema: Schema,
    pub id: Field,
    pub intent_request: Field,
    pub intent_summary: Field,
    pub transcript_text: Field,
    pub agent_name: Field,
    pub agent_model: Field,
    pub created_at: Field,
    pub file_paths: Field,
    pub dead_ends: Field,
    pub cost_usd: Field,
    pub total_tokens: Field,
    pub manifest_json: Field,
}

impl EngramSchema {
    pub fn new() -> Self {
        let mut builder = Schema::builder();

        let id = builder.add_text_field("id", STRING | STORED);
        let intent_request = builder.add_text_field("intent_request", TEXT | STORED);
        let intent_summary = builder.add_text_field("intent_summary", TEXT | STORED);
        let transcript_text = builder.add_text_field("transcript_text", TEXT);
        let agent_name = builder.add_text_field("agent_name", STRING | STORED);
        let agent_model = builder.add_text_field("agent_model", STRING | STORED);
        let created_at = builder.add_date_field("created_at", INDEXED | STORED);
        let file_paths = builder.add_text_field("file_paths", TEXT | STORED);
        let dead_ends = builder.add_text_field("dead_ends", TEXT | STORED);
        let cost_usd = builder.add_f64_field("cost_usd", INDEXED | STORED);
        let total_tokens = builder.add_u64_field("total_tokens", INDEXED | STORED);
        let manifest_json = builder.add_text_field("manifest_json", STORED);

        let schema = builder.build();

        Self {
            schema,
            id,
            intent_request,
            intent_summary,
            transcript_text,
            agent_name,
            agent_model,
            created_at,
            file_paths,
            dead_ends,
            cost_usd,
            total_tokens,
            manifest_json,
        }
    }
}

impl Default for EngramSchema {
    fn default() -> Self {
        Self::new()
    }
}
