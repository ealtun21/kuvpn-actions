use std::sync::Mutex;
use tokio::sync::mpsc;

pub struct GuiLogger {
    pub tx: Mutex<Option<mpsc::Sender<String>>>,
    pub user_level: Mutex<log::LevelFilter>,
}

impl GuiLogger {
    pub fn set_tx(&self, tx: mpsc::Sender<String>) {
        if let Ok(mut guard) = self.tx.lock() {
            *guard = Some(tx);
        }
    }

    pub fn set_level(&self, level: log::LevelFilter) {
        if let Ok(mut guard) = self.user_level.lock() {
            *guard = level;
        }
    }

    pub fn get_level(&self) -> log::LevelFilter {
        self.user_level
            .lock()
            .map(|g| *g)
            .unwrap_or(log::LevelFilter::Info)
    }
}

impl log::Log for GuiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Trace
    }
    fn log(&self, record: &log::Record) {
        let Ok(guard) = self.tx.lock() else { return };
        if let Some(tx) = &*guard {
            let _ = tx.try_send(format!("{:?}|{}", record.level(), record.args()));
        }
    }
    fn flush(&self) {}
}

pub static GUI_LOGGER: GuiLogger = GuiLogger {
    tx: Mutex::new(None),
    user_level: Mutex::new(log::LevelFilter::Info),
};

pub static LOGGER_INIT: std::sync::Once = std::sync::Once::new();
