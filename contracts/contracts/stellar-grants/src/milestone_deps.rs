use crate::storage::Storage;
use crate::types::{ContractError, MilestoneDag, MilestoneDependency, MilestoneState};
use soroban_sdk::{Address, Env, Vec as SorobanVec};

/// Check if a milestone can be submitted by verifying all dependencies are satisfied.
/// A milestone can be submitted if all previous milestones have been approved.
pub fn can_submit(env: &Env, grant_id: u64, milestone_idx: u32) -> Result<(), ContractError> {
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
