use rmcp::model::JsonObject;

pub trait McpToolSchemaMapper {
    fn get_schema(&self) -> McpToolSchema;
}


pub struct McpToolSchema {
    pub input_schema: JsonObject,
    pub output_schema: Option<JsonObject>,
}