use alloy_signer_local::PrivateKeySigner;

use crate::error::X402Error;

const DEFAULT_RPC_URL: &str = "https://mainnet.base.org";
const BALANCE_OF_SELECTOR: &str = "0x70a08231";
const DECIMALS_SELECTOR: &str = "0x313ce567";
const SYMBOL_SELECTOR: &str = "0x95d89b41";

/// USDC contract addresses by chain ID.
fn usdc_contract(chain_id: u64) -> Result<&'static str, X402Error> {
    match chain_id {
        8453 => Ok("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
        84532 => Ok("0x036CbD53842c5426634e7929541eC2318f3dCF7e"),
        _ => Err(X402Error::Rpc(format!(
            "Unsupported chain ID: {}. Only Base (8453) and Base Sepolia (84532) are supported. Use --x402-token to specify a token address.",
            chain_id
        ))),
    }
}

fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "Ethereum",
        8453 => "Base",
        10 => "Optimism",
        42161 => "Arbitrum",
        137 => "Polygon",
        84532 => "Base Sepolia",
        11155111 => "Sepolia",
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

fn format_token_balance(raw: u128, decimals: u8) -> String {
    let divisor = 10u128.pow(decimals as u32);
    let whole = raw / divisor;
    let frac = raw % divisor;
    format!("{}.{:0>width$}", whole, frac, width = decimals as usize)
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

/// Make an eth_call and return the hex result string.
async fn eth_call(
    client: &reqwest::Client,
    rpc_url: &str,
    to: &str,
    data: &str,
) -> Result<String, X402Error> {
    let response = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [{"to": to, "data": data}, "latest"],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| X402Error::Rpc(format!("Failed to query contract: {}", e)))?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| X402Error::Rpc(format!("Invalid RPC response: {}", e)))?;

    if let Some(err) = json.get("error") {
        return Err(X402Error::Rpc(format!("RPC error: {}", err)));
    }

    json["result"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| X402Error::Rpc("Missing result in RPC response".to_string()))
}

/// Query the ERC-20 `decimals()` view function. Falls back to 18 on failure.
async fn query_token_decimals(client: &reqwest::Client, rpc_url: &str, token_address: &str) -> u8 {
    match eth_call(client, rpc_url, token_address, DECIMALS_SELECTOR).await {
        Ok(hex) => parse_hex_u128(&hex).map(|v| v as u8).unwrap_or(18),
        Err(_) => 18,
    }
}

/// Query the ERC-20 `symbol()` view function. Falls back to "Token" on failure.
async fn query_token_symbol(
    client: &reqwest::Client,
    rpc_url: &str,
    token_address: &str,
) -> String {
    match eth_call(client, rpc_url, token_address, SYMBOL_SELECTOR).await {
        Ok(hex) => decode_abi_string(&hex).unwrap_or_else(|| "Token".to_string()),
        Err(_) => "Token".to_string(),
    }
}

/// Decode an ABI-encoded string from a hex eth_call result.
/// Layout: 32 bytes offset | 32 bytes length | N bytes UTF-8 data (padded to 32).
fn decode_abi_string(hex: &str) -> Option<String> {
    let hex = hex.trim().trim_start_matches("0x").trim_start_matches("0X");
    // Need at least offset (64 hex chars) + length (64 hex chars) = 128 hex chars
    if hex.len() < 128 {
        return None;
    }
    // Bytes 32..64 contain the string length
    let len = usize::from_str_radix(&hex[64..128], 16).ok()?;
    if len == 0 {
        return Some(String::new());
    }
    // String data starts at byte 64 (hex offset 128)
    let data_end = 128 + len * 2;
    if hex.len() < data_end {
        return None;
    }
    let bytes: Vec<u8> = (128..data_end)
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    String::from_utf8(bytes).ok()
}

/// Query and display the wallet's token balance on the detected chain.
pub async fn query_balance(
    signer: &PrivateKeySigner,
    rpc_url: &str,
    token_override: Option<&str>,
) -> Result<(), X402Error> {
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

    // 2. Resolve token contract, decimals, and symbol
    let (token_address, decimals, symbol) = if let Some(addr) = token_override {
        // Custom token: query decimals and symbol on-chain
        let decimals = query_token_decimals(&client, rpc_url, addr).await;
        let symbol = query_token_symbol(&client, rpc_url, addr).await;
        (addr.to_string(), decimals, symbol)
    } else {
        // Default: USDC lookup (errors on unsupported chains)
        let addr = usdc_contract(chain_id)?;
        (addr.to_string(), 6u8, "USDC".to_string())
    };

    // 3. Build balanceOf calldata: selector + zero-padded address
    let address = format!("{:?}", signer.address()); // 0x-prefixed
    let padded_address = format!("{:0>64}", &address[2..]); // 32-byte left-padded
    let calldata = format!("{}{}", BALANCE_OF_SELECTOR, padded_address);

    // 4. Query balance via eth_call
    let balance_hex = eth_call(&client, rpc_url, &token_address, &calldata).await?;
    let balance_raw = parse_hex_u128(&balance_hex)?;
    let balance_formatted = format_token_balance(balance_raw, decimals);

    // 5. Print to stderr (matches existing diagnostic output convention)
    eprintln!(
        "Network:  {} (Chain ID: {})",
        chain_name(chain_id),
        chain_id
    );
    eprintln!("RPC:      {}", rpc_url);
    eprintln!("Address:  {:?}", signer.address());
    eprintln!("{}:     {} {}", symbol, balance_formatted, symbol);

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
    fn test_format_token_balance_usdc() {
        // Backward compat: 6 decimals (USDC)
        assert_eq!(format_token_balance(1_000_000, 6), "1.000000");
    }

    #[test]
    fn test_format_token_balance_18_decimals() {
        assert_eq!(
            format_token_balance(10u128.pow(18), 18),
            "1.000000000000000000"
        );
    }

    #[test]
    fn test_format_token_balance_zero_18() {
        assert_eq!(format_token_balance(0, 18), "0.000000000000000000");
    }

    #[test]
    fn test_format_token_balance_smallest_unit() {
        assert_eq!(format_token_balance(1, 18), "0.000000000000000001");
    }

    #[test]
    fn test_format_token_balance_zero_usdc() {
        assert_eq!(format_token_balance(0, 6), "0.000000");
    }

    #[test]
    fn test_format_token_balance_fractional() {
        assert_eq!(format_token_balance(1_500_000, 6), "1.500000");
    }

    #[test]
    fn test_format_token_balance_sub_penny() {
        assert_eq!(format_token_balance(1, 6), "0.000001");
        assert_eq!(format_token_balance(123, 6), "0.000123");
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
        assert_eq!(chain_name(1), "Ethereum");
        assert_eq!(chain_name(99999), "Unknown");
    }

    #[test]
    fn test_decode_abi_string_usdc() {
        // ABI-encoded "USDC":
        // offset = 0x20 (32), length = 0x04 (4), data = "USDC" + padding
        let hex = "0x\
            0000000000000000000000000000000000000000000000000000000000000020\
            0000000000000000000000000000000000000000000000000000000000000004\
            5553444300000000000000000000000000000000000000000000000000000000";
        assert_eq!(decode_abi_string(hex).unwrap(), "USDC");
    }

    #[test]
    fn test_decode_abi_string_weth() {
        // ABI-encoded "WETH"
        let hex = "0x\
            0000000000000000000000000000000000000000000000000000000000000020\
            0000000000000000000000000000000000000000000000000000000000000004\
            5745544800000000000000000000000000000000000000000000000000000000";
        assert_eq!(decode_abi_string(hex).unwrap(), "WETH");
    }

    #[test]
    fn test_decode_abi_string_empty() {
        let hex = "0x\
            0000000000000000000000000000000000000000000000000000000000000020\
            0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(decode_abi_string(hex).unwrap(), "");
    }

    #[test]
    fn test_decode_abi_string_too_short() {
        assert_eq!(decode_abi_string("0x1234"), None);
    }
}
