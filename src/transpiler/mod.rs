pub mod ast;
pub mod converter;

use anyhow::Result;
use ast::Contract;

pub fn convert(contract: Contract) -> Result<converter::ClarityContract> {
    converter::convert_contract(contract)
}