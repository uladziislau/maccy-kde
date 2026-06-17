#[derive(Debug, Clone)]
pub enum Mode {
    All,
    InstallAutostart,
    RemoveAutostart,
}

pub struct Bootstrap {
    mode: Mode,
}

impl Bootstrap {
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mode = if args.len() > 1 {
            match args[1].as_str() {
                "--install-autostart" => Mode::InstallAutostart,
                "--remove-autostart" => Mode::RemoveAutostart,
                _ => Mode::All,
            }
        } else {
            Mode::All
        };
        Self { mode }
    }

    pub fn run(&self) {
        match self.mode {
            Mode::All => {
                println!("Starting maccy-kde...");
                super::run_all_in_one();
            }
            Mode::InstallAutostart => {
                match crate::autostart::install_autostart() {
                    Ok(_) => println!("Autostart installed!"),
                    Err(e) => eprintln!("Failed to install autostart: {}", e),
                }
            }
            Mode::RemoveAutostart => {
                match crate::autostart::remove_autostart() {
                    Ok(_) => println!("Autostart removed!"),
                    Err(e) => eprintln!("Failed to remove autostart: {}", e),
                }
            }
        }
    }
}
