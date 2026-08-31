use crate::storage::Storage;
use crate::types::{ContractError, MilestoneDag, MilestoneDependency, MilestoneState};
use soroban_sdk::{Address, Env, Vec as SorobanVec};

/// Check if a milestone can be submitted by verifying all dependencies are satisfied.
/// A milestone can be submitted if all previous milestones have been approved.
pub fn can_submit(env: &Env, grant_id: u64, milestone_idx: u32) -> Result<(), ContractError> {
    if let Some(dag) = Storage::get_milestone_dag(env, grant_id) {
        for dep in dag.dependencies.iter() {
            if dep.milestone_idx == milestone_idx {
                for parent in dep.depends_on.iter() {
                    if let Some(m) = Storage::get_milestone(env, grant_id, parent) {
                        if m.state != MilestoneState::Approved {
                            return Err(ContractError::DependencyNotSatisfied);
                        }
                    } else {
                        return Err(ContractError::DependencyNotSatisfied);
                    }
                }
                return Ok(());
            }
        }
    }
    // Fallback: sequential ordering when no DAG entry covers this milestone.
    if milestone_idx == 0 {
        return Ok(());
    }

    for prev_idx in 0..milestone_idx {
        if let Some(milestone) = Storage::get_milestone(env, grant_id, prev_idx) {
            if milestone.state != MilestoneState::Approved {
                return Err(ContractError::DependencyNotSatisfied);
            }
        } else {
            return Err(ContractError::DependencyNotSatisfied);
        }
    }

    Ok(())
}

/// Attach a dependency graph (DAG) to a grant. Validates that dependencies
/// reference valid milestone indices and stores the graph.
pub fn attach_dag(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    deps: SorobanVec<MilestoneDependency>,
) -> Result<(), ContractError> {
    owner.require_auth();
    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    for dep in deps.iter() {
        if dep.milestone_idx >= grant.total_milestones {
            return Err(ContractError::InvalidInput);
        }
        for parent in dep.depends_on.iter() {
            if parent >= grant.total_milestones {
                return Err(ContractError::InvalidInput);
            }
            if parent >= dep.milestone_idx {
                return Err(ContractError::InvalidInput);
            }
        }
    }

    let dag = MilestoneDag {
        grant_id,
        dependencies: deps,
        is_valid: true,
    };
    Storage::set_milestone_dag(env, grant_id, &dag);
    Ok(())
}

/// Return the set of milestone indices whose dependencies are all satisfied.
pub fn unblocked_milestones(env: &Env, grant_id: u64) -> SorobanVec<u32> {
    let grant = match Storage::get_grant(env, grant_id) {
        Some(g) => g,
        None => return SorobanVec::new(env),
    };
    let dag = Storage::get_milestone_dag(env, grant_id);
    let mut result: SorobanVec<u32> = SorobanVec::new(env);

    for idx in 0..grant.total_milestones {
        if let Some(milestone) = Storage::get_milestone(env, grant_id, idx) {
            if milestone.state == MilestoneState::Approved {
                continue;
            }
        }
        let blocked = match &dag {
            Some(d) => d.dependencies.iter().any(|dep| {
                dep.milestone_idx == idx
                    && dep.depends_on.iter().any(|parent| {
                        Storage::get_milestone(env, grant_id, parent)
                            .map(|m| m.state != MilestoneState::Approved)
                            .unwrap_or(true)
                    })
            }),
            None => false,
        };
        if !blocked {
            result.push_back(idx);
        }
    }
    result
}

/// Return milestone indices that directly depend on `idx`.
pub fn dependents_of(env: &Env, grant_id: u64, idx: u32) -> SorobanVec<u32> {
    let dag = match Storage::get_milestone_dag(env, grant_id) {
        Some(d) => d,
        None => return SorobanVec::new(env),
    };
    let mut result: SorobanVec<u32> = SorobanVec::new(env);
    for dep in dag.dependencies.iter() {
        if dep.depends_on.iter().any(|p| p == idx) {
            result.push_back(dep.milestone_idx);
        }
    }
    result
}

/// Retrieve the stored DAG for a grant.
pub fn get_dag(env: &Env, grant_id: u64) -> Option<MilestoneDag> {
    Storage::get_milestone_dag(env, grant_id)
}

/// Compute a topological ordering of milestones given a set of dependencies.
/// Returns an error if a cycle is detected.
pub fn topological_order(
    env: &Env,
    deps: &SorobanVec<MilestoneDependency>,
    total: u32,
) -> Result<SorobanVec<u32>, ContractError> {
    let n = total;
    let mut in_degree: SorobanVec<u32> = SorobanVec::new(env);
    let mut adj: SorobanVec<SorobanVec<u32>> = SorobanVec::new(env);

    for i in 0..n {
        in_degree.push_back(0u32);
        adj.push_back(SorobanVec::new(env));
    }

    for dep in deps.iter() {
        let u = dep.milestone_idx;
        for parent in dep.depends_on.iter() {
            let p = parent;
            if p < n && u < n {
                let mut neighbors = adj.get(p).unwrap();
                neighbors.push_back(u);
                adj.set(p, neighbors);
                let mut deg = in_degree.get(u).unwrap();
                deg += 1;
                in_degree.set(u, deg);
            }
        }
    }

    let mut queue: SorobanVec<u32> = SorobanVec::new(env);
    for i in 0..n {
        if in_degree.get(i).unwrap() == 0 {
            queue.push_back(i);
        }
    }

    let mut order: SorobanVec<u32> = SorobanVec::new(env);
    while let Some(node) = queue.pop_back() {
        order.push_back(node);
        let neighbors = adj.get(node).unwrap();
        for next in neighbors.iter() {
            let mut deg = in_degree.get(next).unwrap();
            deg = deg.saturating_sub(1);
            in_degree.set(next, deg);
            if deg == 0 {
                queue.push_back(next);
            }
        }
    }

    if order.len() != n {
        return Err(ContractError::InvalidInput);
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantStatus, Milestone, MilestoneState};
    use crate::StellarGrantsContract;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, Map, String};

    fn make_grant(env: &Env, owner: &Address, total_milestones: u32) -> Grant {
        Grant {
            id: 1,
            owner: owner.clone(),
            title: String::from_str(env, "T"),
            description: String::from_str(env, "D"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1_000,
            milestone_amount: 250,
            reviewers: SorobanVec::new(env),
            total_milestones,
            milestones_paid_out: 0,
            escrow_balance: 0,
            funders: SorobanVec::new(env),
            reason: None,
            timestamp: 0,
            require_compliance: None,
        }
    }

    fn make_milestone(env: &Env, idx: u32, state: MilestoneState) -> Milestone {
        Milestone {
            idx,
            description: String::from_str(env, "M"),
            amount: 250,
            state,
            votes: Map::new(env),
            approvals: 0,
            rejections: 0,
            reasons: Map::new(env),
            status_updated_at: 0,
            proof_url: None,
            submission_timestamp: 0,
            deadline: None,
            reviewer_count_snapshot: 0,
        }
    }

    fn dep(env: &Env, milestone_idx: u32, parents: &[u32]) -> MilestoneDependency {
        let mut depends_on = SorobanVec::new(env);
        for p in parents {
            depends_on.push_back(*p);
        }
        MilestoneDependency {
            milestone_idx,
            depends_on,
        }
    }

    // ── can_submit: sequential fallback (no DAG attached) ────────────────────

    #[test]
    fn can_submit_first_milestone_always_allowed() {
        let env = Env::default();
        let contract_id = env.register(StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            assert_eq!(can_submit(&env, 1, 0), Ok(()));
        });
    }

    #[test]
    fn can_submit_blocks_when_previous_milestone_not_approved() {
        let env = Env::default();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));
            Storage::set_milestone(
                &env,
                1,
                0,
                &make_milestone(&env, 0, MilestoneState::Approved),
            );
            Storage::set_milestone(
                &env,
                1,
                1,
                &make_milestone(&env, 1, MilestoneState::Submitted),
            );

            // milestone 1 is only Submitted -> milestone 2 must not be submittable.
            assert_eq!(
                can_submit(&env, 1, 2),
                Err(ContractError::DependencyNotSatisfied)
            );
            // milestone 1 itself is fine (milestone 0 is Approved).
            assert_eq!(can_submit(&env, 1, 1), Ok(()));
        });
    }

    #[test]
    fn can_submit_blocks_when_previous_milestone_missing() {
        let env = Env::default();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));
            // No milestone 0 record at all.
            assert_eq!(
                can_submit(&env, 1, 1),
                Err(ContractError::DependencyNotSatisfied)
            );
        });
    }

    // ── attach_dag + can_submit: declared-dependency path (issue #996) ───────

    #[test]
    fn attach_dag_stores_valid_non_linear_dag() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));

            let mut deps = SorobanVec::new(&env);
            deps.push_back(dep(&env, 2, &[0]));
            deps.push_back(dep(&env, 3, &[1, 2]));

            attach_dag(&env, &owner, 1, deps).unwrap();

            let stored = get_dag(&env, 1).unwrap();
            assert_eq!(stored.dependencies.len(), 2);
            assert!(stored.is_valid);
        });
    }

    #[test]
    fn can_submit_with_dag_honors_only_declared_dependencies() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));

            // Milestone 2 depends only on 0; milestone 3 depends on 1 and 2.
            let mut deps = SorobanVec::new(&env);
            deps.push_back(dep(&env, 2, &[0]));
            deps.push_back(dep(&env, 3, &[1, 2]));
            attach_dag(&env, &owner, 1, deps).unwrap();

            // Only milestone 0 approved; milestone 1 still pending.
            Storage::set_milestone(
                &env,
                1,
                0,
                &make_milestone(&env, 0, MilestoneState::Approved),
            );
            Storage::set_milestone(
                &env,
                1,
                1,
                &make_milestone(&env, 1, MilestoneState::Pending),
            );

            // Milestone 2 only declares 0 as a dependency -> submittable even
            // though the preceding milestone 1 is not approved (issue #996).
            assert_eq!(can_submit(&env, 1, 2), Ok(()));

            // Milestone 3 declares 1 and 2, neither approved -> blocked.
            assert_eq!(
                can_submit(&env, 1, 3),
                Err(ContractError::DependencyNotSatisfied)
            );
        });
    }

    // ── attach_dag validation (cycle / range / auth) ────────────────────────

    #[test]
    fn attach_dag_rejects_forward_and_self_dependencies() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));
        });

        // parent index >= dependent index would allow a cycle -> rejected.
        env.as_contract(&contract_id, || {
            let mut forward = SorobanVec::new(&env);
            forward.push_back(dep(&env, 1, &[2]));
            assert_eq!(
                attach_dag(&env, &owner, 1, forward),
                Err(ContractError::InvalidInput)
            );
        });

        env.as_contract(&contract_id, || {
            let mut selfdep = SorobanVec::new(&env);
            selfdep.push_back(dep(&env, 1, &[1]));
            assert_eq!(
                attach_dag(&env, &owner, 1, selfdep),
                Err(ContractError::InvalidInput)
            );
        });
    }

    #[test]
    fn attach_dag_rejects_out_of_range_indices() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));
        });

        env.as_contract(&contract_id, || {
            let mut bad_dependent = SorobanVec::new(&env);
            bad_dependent.push_back(dep(&env, 9, &[0]));
            assert_eq!(
                attach_dag(&env, &owner, 1, bad_dependent),
                Err(ContractError::InvalidInput)
            );
        });

        env.as_contract(&contract_id, || {
            let mut bad_parent = SorobanVec::new(&env);
            bad_parent.push_back(dep(&env, 2, &[9]));
            assert_eq!(
                attach_dag(&env, &owner, 1, bad_parent),
                Err(ContractError::InvalidInput)
            );
        });
    }

    #[test]
    fn attach_dag_rejects_non_owner_and_missing_grant() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);
        env.as_contract(&contract_id, || {
            assert_eq!(
                attach_dag(&env, &owner, 1, SorobanVec::new(&env)),
                Err(ContractError::GrantNotFound)
            );
        });

        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));
        });

        env.as_contract(&contract_id, || {
            assert_eq!(
                attach_dag(&env, &stranger, 1, SorobanVec::new(&env)),
                Err(ContractError::Unauthorized)
            );
        });
    }

    // ── unblocked_milestones / dependents_of ────────────────────────────────

    #[test]
    fn unblocked_milestones_tracks_dag_readiness() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));

            let mut deps = SorobanVec::new(&env);
            deps.push_back(dep(&env, 1, &[0]));
            deps.push_back(dep(&env, 2, &[0]));
            deps.push_back(dep(&env, 3, &[1, 2]));
            attach_dag(&env, &owner, 1, deps).unwrap();

            // Nothing approved yet -> only the root (0) is unblocked.
            let unblocked = unblocked_milestones(&env, 1);
            assert_eq!(unblocked.len(), 1);
            assert_eq!(unblocked.get(0).unwrap(), 0);

            // Approve milestone 0 -> milestones 1 and 2 open up, 3 stays blocked.
            Storage::set_milestone(
                &env,
                1,
                0,
                &make_milestone(&env, 0, MilestoneState::Approved),
            );
            let unblocked = unblocked_milestones(&env, 1);
            assert_eq!(unblocked.len(), 2);
            assert_eq!(unblocked.get(0).unwrap(), 1);
            assert_eq!(unblocked.get(1).unwrap(), 2);
        });
    }

    #[test]
    fn dependents_of_returns_direct_children() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(StellarGrantsContract, ());
        let owner = Address::generate(&env);
        env.as_contract(&contract_id, || {
            Storage::set_grant(&env, 1, &make_grant(&env, &owner, 4));

            let mut deps = SorobanVec::new(&env);
            deps.push_back(dep(&env, 1, &[0]));
            deps.push_back(dep(&env, 2, &[0]));
            deps.push_back(dep(&env, 3, &[1]));
            attach_dag(&env, &owner, 1, deps).unwrap();

            let children_of_0 = dependents_of(&env, 1, 0);
            assert_eq!(children_of_0.len(), 2);
            assert_eq!(children_of_0.get(0).unwrap(), 1);
            assert_eq!(children_of_0.get(1).unwrap(), 2);

            let children_of_1 = dependents_of(&env, 1, 1);
            assert_eq!(children_of_1.len(), 1);
            assert_eq!(children_of_1.get(0).unwrap(), 3);

            // Leaf node has no dependents.
            assert_eq!(dependents_of(&env, 1, 3).len(), 0);
        });
    }

    #[test]
    fn dependents_of_empty_without_dag() {
        let env = Env::default();
        let contract_id = env.register(StellarGrantsContract, ());
        env.as_contract(&contract_id, || {
            assert_eq!(dependents_of(&env, 1, 0).len(), 0);
        });
    }

    // ── topological_order ──────────────────────────────────────────────────

    #[test]
    fn topological_order_linear_chain() {
        let env = Env::default();
        let mut deps = SorobanVec::new(&env);
        deps.push_back(dep(&env, 1, &[0]));
        deps.push_back(dep(&env, 2, &[1]));
        deps.push_back(dep(&env, 3, &[2]));

        let order = topological_order(&env, &deps, 4).unwrap();
        assert_eq!(order.len(), 4);
        assert_eq!(order.get(0).unwrap(), 0);
        assert_eq!(order.get(3).unwrap(), 3);
    }

    #[test]
    fn topological_order_rejects_cycle() {
        let env = Env::default();
        let mut deps = SorobanVec::new(&env);
        deps.push_back(dep(&env, 0, &[1]));
        deps.push_back(dep(&env, 1, &[0]));

        assert_eq!(
            topological_order(&env, &deps, 2),
            Err(ContractError::InvalidInput)
        );
    }
}
