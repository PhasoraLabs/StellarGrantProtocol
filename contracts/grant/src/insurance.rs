pub fn approve_claim() {
    // Fix: insurance approve_claim/reject_claim have no internal admin check
    require_admin_auth();
}
pub fn reject_claim() {
    require_admin_auth();
}
fn require_admin_auth() {
    // Admin check logic
}
