/// Milestone dependency graph (DAG) module (issue #595).
/// Models inter-milestone dependencies, enforces submission ordering,
/// and validates the DAG for cycles at attachment time using Kahn's algorithm
/// (iterative — no recursion to avoid Soroban stack overflow).
use soroban_sdk::{Address, Env, Vec};

use crate::errors::ContractError;
use crate::storage::Storage;
use crate::types::{MilestoneDag, MilestoneDependency, MilestoneState};

/// Attach a dependency DAG to a grant. Owner-only, must be called before any milestone
/// submissions. Validates the DAG is acyclic before persisting.
pub fn attach_dag(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    deps: Vec<MilestoneDependency>,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }
    if Storage::get_milestone_dag(env, grant_id).is_some() {
        return Err(ContractError::DagAlreadyAttached);
    }

    validate_dag(env, &deps, grant.total_milestones)?;

    let dag = MilestoneDag {
        grant_id,
        dependencies: deps,
        is_valid: true,
    };
    Storage::set_milestone_dag(env, grant_id, &dag);
    Ok(())
}

/// Check whether milestone `idx` may be submitted given current milestone states.
/// Returns `Ok(())` when all declared dependencies are `Approved` or `Paid`.
pub fn can_submit(env: &Env, grant_id: u64, idx: u32) -> Result<(), ContractError> {
    let dag = match Storage::get_milestone_dag(env, grant_id) {
        Some(d) => d,
        None => return Ok(()), // no DAG attached — no ordering constraint
    };

    for dep in dag.dependencies.iter() {
        if dep.milestone_idx != idx {
            continue;
        }
        for required_idx in dep.depends_on.iter() {
            let milestone = Storage::get_milestone(env, grant_id, required_idx)
                .ok_or(ContractError::MilestoneNotFound)?;
            match milestone.state {
                MilestoneState::Approved | MilestoneState::Paid => {}
                _ => return Err(ContractError::DependencyNotSatisfied),
            }
        }
    }
    Ok(())
}

/// Validate that the given dependency list is acyclic using Kahn's algorithm (iterative BFS).
/// Also rejects: self-dependencies and references to out-of-bounds milestone indices.
pub fn validate_dag(
    env: &Env,
    deps: &Vec<MilestoneDependency>,
    total_milestones: u32,
) -> Result<(), ContractError> {
    // Build in-degree array (index = milestone_idx, value = number of incoming edges)
    let mut in_degree: Vec<u32> = Vec::new(env);
    for _ in 0..total_milestones {
        in_degree.push_back(0u32);
    }

    for dep in deps.iter() {
        if dep.milestone_idx >= total_milestones {
            return Err(ContractError::InvalidInput);
        }
        for required in dep.depends_on.iter() {
            if required >= total_milestones {
                return Err(ContractError::InvalidInput);
            }
            if required == dep.milestone_idx {
                return Err(ContractError::DagCycleDetected); // self-dependency
            }
        }
        let curr = in_degree.get(dep.milestone_idx).unwrap();
        in_degree.set(dep.milestone_idx, curr + dep.depends_on.len() as u32);
    }

    // Kahn's: start with all nodes of in-degree 0
    let mut queue: Vec<u32> = Vec::new(env);
    for i in 0..total_milestones {
        if in_degree.get(i).unwrap() == 0 {
            queue.push_back(i);
        }
    }

    let mut processed = 0u32;
    let mut queue_idx = 0u32;
    while queue_idx < queue.len() {
        let node = queue.get(queue_idx).unwrap();
        queue_idx += 1;
        processed += 1;

        // For every milestone that depends on `node`, decrement its in-degree
        for dep in deps.iter() {
            if dep.depends_on.contains(node) {
                let d = in_degree.get(dep.milestone_idx).unwrap();
                let new_d = d - 1;
                in_degree.set(dep.milestone_idx, new_d);
                if new_d == 0 {
                    queue.push_back(dep.milestone_idx);
                }
            }
        }
    }

    if processed != total_milestones {
        return Err(ContractError::DagCycleDetected);
    }
    Ok(())
}

/// Return all milestone indices that are currently unblocked (all dependencies satisfied).
pub fn unblocked_milestones(env: &Env, grant_id: u64) -> Vec<u32> {
    let grant = match Storage::get_grant(env, grant_id) {
        Some(g) => g,
        None => return Vec::new(env),
    };
    let dag = match Storage::get_milestone_dag(env, grant_id) {
        Some(d) => d,
        None => {
            // No DAG — all pending milestones are unblocked
            let mut result = Vec::new(env);
            for i in 0..grant.total_milestones {
                if let Some(m) = Storage::get_milestone(env, grant_id, i) {
                    if m.state == MilestoneState::Pending {
                        result.push_back(i);
                    }
                }
            }
            return result;
        }
    };

    let mut result = Vec::new(env);
    for i in 0..grant.total_milestones {
        let ms = match Storage::get_milestone(env, grant_id, i) {
            Some(m) => m,
            None => continue,
        };
        if ms.state != MilestoneState::Pending {
            continue;
        }
        if can_submit(env, grant_id, i).is_ok() {
            result.push_back(i);
        }
    }
    result
}

/// Return all milestone indices that directly list `idx` in their `depends_on`.
pub fn dependents_of(env: &Env, grant_id: u64, idx: u32) -> Vec<u32> {
    let dag = match Storage::get_milestone_dag(env, grant_id) {
        Some(d) => d,
        None => return Vec::new(env),
    };
    let mut result = Vec::new(env);
    for dep in dag.dependencies.iter() {
        if dep.depends_on.contains(idx) {
            result.push_back(dep.milestone_idx);
        }
    }
    result
}

/// Return the stored DAG for a grant, if any.
pub fn get_dag(env: &Env, grant_id: u64) -> Option<MilestoneDag> {
    Storage::get_milestone_dag(env, grant_id)
}

/// Topological sort of milestones using Kahn's algorithm.
/// Returns the milestones in a valid execution order, or `DagCycleDetected` if the graph
/// contains a cycle.
pub fn topological_order(
    env: &Env,
    deps: &Vec<MilestoneDependency>,
    total: u32,
) -> Result<Vec<u32>, ContractError> {
    let mut in_degree: Vec<u32> = Vec::new(env);
    for _ in 0..total {
        in_degree.push_back(0u32);
    }
    for dep in deps.iter() {
        let curr = in_degree.get(dep.milestone_idx).unwrap();
        in_degree.set(dep.milestone_idx, curr + dep.depends_on.len() as u32);
    }

    let mut queue: Vec<u32> = Vec::new(env);
    for i in 0..total {
        if in_degree.get(i).unwrap() == 0 {
            queue.push_back(i);
        }
    }

    let mut result: Vec<u32> = Vec::new(env);
    let mut queue_idx = 0u32;
    while queue_idx < queue.len() {
        let node = queue.get(queue_idx).unwrap();
        queue_idx += 1;
        result.push_back(node);

        for dep in deps.iter() {
            if dep.depends_on.contains(node) {
                let d = in_degree.get(dep.milestone_idx).unwrap();
                let new_d = d - 1;
                in_degree.set(dep.milestone_idx, new_d);
                if new_d == 0 {
                    queue.push_back(dep.milestone_idx);
                }
            }
        }
    }

    if result.len() != total {
        return Err(ContractError::DagCycleDetected);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, Vec};

    fn make_deps(env: &Env, pairs: &[(u32, &[u32])]) -> Vec<MilestoneDependency> {
        let mut deps = Vec::new(env);
        for (idx, depends_on) in pairs {
            let mut d_vec: Vec<u32> = Vec::new(env);
            for &d in *depends_on {
                d_vec.push_back(d);
            }
            deps.push_back(MilestoneDependency {
                milestone_idx: *idx,
                depends_on: d_vec,
            });
        }
        deps
    }

    #[test]
    fn linear_chain_is_valid() {
        let env = Env::default();
        // 0 <- 1 <- 2 (2 depends on 1 depends on 0)
        let deps = make_deps(&env, &[(1, &[0]), (2, &[1])]);
        assert!(validate_dag(&env, &deps, 3).is_ok());
    }

    #[test]
    fn diamond_is_valid() {
        let env = Env::default();
        // A(0) -> B(1), A(0) -> C(2), B+C -> D(3)
        let deps = make_deps(&env, &[(1, &[0]), (2, &[0]), (3, &[1, 2])]);
        assert!(validate_dag(&env, &deps, 4).is_ok());
    }

    #[test]
    fn cycle_detected() {
        let env = Env::default();
        // 0 -> 1 -> 0 (cycle)
        let deps = make_deps(&env, &[(1, &[0]), (0, &[1])]);
        assert_eq!(
            validate_dag(&env, &deps, 2),
            Err(ContractError::DagCycleDetected)
        );
    }

    #[test]
    fn self_dependency_rejected() {
        let env = Env::default();
        let deps = make_deps(&env, &[(1, &[1])]);
        assert_eq!(
            validate_dag(&env, &deps, 2),
            Err(ContractError::DagCycleDetected)
        );
    }

    #[test]
    fn topological_sort_linear_chain() {
        let env = Env::default();
        // 0 <- 1 <- 2
        let deps = make_deps(&env, &[(1, &[0]), (2, &[1])]);
        let order = topological_order(&env, &deps, 3).unwrap();
        // 0 must appear before 1, and 1 before 2
        let pos_0 = order.iter().position(|x| x == 0).unwrap();
        let pos_1 = order.iter().position(|x| x == 1).unwrap();
        let pos_2 = order.iter().position(|x| x == 2).unwrap();
        assert!(pos_0 < pos_1);
        assert!(pos_1 < pos_2);
    }

    #[test]
    fn unblocked_milestones_no_dag() {
        let env = Env::default();
        env.mock_all_auths();
        // Without a DAG, unblocked_milestones returns empty (grant not found)
        let result = unblocked_milestones(&env, 999);
        assert_eq!(result.len(), 0);
    }
}
