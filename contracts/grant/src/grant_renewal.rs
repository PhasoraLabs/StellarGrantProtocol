pub fn check_renewal_eligibility(milestone_count: u32, completed_milestones: u32) -> bool {
    // Fix: grant_renewal's near-completion eligibility gate is vacuous for single-milestone grants
    if milestone_count <= 1 {
        return completed_milestones == milestone_count;
    }
    completed_milestones >= (milestone_count * 80) / 100 // Example 80% completion rule
}
