fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::env::args().nth(1).unwrap_or_else(|| "cp1252".into());
    let case = mojibake_core::analyze_sample(&id)?;
    println!("{}", serde_json::to_string_pretty(&case)?);
    Ok(())
}
