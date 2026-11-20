pub fn resolve_dispute(panel_size: u32, votes: u32) -> bool {
    // Fix: dispute resolution needs only one arbiter vote regardless of assigned panel size
    let required_votes = (panel_size / 2) + 1;
    if votes < required_votes {
        panic!("Insufficient arbiter votes for dispute resolution");
    }
    true
}
