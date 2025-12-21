use crate::system_info::error::SystemError;
use std::env;

pub(crate) fn get_env_any(
    keys: &[&str],
    label: &'static str,
) -> Result<String, SystemError> {
    for &key in keys {
        if let Ok(val) = env::var(key) {
            return Ok(val);
        }
    }
    Err(SystemError::MissingEnv(label))
}
