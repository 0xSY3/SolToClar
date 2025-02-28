#[derive(Debug)]
pub struct Contract {
    pub name: String,
    pub functions: Vec<Function>,
    pub state_variables: Vec<StateVariable>,
    pub events: Vec<Event>,
    pub constructor: Option<Constructor>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub visibility: Option<String>,
    pub mutability: Option<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub struct Constructor {
    pub params: Vec<Parameter>,
    pub visibility: Option<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug)]
pub struct StateVariable {
    pub name: String,
    pub var_type: String,
    pub visibility: Option<String>,
    pub is_mapping: bool,
    pub mapping_key_type: Option<String>,
    pub mapping_value_type: Option<String>,
    pub is_constant: bool,
    pub initial_value: Option<Expression>,
    pub nested_mapping: Option<Box<MappingType>>,
}

#[derive(Debug)]
pub struct MappingType {
    pub key_type: String,
    pub value_type: String,
    pub nested: Option<Box<MappingType>>,
}

#[derive(Debug)]
pub struct Event {
    pub name: String,
    pub params: Vec<EventParameter>,
}

#[derive(Debug)]
pub struct EventParameter {
    pub name: String,
    pub param_type: String,
    pub indexed: bool,
}

#[derive(Debug)]
pub enum Statement {
    Expression(Expression),
    Return(Expression),
    Assignment(String, Expression),
    MapAccessAssignment(String, Box<Expression>, Expression),
    Emit(String, Vec<Expression>),
}

#[derive(Debug)]
pub enum Expression {
    Literal(String),
    Identifier(String),
    BinaryOp(Box<Expression>, String, Box<Expression>),
    MapAccess(String, Box<Expression>),
    MemberAccess(Box<Expression>, String),
}