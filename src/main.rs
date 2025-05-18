fn main() -> Result<(), Box<dyn std::error::Error>> {
    berry_api_api::start()?;
    Ok(())
}
