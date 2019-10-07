#[cfg(debug_assertions)]
pub fn is_in_release_mode() -> bool {
    false
}

#[cfg(not(debug_assertions))]
pub fn is_in_release_mode() -> bool {
    true
}

pub fn is_in_debug_mode() -> bool {
    !is_in_release_mode()
}
