use super::ast::*;
use anyhow::Result;

#[derive(Debug)]
pub struct ClarityContract {
    pub name: String,
    pub functions: Vec<ClarityFunction>,
    pub data_vars: Vec<ClarityDataVar>,
    pub maps: Vec<ClarityMap>,
    pub events: Vec<ClarityEvent>,
}

#[derive(Debug)]
pub struct ClarityFunction {
    pub name: String,
    pub params: Vec<ClarityParameter>,
    pub public: bool,
    pub read_only: bool,
    pub body: Vec<ClarityExpression>,
}

#[derive(Debug)]
pub struct ClarityParameter {
    pub name: String,
    pub param_type: String,
}

#[derive(Debug)]
pub struct ClarityDataVar {
    pub name: String,
    pub var_type: String,
    pub initial_value: String,
    pub is_constant: bool,
    pub visibility: Option<String>,
}

#[derive(Debug)]
pub struct ClarityMap {
    pub name: String,
    pub key_type: String,
    pub value_type: String,
}

#[derive(Debug)]
pub struct ClarityEvent {
    pub name: String,
    pub fields: Vec<ClarityEventField>,
}

#[derive(Debug)]
pub struct ClarityEventField {
    pub name: String,
    pub field_type: String,
    pub indexed: bool,
}

#[derive(Debug)]
pub enum ClarityExpression {
    Literal(String),
    Var(String),
    FunctionCall(String, Vec<ClarityExpression>),
    MapGet(String, Vec<ClarityExpression>),
    MapSet(String, Vec<ClarityExpression>, Box<ClarityExpression>),
    Print(Vec<ClarityExpression>),
}

fn convert_nested_mapping_type(mapping: &MappingType) -> (String, String) {
    if let Some(nested) = &mapping.nested {
        let (nested_key_type, nested_value_type) = convert_nested_mapping_type(nested);
        (
            format!("{{owner: {}, token-id: {}}}", 
                convert_solidity_type(&mapping.key_type),
                nested_key_type
            ),
            nested_value_type
        )
    } else {
        (
            convert_solidity_type(&mapping.key_type),
            convert_solidity_type(&mapping.value_type)
        )
    }
}

pub fn convert_solidity_type(solidity_type: &str) -> String {
    match solidity_type {
        "uint256" | "uint" => "uint".to_string(),
        "bool" => "bool".to_string(),
        "address" => "principal".to_string(),
        "string" => "string-ascii".to_string(),
        _ => {
            if solidity_type.starts_with("mapping") {
                solidity_type.to_string()
            } else {
                "uint".to_string()
            }
        }
    }
}

fn convert_mapping(var: &StateVariable) -> Result<ClarityMap> {
    if let Some(nested) = &var.nested_mapping {
        let (key_type, value_type) = convert_nested_mapping_type(nested);
        Ok(ClarityMap {
            name: var.name.clone(),
            key_type,
            value_type,
        })
    } else {
        Ok(ClarityMap {
            name: var.name.clone(),
            key_type: convert_solidity_type(&var.mapping_key_type.clone().unwrap()),
            value_type: convert_solidity_type(&var.mapping_value_type.clone().unwrap()),
        })
    }
}

pub fn convert_contract(contract: Contract) -> Result<ClarityContract> {
    let mut clarity_contract = ClarityContract {
        name: contract.name,
        functions: Vec::new(),
        data_vars: Vec::new(),
        maps: Vec::new(),
        events: Vec::new(),
    };

    for var in contract.state_variables {
        if var.is_mapping {
            clarity_contract.maps.push(convert_mapping(&var)?);
        } else {
            clarity_contract.data_vars.push(convert_state_variable(var));
        }
    }

    for event in contract.events {
        clarity_contract.events.push(ClarityEvent {
            name: event.name,
            fields: event.params.into_iter()
                .map(|p| ClarityEventField {
                    name: p.name,
                    field_type: convert_solidity_type(&p.param_type),
                    indexed: p.indexed,
                })
                .collect(),
        });
    }

    if let Some(constructor) = contract.constructor {
        clarity_contract.functions.push(ClarityFunction {
            name: "init".to_string(),
            params: constructor.params.into_iter()
                .map(|p| ClarityParameter {
                    name: p.name,
                    param_type: convert_solidity_type(&p.param_type),
                })
                .collect(),
            public: true,
            read_only: false,
            body: convert_statements(constructor.body)?,
        });
    }

    for func in contract.functions {
        clarity_contract.functions.push(convert_function(func)?);
    }

    Ok(clarity_contract)
}

fn convert_state_variable(var: StateVariable) -> ClarityDataVar {
    let var_type = convert_solidity_type(&var.var_type);
    let initial_value = if let Some(expr) = var.initial_value {
        match expr {
            Expression::Literal(val) => {
                if var_type == "uint" && val.chars().all(|c| c.is_digit(10)) {
                    format!("u{}", val)
                } else {
                    val
                }
            },
            _ => match var_type.as_str() {
                "uint" => "u0".to_string(),
                "bool" => "false".to_string(),
                "principal" => "tx-sender".to_string(),
                "string-ascii" => "\"\"".to_string(),
                _ => "u0".to_string(),
            }
        }
    } else {
        match var_type.as_str() {
            "uint" => "u0".to_string(),
            "bool" => "false".to_string(),
            "principal" => "tx-sender".to_string(),
            "string-ascii" => "\"\"".to_string(),
            _ => "u0".to_string(),
        }
    };

    ClarityDataVar {
        name: var.name,
        var_type,
        initial_value,
        is_constant: var.is_constant,
        visibility: var.visibility,
    }
}

pub fn convert_function(func: Function) -> Result<ClarityFunction> {
    Ok(ClarityFunction {
        name: func.name,
        params: func.params.into_iter()
            .map(|p| ClarityParameter {
                name: p.name,
                param_type: convert_solidity_type(&p.param_type),
            })
            .collect(),
        public: func.visibility.as_ref().map_or(false, |v| v == "public" || v == "external"),
        read_only: func.mutability.as_ref().map_or(false, |m| m == "view" || m == "pure"),
        body: convert_statements(func.body)?,
    })
}

fn convert_statements(statements: Vec<Statement>) -> Result<Vec<ClarityExpression>> {
    let mut clarity_statements = Vec::new();

    for stmt in statements {
        match stmt {
            Statement::Expression(expr) => {
                clarity_statements.push(convert_expression(expr));
            }
            Statement::Return(expr) => {
                clarity_statements.push(convert_expression(expr));
            }
            Statement::Assignment(var_name, expr) => {
                clarity_statements.push(ClarityExpression::FunctionCall(
                    "var-set".to_string(),
                    vec![
                        ClarityExpression::Var(var_name),
                        convert_expression(expr)
                    ]
                ));
            }
            Statement::MapAccessAssignment(map_name, key, value) => {
                clarity_statements.push(ClarityExpression::MapSet(
                    map_name,
                    vec![convert_expression(*key)],
                    Box::new(convert_expression(value))
                ));
            }
            Statement::Emit(event_name, args) => {
                let mut print_args = vec![ClarityExpression::Literal(format!("\"{}\"", event_name))];
                print_args.extend(args.into_iter().map(convert_expression));
                clarity_statements.push(ClarityExpression::Print(print_args));
            }
        }
    }

    Ok(clarity_statements)
}

fn convert_expression(expr: Expression) -> ClarityExpression {
    match expr {
        Expression::Literal(val) => {
            if val == "true" {
                ClarityExpression::Literal("true".to_string())
            } else if val == "false" {
                ClarityExpression::Literal("false".to_string())
            } else if val.chars().all(|c| c.is_digit(10)) {
                ClarityExpression::Literal(format!("u{}", val))
            } else {
                ClarityExpression::Literal(val)
            }
        }
        Expression::Identifier(name) => {
            ClarityExpression::FunctionCall(
                "var-get".to_string(),
                vec![ClarityExpression::Var(name)]
            )
        }
        Expression::BinaryOp(left, op, right) => {
            match op.as_str() {
                "," => {
                    ClarityExpression::FunctionCall(
                        "tuple".to_string(),
                        vec![convert_expression(*left), convert_expression(*right)]
                    )
                }
                _ => ClarityExpression::FunctionCall(
                    op,
                    vec![convert_expression(*left), convert_expression(*right)]
                )
            }
        }
        Expression::MapAccess(map_name, key) => {
            ClarityExpression::MapGet(
                map_name,
                vec![convert_expression(*key)]
            )
        }
        Expression::MemberAccess(expr, member) => {
            if let Expression::Identifier(name) = *expr {
                if name == "msg" && member == "sender" {
                    ClarityExpression::Var("tx-sender".to_string())
                } else {
                    ClarityExpression::Var(format!("{}-{}", name, member))
                }
            } else {
                ClarityExpression::Var(format!("{}-{}", expr, member))
            }
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Identifier(name) => write!(f, "{}", name),
            Expression::Literal(val) => write!(f, "{}", val),
            Expression::BinaryOp(left, op, right) => write!(f, "({} {} {})", left, op, right),
            Expression::MapAccess(map, key) => write!(f, "{}[{}]", map, key),
            Expression::MemberAccess(expr, member) => write!(f, "{}.{}", expr, member),
        }
    }
}