use axum_flash::IncomingFlashes;

pub fn get_success_and_error_flash_message(
    flashes: &IncomingFlashes,
) -> (Option<String>, Option<String>) {
    let success_msg = flashes
        .iter()
        .find(|(l, _)| l == &axum_flash::Level::Success)
        .map(|(_, m)| m.to_string());
    let error_msg = flashes
        .iter()
        .find(|(l, _)| l == &axum_flash::Level::Error)
        .map(|(_, m)| m.to_string());

    (success_msg, error_msg)
}
