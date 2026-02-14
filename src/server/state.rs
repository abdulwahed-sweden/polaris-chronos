use crate::location::LocationResolver;
use std::sync::Mutex;

pub struct AppState {
    pub resolver: Mutex<LocationResolver>,
}
