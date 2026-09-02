use color_eyre::eyre::{self, Result};
use std::path::Path;
use yansi::Paint;

pub fn run(cwd: &Path) -> Result<()> {
    let config_path = cwd.join("cari.json");

    if config_path.exists() {
        eyre::bail!("cari.json already exists");
    }

    std::fs::write(&config_path, DEFAULT_CONFIG)?;

    println!(" {} {}", "✔".green(), "Created cari.json".bold());
    println!("   {}", "Edit it to configure your project".dim());

    Ok(())
}

const DEFAULT_CONFIG: &str = r#"{
    "$schema": "https://raw.githubusercontent.com/ghostdevv/cari/refs/heads/main/cari.schema.json",
    "loader": "",
    "gameVersion": "",
    "runtime": {},
    "content": []
}
"#;
