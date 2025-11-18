use crate::{StorageMediaState, Arc, Mutex};

pub struct DialogOption {
    pub text: String,
    pub value: String,
    pub disabled: bool,
}

pub struct Dialog {
    pub id: String,
    pub desc: Option<String>,
    pub options: Vec<DialogOption>,
    pub selection: usize,
}

pub fn create_confirm_delete_dialog() -> Dialog {
    Dialog {
        id: "confirm_delete".to_string(),
        desc: Some("PERMANENTLY DELETE THIS SAVE DATA?".to_string()),
        options: vec![
            DialogOption {
                text: "DELETE".to_string(),
                value: "DELETE".to_string(),
                disabled: false,
            },
            DialogOption {
                text: "CANCEL".to_string(),
                value: "CANCEL".to_string(),
                disabled: false,
            }
        ],
        selection: 1,
    }
}

pub fn create_copy_storage_dialog(storage_state: &Arc<Mutex<StorageMediaState>>) -> Dialog {
    let mut options = Vec::new();
    if let Ok(state) = storage_state.lock() {
        for drive in state.media.iter() {
            if drive.id == state.media[state.selected].id {
                continue;
            }
            options.push(DialogOption {
                text: format!("{} ({} MB Free)", drive.id.clone(), drive.free),
                value: drive.id.clone(),
                disabled: false,
            });
        }
    }
    options.push(DialogOption {
        text: "CANCEL".to_string(),
        value: "CANCEL".to_string(),
        disabled: false,
    });

    Dialog {
        id: "copy_storage_select".to_string(),
        desc: Some("WHERE TO COPY THIS SAVE DATA?".to_string()),
        options,
        selection: 0,
    }
}

pub fn create_main_dialog(storage_state: &Arc<Mutex<StorageMediaState>>) -> Dialog {
    let has_external_devices = if let Ok(state) = storage_state.lock() {
        state.media.len() > 1
    } else {
        false
    };

    let options = vec![
        DialogOption {
            text: "COPY".to_string(),
            value: "COPY".to_string(),
            disabled: !has_external_devices,
        },
        DialogOption {
            text: "DELETE".to_string(),
            value: "DELETE".to_string(),
            disabled: false,
        },
        DialogOption {
            text: "CANCEL".to_string(),
            value: "CANCEL".to_string(),
            disabled: false,
        },
    ];

    Dialog {
        id: "main".to_string(),
        desc: None,
        options,
        selection: 0,
    }
}

pub fn create_save_exists_dialog() -> Dialog {
    Dialog {
        id: "save_exists".to_string(),
        desc: Some("THIS SAVE DATA ALREADY EXISTS AT THE SELECTED DESTINATION".to_string()),
        options: vec![
            DialogOption {
                text: "OK".to_string(),
                value: "OK".to_string(),
                disabled: false,
            }
        ],
        selection: 0,
    }
}

pub fn create_error_dialog(message: String) -> Dialog {
    Dialog {
        id: "error".to_string(),
        desc: Some(message),
        options: vec![
            DialogOption {
                text: "OK".to_string(),
                value: "OK".to_string(),
                disabled: false,
            }
        ],
        selection: 0,
    }
}
