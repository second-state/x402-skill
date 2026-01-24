use crate::error::X402Error;
use reqwest::Response;
use std::fs::File;
use std::io::Write;

pub async fn handle_response(
    response: Response,
    output_file: Option<&str>,
    fail_on_error: bool,
    verbose: bool,
) -> Result<(), X402Error> {
    let status = response.status();

    if verbose {
        eprintln!("< {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));
    }

    if fail_on_error && !status.is_success() {
        return Err(X402Error::Http(format!("HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""))));
    }

    let body = response.bytes().await?;

    match output_file {
        Some(path) => {
            let mut file = File::create(path)
                .map_err(|e| X402Error::General(format!("Failed to create output file: {}", e)))?;
            file.write_all(&body)
                .map_err(|e| X402Error::General(format!("Failed to write output file: {}", e)))?;
        }
        None => {
            // Write to stdout
            std::io::stdout()
                .write_all(&body)
                .map_err(|e| X402Error::General(format!("Failed to write to stdout: {}", e)))?;
            // Add newline if output doesn't end with one
            if !body.ends_with(b"\n") {
                println!();
            }
        }
    }

    Ok(())
}
