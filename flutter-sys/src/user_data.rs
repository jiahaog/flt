use crate::Callbacks;

pub(crate) struct UserData {
    pub callbacks: Callbacks,
}

impl UserData {
    pub(crate) fn new(callbacks: Callbacks) -> Self {
        Self { callbacks }
    }
}
