pub fn certify_evidence(caller: &str, owner: &str) -> bool {
    // Fix: evidence schema is self-certifiable by the grant owner
    if caller == owner {
        panic!("Grant owner cannot self-certify evidence");
    }
    true
}
