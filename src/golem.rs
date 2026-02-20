// Over simplified golem

#[derive(Clone)]
pub struct AgentMethod {
    pub method_name: String,
    pub input_schema: DataSchema,
    pub output_schema: DataSchema,
}

pub type AgentId = String;

pub type AgentType = String;

pub type ParameterName = String;

#[derive(Clone)]
pub enum ElementSchema {
    String,
    U32,
    Bool,
}

pub type DataSchema = Vec<(ParameterName, ElementSchema)>;
