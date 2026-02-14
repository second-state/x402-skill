use alloy_signer_local::PrivateKeySigner;

use crate::error::X402Error;

const DEFAULT_RPC_URL: &str = "https://mainnet.base.org";
const BALANCE_OF_SELECTOR: &str = "0x70a08231";

/// USDC contract addresses by chain ID.
fn usdc_contract(chain_id: u64) -> Result<&'static str, X402Error> {
    match chain_id {
        8453 => Ok("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
        84532 => Ok("0x036CbD53842c5426634e7929541eC2318f3dCF7e"),
        _ => Err(X402Error::Rpc(format!(
            "Unsupported chain ID: {}. Only Base (8453) and Base Sepolia (84532) are supported.",
            chain_id
        ))),
    }
}

fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        8453 => "Base",
        84532 => "Base Sepolia",
        _ => "Unknown",
    }
}

fn parse_hex_u128(hex: &str) -> Result<u128, X402Error> {
    let hex = hex.trim().trim_start_matches("0x").trim_start_matches("0X");
    if hex.is_empty() {
        return Ok(0);
    }
    u128::from_str_radix(hex, 16)
        .map_err(|e| X402Error::Rpc(format!("Failed to parse hex value: {}", e)))
}

fn format_usdc(raw: u128) -> String {
    let whole = raw / 1_000_000;
    let frac = raw % 1_000_000;
    format!("{}.{:06}", whole, frac)
}

/// Resolve RPC URL: CLI flag > env var > default.
pub fn resolve_rpc(cli_rpc_url: Option<&str>) -> String {
    if let Some(url) = cli_rpc_url {
        return url.to_string();
    }
    if let Ok(url) = std::env::var("X402_RPC_URL") {
        if !url.is_empty() {
            return url;
        }
    }
    DEFAULT_RPC_URL.to_string()
}

/// Query and display the wallet's USDC balance on the detected chain.
pub async fn query_usdc_balance(signer: &PrivateKeySigner, rpc_url: &str) -> Result<(), X402Error> {
    let client = reqwest::Client::new();

    // 1. Detect chain ID
    let chain_id_response = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_chainId",
            "params": [],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| X402Error::Rpc(format!("Failed to connect to RPC: {}", e)))?;

    let chain_id_json: serde_json::Value = chain_id_response
        .json()
        .await
        .map_err(|e| X402Error::Rpc(format!("Invalid RPC response: {}", e)))?;

    if let Some(err) = chain_id_json.get("error") {
        return Err(X402Error::Rpc(format!("RPC error: {}", err)));
    }

    let chain_id_hex = chain_id_json["result"]
        .as_str()
        .ok_or_else(|| X402Error::Rpc("Missing chain ID in RPC response".to_string()))?;

    let chain_id = parse_hex_u128(chain_id_hex)? as u64;

    // 2. Resolve USDC contract for this chain
    let usdc_address = usdc_contract(chain_id)?;

    // 3. Build balanceOf calldata: selector + zero-padded address
    let address = format!("{:?}", signer.address()); // 0x-prefixed
    let padded_address = format!("{:0>64}", &address[2..]); // 32-byte left-padded
    let calldata = format!("{}{}", BALANCE_OF_SELECTOR, padded_address);

    // 4. Query balance via eth_call
    let balance_response = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{
                "to": usdc_address,
                "data": calldata
            }, "latest"],
            "id": 2
        }))
        .send()
        .await
        .map_err(|e| X402Error::Rpc(format!("Failed to query balance: {}", e)))?;

    let balance_json: serde_json::Value = balance_response
        .json()
        .await
        .map_err(|e| X402Error::Rpc(format!("Invalid balance response: {}", e)))?;

    if let Some(err) = balance_json.get("error") {
        return Err(X402Error::Rpc(format!("RPC error: {}", err)));
    }

    let balance_hex = balance_json["result"]
        .as_str()
        .ok_or_else(|| X402Error::Rpc("Missing balance in RPC response".to_string()))?;

    let balance_raw = parse_hex_u128(balance_hex)?;
    let balance_formatted = format_usdc(balance_raw);

    // 5. Print to stderr (matches existing diagnostic output convention)
    eprintln!(
        "Network:  {} (Chain ID: {})",
        chain_name(chain_id),
        chain_id
    );
    eprintln!("RPC:      {}", rpc_url);
    eprintln!("Address:  {:?}", signer.address());
    eprintln!("USDC:     {} USDC", balance_formatted);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_u128_zero() {
        assert_eq!(parse_hex_u128("0x0").unwrap(), 0);
        assert_eq!(parse_hex_u128("0x").unwrap(), 0);
        assert_eq!(parse_hex_u128("0").unwrap(), 0);
    }

    #[test]
    fn test_parse_hex_u128_one_usdc() {
        // 1 USDC = 1_000_000 = 0xF4240
        assert_eq!(parse_hex_u128("0xF4240").unwrap(), 1_000_000);
        assert_eq!(parse_hex_u128("F4240").unwrap(), 1_000_000);
    }

    #[test]
    fn test_parse_hex_u128_with_leading_zeros() {
        let hex = "0x0000000000000000000000000000000000000000000000000000000000F4240";
        assert_eq!(parse_hex_u128(hex).unwrap(), 1_000_000);
    }

    #[test]
    fn test_format_usdc_zero() {
        assert_eq!(format_usdc(0), "0.000000");
    }

    #[test]
    fn test_format_usdc_one() {
        assert_eq!(format_usdc(1_000_000), "1.000000");
    }

    #[test]
    fn test_format_usdc_fractional() {
        assert_eq!(format_usdc(1_500_000), "1.500000");
    }

    #[test]
    fn test_format_usdc_sub_penny() {
        assert_eq!(format_usdc(1), "0.000001");
        assert_eq!(format_usdc(123), "0.000123");
    }

    #[test]
    fn test_usdc_contract_base_mainnet() {
        assert_eq!(
            usdc_contract(8453).unwrap(),
            "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
        );
    }

    #[test]
    fn test_usdc_contract_base_sepolia() {
        assert_eq!(
            usdc_contract(84532).unwrap(),
            "0x036CbD53842c5426634e7929541eC2318f3dCF7e"
        );
    }

    #[test]
    fn test_usdc_contract_unsupported() {
        assert!(usdc_contract(1).is_err());
    }

    #[test]
    fn test_chain_name() {
        assert_eq!(chain_name(8453), "Base");
        assert_eq!(chain_name(84532), "Base Sepolia");
        assert_eq!(chain_name(1), "Unknown");
    }
}
