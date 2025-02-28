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
    pub return_type: Option<String>,
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
    pub visibility: String,
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
}

#[derive(Debug)]
pub enum ClarityExpression {
    Literal(String),
    Var(String),
    FunctionCall(String, Vec<ClarityExpression>),
    MapGet(String, Vec<ClarityExpression>),
    MapSet(String, Vec<ClarityExpression>, Box<ClarityExpression>),
    Print(Vec<ClarityExpression>),  // For event emission
}

pub fn convert_contract(contract: Contract) -> Result<ClarityContract> {
    let mut clarity_contract = ClarityContract {
        name: contract.name,
        functions: Vec::new(),
        data_vars: Vec::new(),
        maps: Vec::new(),
        events: Vec::new(),
    };

    // Convert state variables
    for var in contract.state_variables {
        if var.is_mapping {
            clarity_contract.maps.push(convert_mapping(var)?);
        } else {
            clarity_contract.data_vars.push(convert_state_variable(var));
        }
    }

    // Convert events
    for event in contract.events {
        clarity_contract.events.push(convert_event(event));
    }

    // Convert constructor if present
    if let Some(constructor) = contract.constructor {
        clarity_contract.functions.push(convert_constructor(constructor)?);
    }

    // Convert functions
    for func in contract.functions {
        clarity_contract.functions.push(convert_function(func)?);
    }

    Ok(clarity_contract)
}

fn convert_mapping(var: StateVariable) -> Result<ClarityMap> {
    Ok(ClarityMap {
        name: var.name,
        key_type: var.mapping_key_type.unwrap_or_else(|| "uint".to_string()),
        value_type: var.mapping_value_type.unwrap_or_else(|| "uint".to_string()),
    })
}

fn convert_event(event: Event) -> ClarityEvent {
    ClarityEvent {
        name: event.name,
        fields: event.params.into_iter()
            .map(|p| ClarityEventField {
                name: p.name,
                field_type: convert_type(&p.param_type),
            })
            .collect(),
    }
}

fn convert_constructor(constructor: Constructor) -> Result<ClarityFunction> {
    Ok(ClarityFunction {
        name: "init".to_string(),
        params: constructor.params.into_iter()
            .map(|p| ClarityParameter {
                name: p.name,
                param_type: convert_type(&p.param_type),
            })
            .collect(),
        return_type: None,
        public: true,
        read_only: false,
        body: convert_statements(constructor.body)?,
    })
}

pub fn convert_state_variable(var: StateVariable) -> ClarityDataVar {
    let var_type = convert_type(&var.var_type);
    let initial_value = if let Some(val) = var.initial_value {
        if var_type == "uint" && val.chars().all(|c| c.is_digit(10)) {
            format!("u{}", val)
        } else {
            val
        }
    } else {
        match var_type.as_str() {
            "uint" => "u0",
            "bool" => "false",
            "principal" => "tx-sender",
            "string-ascii" => "\"\"",
            _ => "u0",
        }.to_string()
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
    // Public and external functions are accessible from outside the contract
    let public = func.visibility.as_deref() == Some("public") ||
                 func.visibility.as_deref() == Some("external");

    // View and pure functions are read-only
    let read_only = func.mutability.as_deref() == Some("view") ||
                    func.mutability.as_deref() == Some("pure");

    Ok(ClarityFunction {
        name: func.name,
        params: func.params.into_iter()
            .map(|p| ClarityParameter {
                name: p.name,
                param_type: convert_type(&p.param_type),
            })
            .collect(),
        return_type: func.return_type.map(|t| convert_type(&t)),
        public,
        read_only,
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
            Statement::Emit(event_name, args) => {
                let mut print_args = vec![ClarityExpression::Literal(format!("\"{}\"", event_name))];
                print_args.extend(args.into_iter().map(convert_expression));
                clarity_statements.push(ClarityExpression::Print(print_args));
            }
        }
    }

    Ok(clarity_statements)
}

pub fn convert_type(solidity_type: &str) -> String {
    match solidity_type {
        "uint256" | "uint" => "uint".to_string(),
        "bool" => "bool".to_string(),
        "address" => "principal".to_string(),
        "string" => "string-ascii".to_string(),
        _ => "uint".to_string(), // Default to uint for unknown types
    }
}

fn convert_expression(expr: Expression) -> ClarityExpression {
    match expr {
        Expression::Literal(val) => {
            // Add 'u' prefix for number literals to match Clarity's uint format
            if val.chars().all(|c| c.is_digit(10)) {
                ClarityExpression::Literal(format!("u{}", val))
            } else {
                ClarityExpression::Literal(val)
            }
        }
        Expression::Identifier(name) => ClarityExpression::FunctionCall(
            "var-get".to_string(),
            vec![ClarityExpression::Var(name)]
        ),
        Expression::BinaryOp(left, op, right) => {
            ClarityExpression::FunctionCall(
                op,
                vec![convert_expression(*left), convert_expression(*right)]
            )
        }
        Expression::FunctionCall(name, args) => {
            ClarityExpression::FunctionCall(
                name,
                args.into_iter().map(convert_expression).collect()
            )
        }
        Expression::MapAccess(map_name, key) => {
            ClarityExpression::MapGet(
                map_name,
                vec![convert_expression(*key)]
            )
        }
    }
}