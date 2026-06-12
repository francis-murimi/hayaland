use serde::{Deserialize, Serialize};

/// A named role and the OAuth-style scopes it grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub scopes: Vec<String>,
    pub is_builtin: bool,
}

impl Role {
    pub fn new(name: impl Into<String>, scopes: Vec<String>) -> Self {
        Self {
            name: name.into(),
            scopes,
            is_builtin: false,
        }
    }

    pub fn builtin(name: impl Into<String>, scopes: Vec<String>) -> Self {
        Self {
            name: name.into(),
            scopes,
            is_builtin: true,
        }
    }
}
