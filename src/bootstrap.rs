/// Bootstrap module - application entry point coordinator
/// This module provides a clean entry point for the application

#[derive(Debug, Clone)]
pub enum Mode {
    Daemon,
    Popup,
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
                "--daemon" => Mode::Daemon,
                "--popup" => Mode::Popup,
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
            Mode::Daemon => self.run_daemon(),
            Mode::Popup => self.run_popup(),
            Mode::All => self.run_all(),
            Mode::InstallAutostart => self.install_autostart(),
            Mode::RemoveAutostart => self.remove_autostart(),
        }
    }

    fn run_daemon(&self) {
        println!("Starting maccy-kde daemon...");
        crate::daemon::run();
    }

    fn run_popup(&self) {
        println!("Starting maccy-kde popup...");
        crate::popup::run();
    }

    fn run_all(&self) {
        println!("Starting maccy-kde in dev mode (everything)...");
        super::run_all_in_one();
    }

    fn install_autostart(&self) {
        match crate::autostart::install_autostart() {
            Ok(_) => println!("Автостарт успешно установлен!"),
            Err(e) => eprintln!("Ошибка установки автостарта: {}", e),
        }
    }

    fn remove_autostart(&self) {
        match crate::autostart::remove_autostart() {
            Ok(_) => println!("Автостарт успешно удален!"),
            Err(e) => eprintln!("Ошибка удаления автостарта: {}", e),
        }
    }
}