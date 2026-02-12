use std::sync::Mutex;
use tokio::sync::mpsc;

pub struct GuiLogger {
    pub tx: Mutex<Option<mpsc::Sender<String>>>,
    pub user_level: Mutex<log::LevelFilter>,
}

impl log::Log for GuiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool { 
        metadata.level() <= log::Level::Trace
    }
    fn log(&self, record: &log::Record) {
        if let Ok(guard) = self.tx.lock() {
            if let Some(tx) = &*guard {
                let _ = tx.try_send(format!("{:?}|{}", record.level(), record.args()));
            }
        }
    }
    fn flush(&self) {}
}

pub static GUI_LOGGER: GuiLogger = GuiLogger {
    tx: Mutex::new(None),
    user_level: Mutex::new(log::LevelFilter::Info),
};

pub static LOGGER_INIT: std::sync::Once = std::sync::Once::new();
