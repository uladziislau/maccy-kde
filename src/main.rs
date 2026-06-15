use std::fs::File;
use clap::Parser;
use log::{info, error};
use maccy_kde::app;
use maccy_kde::ipc;
use maccy_kde::autostart;
use fs2::FileExt;

/// Легковесный менеджер буфера обмена для KDE Plasma 6 (Wayland)
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Запустить фоновый демон
    #[arg(long)]
    daemon: bool,

    /// Запустить графическое окно
    #[arg(long)]
    popup: bool,

    /// Установить автостарт для KDE Plasma
    #[arg(long)]
    install_autostart: bool,

    /// Удалить автостарт для KDE Plasma
    #[arg(long)]
    remove_autostart: bool,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    let _lock_file = if args.daemon || (!args.daemon && !args.popup) {
        let lock_path = std::env::temp_dir().join("maccy-kde.lock");
        let file = File::create(&lock_path).expect("Failed to create lock file");
        if file.try_lock_exclusive().is_err() {
            if args.daemon {
                eprintln!("Daemon is already running.");
                std::process::exit(1);
            }
            None
        } else {
            Some(file)
        }
    } else {
        None
    };

    if args.install_autostart {
        match autostart::install_autostart() {
            Ok(_) => println!("Автостарт успешно установлен!"),
            Err(e) => eprintln!("Ошибка установки автостарта: {}", e),
        }
        return;
    }

    if args.remove_autostart {
        match autostart::remove_autostart() {
            Ok(_) => println!("Автостарт успешно удален!"),
            Err(e) => eprintln!("Ошибка удаления автостарта: {}", e),
        }
        return;
    }

    // Если ни один флаг не указан, запускаем всё (для разработки)
    if !args.daemon && !args.popup {
        info!("Starting maccy-kde in dev mode (everything)...");
        app::run_all_in_one();
        return;
    }

    if args.daemon {
        info!("Starting maccy-kde daemon...");
        app::run_daemon();
    }

    if args.popup {
        info!("Requesting popup from daemon...");
        let rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(ipc::send_command(ipc::IpcCommand::ShowPopup)) {
            Ok(_) => {
                info!("Popup requested successfully");
                return;
            },
            Err(e) => {
                error!("Failed to request popup: {}", e);
                info!("Starting popup in standalone mode...");
                app::run_popup();
                return;
            }
        }
    }
}
