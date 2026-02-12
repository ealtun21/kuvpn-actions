use kuvpn::utils::{CredentialsProvider, CancellationToken};
use tokio::sync::{mpsc, oneshot};
use crate::types::InputRequest;

#[derive(Debug)]
pub enum GuiInteraction {
    Request(InputRequest),
    MfaPush(String),
    MfaComplete,
}

pub struct GuiProvider {
    pub interaction_tx: mpsc::Sender<GuiInteraction>,
    pub cancel_token: CancellationToken,
}

impl CredentialsProvider for GuiProvider {
    fn request_text(&self, msg: &str) -> String {
        self.request(msg, false)
    }
    fn request_password(&self, msg: &str) -> String {
        self.request(msg, true)
    }
    fn on_mfa_push(&self, code: &str) {
        let _ = self.interaction_tx.blocking_send(GuiInteraction::MfaPush(code.to_string()));
    }
    fn on_mfa_complete(&self) {
        let _ = self.interaction_tx.blocking_send(GuiInteraction::MfaComplete);
    }
}

impl GuiProvider {
    fn request(&self, msg: &str, is_password: bool) -> String {
        let (tx, rx) = oneshot::channel();
        let request = InputRequest {
            msg: msg.to_string(),
            is_password,
            response_tx: tx,
        };
        
        let _ = self.interaction_tx.blocking_send(GuiInteraction::Request(request));
        
        // Wait for response, but also poll for cancellation
        let mut rx = rx;
        loop {
            if self.cancel_token.is_cancelled() {
                return String::new();
            }
            match rx.try_recv() {
                Ok(val) => return val,
                Err(oneshot::error::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(oneshot::error::TryRecvError::Closed) => return String::new(),
            }
        }
    }
}