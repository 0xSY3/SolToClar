use super::parser;
use super::generator;
use super::transpiler::{ast::*, converter::*};
use anyhow::Result;
use crate::transpiler::converter::convert_solidity_type;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_contract() -> Result<()> {
        let source = r#"
            contract Counter {
                uint256 count;
                function increment() {
                    count = count + 1;
                }
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);
        assert_eq!(contract.name, "Counter");
        assert_eq!(contract.state_variables.len(), 1);
        assert_eq!(contract.functions.len(), 1);
        Ok(())
    }

    #[test]
    fn test_parse_state_variables() -> Result<()> {
        let source = r#"
            contract Test {
                uint256 count;
                bool flag;
                address owner;
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);
        assert_eq!(contract.state_variables.len(), 3);
        assert_eq!(contract.state_variables[0].var_type, "uint256");
        assert_eq!(contract.state_variables[1].var_type, "bool");
        assert_eq!(contract.state_variables[2].var_type, "address");
        Ok(())
    }

    #[test]
    fn test_parse_function_with_return() -> Result<()> {
        let source = r#"
            contract Test {
                uint256 value;
                function getValue() returns (uint256) {
                    return value;
                }
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);
        let func = &contract.functions[0];
        assert_eq!(func.name, "getValue");
        assert_eq!(func.return_type, Some("uint256".to_string()));
        Ok(())
    }

    #[test]
    fn test_parse_binary_expression() -> Result<()> {
        let source = r#"
            contract Test {
                uint256 x;
                function add(uint256 y) {
                    x = x + y;
                }
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);
        let func = &contract.functions[0];
        assert_eq!(func.name, "add");

        // Verify the function body contains an assignment with binary operation
        match &func.body[0] {
            Statement::Assignment(var_name, Expression::BinaryOp(left, op, right)) => {
                assert_eq!(var_name, "x");
                assert_eq!(op, "+");
                match (left.as_ref(), right.as_ref()) {
                    (Expression::Identifier(l), Expression::Identifier(r)) => {
                        assert_eq!(l, "x");
                        assert_eq!(r, "y");
                    },
                    _ => panic!("Expected identifier expressions"),
                }
            },
            _ => panic!("Expected assignment statement"),
        }
        Ok(())
    }

    #[test]
    fn test_generate_clarity_code() -> Result<()> {
        let contract = Contract {
            name: "Test".to_string(),
            state_variables: vec![
                StateVariable {
                    name: "count".to_string(),
                    var_type: "uint256".to_string(),
                    visibility: None,
                    is_mapping: false,
                    mapping_key_type: None,
                    mapping_value_type: None,
                    initial_value: None,
                    is_constant: false,
                    nested_mapping: None,
                }
            ],
            functions: vec![
                Function {
                    name: "increment".to_string(),
                    params: vec![],
                    return_type: None,
                    visibility: Some("public".to_string()),
                    mutability: None,
                    body: vec![
                        Statement::Assignment(
                            "count".to_string(),
                            Expression::BinaryOp(
                                Box::new(Expression::Identifier("count".to_string())),
                                "+".to_string(),
                                Box::new(Expression::Literal("1".to_string()))
                            )
                        )
                    ],
                }
            ],
            events: vec![],
            constructor: None,
        };

        let clarity_contract = convert_contract(contract)?;
        let clarity_code = generator::generate(clarity_contract)?;

        assert!(clarity_code.contains("(define-data-var count uint u0)"));
        assert!(clarity_code.contains("(define-public (increment)"));
        assert!(clarity_code.contains("(var-set count (+ (var-get count) u1))"));
        Ok(())
    }

    #[test]
    fn test_parse_empty_contract() -> Result<()> {
        let source = r#"
            contract Empty {
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);
        assert_eq!(contract.name, "Empty");
        assert_eq!(contract.state_variables.len(), 0);
        assert_eq!(contract.functions.len(), 0);
        Ok(())
    }

    #[test]
    fn test_parse_invalid_contract() {
        let source = r#"
            contract {
                invalid syntax
            }
        "#;
        assert!(parser::parse_all(source).is_err());
    }

    #[test]
    fn test_parse_mapping() -> Result<()> {
        let source = r#"
            contract Test {
                mapping(address => uint256) balances;
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);
        let var = &contract.state_variables[0];
        assert!(var.is_mapping);
        assert_eq!(var.mapping_key_type.as_ref().unwrap(), "address");
        assert_eq!(var.mapping_value_type.as_ref().unwrap(), "uint256");
        Ok(())
    }

    #[test]
    fn test_parse_event() -> Result<()> {
        let source = r#"
            contract Test {
                event Transfer(address indexed from, address indexed to, uint256 amount);
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);
        let event = &contract.events[0];
        assert_eq!(event.name, "Transfer");
        assert_eq!(event.params.len(), 3);
        // Check indexed parameters
        assert!(event.params[0].indexed);
        assert!(event.params[1].indexed);
        assert!(!event.params[2].indexed);
        Ok(())
    }

    #[test]
    fn test_type_conversion() {
        assert_eq!(convert_solidity_type("uint256"), "uint");
        assert_eq!(convert_solidity_type("bool"), "bool");
        assert_eq!(convert_solidity_type("address"), "principal");
        assert_eq!(convert_solidity_type("string"), "string-ascii");
        assert_eq!(convert_solidity_type("unknown"), "uint"); // default case
    }

    #[test]
    fn test_function_visibility() -> Result<()> {
        let source = r#"
            contract Test {
                function publicFunc() public {
                    // public function
                }

                function privateFunc() private {
                    // private function
                }

                function internalFunc() internal {
                    // internal function
                }

                function externalFunc() external {
                    // external function
                }
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);

        assert_eq!(contract.functions.len(), 4);
        assert_eq!(contract.functions[0].visibility, Some("public".to_string()));
        assert_eq!(contract.functions[1].visibility, Some("private".to_string()));
        assert_eq!(contract.functions[2].visibility, Some("internal".to_string()));
        assert_eq!(contract.functions[3].visibility, Some("external".to_string()));

        let clarity_contract = convert_contract(contract)?;
        let clarity_code = generator::generate(clarity_contract)?;

        // Public and external functions should be define-public
        assert!(clarity_code.contains("(define-public (publicFunc)"));
        assert!(clarity_code.contains("(define-public (externalFunc)"));

        // Private and internal functions should be define-private
        assert!(clarity_code.contains("(define-private (privateFunc)"));
        assert!(clarity_code.contains("(define-private (internalFunc)"));

        Ok(())
    }

    #[test]
    fn test_state_variable_visibility() -> Result<()> {
        let source = r#"
            contract Test {
                uint256 public count;
                bool private flag;
                address internal owner;
                uint256 constant LIMIT = 100;
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);

        // Check state variables were parsed correctly
        assert_eq!(contract.state_variables.len(), 4);
        assert_eq!(contract.state_variables[0].visibility, Some("public".to_string()));
        assert_eq!(contract.state_variables[1].visibility, Some("private".to_string()));
        assert_eq!(contract.state_variables[2].visibility, Some("internal".to_string()));
        assert!(contract.state_variables[3].is_constant);
        // Check the initial value of the constant is a literal "100"
        match &contract.state_variables[3].initial_value {
            Some(Expression::Literal(val)) => assert_eq!(val, "100"),
            _ => panic!("Expected literal value for constant"),
        }

        // Check generated Clarity code
        let clarity_contract = convert_contract(contract)?;
        let clarity_code = generator::generate(clarity_contract)?;

        // Public variables should have a getter function
        assert!(clarity_code.contains("(define-read-only (get-count)"));
        // All variables should be defined
        assert!(clarity_code.contains("(define-data-var count uint u0)"));
        assert!(clarity_code.contains("(define-data-var flag bool false)"));
        assert!(clarity_code.contains("(define-data-var owner principal tx-sender)"));
        // Constants should be defined using define-constant
        assert!(clarity_code.contains("(define-constant LIMIT u100)"));

        Ok(())
    }

    #[test]
    fn test_complex_mappings() -> Result<()> {
        let source = r#"
            contract Test {
                // Simple mapping
                mapping(address => uint256) public balances;

                // Nested mapping
                mapping(address => mapping(uint256 => bool)) public approvals;

                // Complex value type
                mapping(uint256 => address) public tokenOwners;
            }
        "#;
        let contract = parser::parse_all(source)?.remove(0);

        // Check balances mapping
        let balances = &contract.state_variables[0];
        assert!(balances.is_mapping);
        assert_eq!(balances.mapping_key_type.as_ref().unwrap(), "address");
        assert_eq!(balances.mapping_value_type.as_ref().unwrap(), "uint256");
        assert_eq!(balances.visibility.as_ref().unwrap(), "public");

        // Check approvals mapping (nested)
        let approvals = &contract.state_variables[1];
        assert!(approvals.is_mapping);
        assert_eq!(approvals.mapping_key_type.as_ref().unwrap(), "address");
        assert_eq!(approvals.mapping_value_type.as_ref().unwrap(), "mapping(uint256 => bool)");
        assert_eq!(approvals.visibility.as_ref().unwrap(), "public");

        // Check tokenOwners mapping
        let token_owners = &contract.state_variables[2];
        assert!(token_owners.is_mapping);
        assert_eq!(token_owners.mapping_key_type.as_ref().unwrap(), "uint256");
        assert_eq!(token_owners.mapping_value_type.as_ref().unwrap(), "address");
        assert_eq!(token_owners.visibility.as_ref().unwrap(), "public");

        // Check generated Clarity code
        let clarity_contract = convert_contract(contract)?;
        let clarity_code = generator::generate(clarity_contract)?;

        // Verify balances map
        assert!(clarity_code.contains("(define-map balances principal uint)"));

        // Verify approvals map (should be flattened in Clarity)
        assert!(clarity_code.contains("(define-map approvals {owner: principal, token-id: uint} bool)"));

        // Verify tokenOwners map
        assert!(clarity_code.contains("(define-map token-owners uint principal)"));

        // Verify public getter functions are generated
        assert!(clarity_code.contains("(define-read-only (get-balances"));
        assert!(clarity_code.contains("(define-read-only (get-approvals"));
        assert!(clarity_code.contains("(define-read-only (get-token-owners"));

        Ok(())
    }
}