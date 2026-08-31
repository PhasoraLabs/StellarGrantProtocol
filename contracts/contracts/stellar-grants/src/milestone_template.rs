use crate::storage::keys::DataKey;
use crate::types::{ContractError, MilestoneTemplate, TemplateCategory};
use soroban_sdk::{Address, Env, String, Vec};

pub fn save_template(
    env: &Env,
    owner: Address,
    name: String,
    description: String,
    category: TemplateCategory,
    default_amount_pct: u32,
    is_public: bool,
) -> Result<u64, ContractError> {
    owner.require_auth();

    let id_key = DataKey::TemplateCounter;
    let mut id: u64 = env.storage().persistent().get(&id_key).unwrap_or(0);
    id += 1;
    env.storage().persistent().set(&id_key, &id);

    let template = MilestoneTemplate {
        id,
        owner: owner.clone(),
        name,
        description,
        category,
        default_amount_pct,
        is_public,
        use_count: 0,
    };

    env.storage()
        .persistent()
        .set(&DataKey::MilestoneTemplate(id), &template);

    let mut owner_templates: Vec<u64> = env
        .storage()
        .persistent()
        .get(&DataKey::TemplatesByOwner(owner.clone()))
        .unwrap_or_else(|| Vec::new(env));
    owner_templates.push_back(id);
    env.storage()
        .persistent()
        .set(&DataKey::TemplatesByOwner(owner), &owner_templates);

    Ok(id)
}

pub fn create_from_templates(
    env: &Env,
    caller: Address,
    template_ids: Vec<u64>,
    total_amount: i128,
) -> Result<Vec<(String, i128)>, ContractError> {
    caller.require_auth();

    let mut results = Vec::new(env);

    for id in template_ids.iter() {
        let mut template = get_template(env, id).ok_or(ContractError::InvalidState)?;
        if !template.is_public && template.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        template.use_count += 1;
        env.storage()
            .persistent()
            .set(&DataKey::MilestoneTemplate(id), &template);

        let amount = (total_amount * (template.default_amount_pct as i128)) / 100;
        results.push_back((template.description.clone(), amount));
    }

    Ok(results)
}

pub fn get_template(env: &Env, id: u64) -> Option<MilestoneTemplate> {
    env.storage()
        .persistent()
        .get(&DataKey::MilestoneTemplate(id))
}

pub fn templates_by_owner(env: &Env, owner: Address) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::TemplatesByOwner(owner))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn public_templates(env: &Env, limit: u32) -> Vec<u64> {
    let mut results = Vec::new(env);
    let id_key = DataKey::TemplateCounter;
    let max_id: u64 = env.storage().persistent().get(&id_key).unwrap_or(0);

    let mut count = 0;
    for id in (1..=max_id).rev() {
        if let Some(template) = get_template(env, id) {
            if template.is_public {
                results.push_back(id);
                count += 1;
                if count >= limit {
                    break;
                }
            }
        }
    }
    results
}

pub fn delete_template(env: &Env, caller: Address, id: u64) -> Result<(), ContractError> {
    caller.require_auth();
    let template = get_template(env, id).ok_or(ContractError::InvalidState)?;
    if template.owner != caller {
        return Err(ContractError::Unauthorized);
    }
    if template.use_count > 0 {
        return Err(ContractError::InvalidState);
    }

    env.storage()
        .persistent()
        .remove(&DataKey::MilestoneTemplate(id));

    let owner_templates = templates_by_owner(env, caller.clone());
    let mut new_templates = Vec::new(env);
    for tid in owner_templates.iter() {
        if tid != id {
            new_templates.push_back(tid);
        }
    }
    env.storage()
        .persistent()
        .set(&DataKey::TemplatesByOwner(caller), &new_templates);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    // `save_template` / `create_from_templates` / `delete_template` each call
    // `require_auth`, so every call runs in its own contract frame.
    fn setup(env: &Env) -> Address {
        env.mock_all_auths();
        env.register(crate::StellarGrantsContract, ())
    }

    fn save(env: &Env, owner: &Address, name: &str, desc: &str, pct: u32, is_public: bool) -> u64 {
        save_template(
            env,
            owner.clone(),
            String::from_str(env, name),
            String::from_str(env, desc),
            TemplateCategory::Development,
            pct,
            is_public,
        )
        .unwrap()
    }

    fn ids_of(env: &Env, ids: &[u64]) -> Vec<u64> {
        let mut v = Vec::new(env);
        for id in ids {
            v.push_back(*id);
        }
        v
    }

    #[test]
    fn save_then_create_round_trip() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);

        let t1 = env.as_contract(&cid, || save(&env, &owner, "T1", "desc1", 60, true));
        let t2 = env.as_contract(&cid, || save(&env, &owner, "T2", "desc2", 40, true));
        assert_eq!((t1, t2), (1, 2));

        env.as_contract(&cid, || {
            let out =
                create_from_templates(&env, owner.clone(), ids_of(&env, &[t1, t2]), 1_000).unwrap();
            assert_eq!(out.len(), 2);
            let (d0, a0) = out.get(0).unwrap();
            let (d1, a1) = out.get(1).unwrap();
            assert_eq!(d0, String::from_str(&env, "desc1"));
            assert_eq!(a0, 600);
            assert_eq!(d1, String::from_str(&env, "desc2"));
            assert_eq!(a1, 400);
            assert_eq!(get_template(&env, t1).unwrap().use_count, 1);
        });
    }

    #[test]
    fn delete_template_is_owner_gated() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);

        let id = env.as_contract(&cid, || save(&env, &owner, "T", "d", 50, true));

        env.as_contract(&cid, || {
            assert_eq!(
                delete_template(&env, stranger.clone(), id),
                Err(ContractError::Unauthorized)
            );
        });
        env.as_contract(&cid, || {
            delete_template(&env, owner.clone(), id).unwrap();
            assert!(get_template(&env, id).is_none());
        });
    }

    #[test]
    fn deleted_template_cannot_be_used() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);

        let id = env.as_contract(&cid, || save(&env, &owner, "T", "d", 50, false));
        env.as_contract(&cid, || {
            delete_template(&env, owner.clone(), id).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(
                create_from_templates(&env, owner.clone(), ids_of(&env, &[id]), 1_000),
                Err(ContractError::InvalidState)
            );
        });
    }

    #[test]
    fn used_template_cannot_be_deleted() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);

        let id = env.as_contract(&cid, || save(&env, &owner, "T", "d", 50, true));
        env.as_contract(&cid, || {
            create_from_templates(&env, owner.clone(), ids_of(&env, &[id]), 1_000).unwrap();
        });
        env.as_contract(&cid, || {
            assert_eq!(
                delete_template(&env, owner.clone(), id),
                Err(ContractError::InvalidState)
            );
        });
    }

    #[test]
    fn private_template_only_usable_by_owner() {
        let env = Env::default();
        let cid = setup(&env);
        let owner = Address::generate(&env);
        let stranger = Address::generate(&env);

        let id = env.as_contract(&cid, || save(&env, &owner, "T", "d", 50, false));

        env.as_contract(&cid, || {
            assert_eq!(
                create_from_templates(&env, stranger.clone(), ids_of(&env, &[id]), 1_000),
                Err(ContractError::Unauthorized)
            );
        });
        env.as_contract(&cid, || {
            assert!(create_from_templates(&env, owner.clone(), ids_of(&env, &[id]), 1_000).is_ok());
        });
    }

    #[test]
    fn visibility_filters() {
        let env = Env::default();
        let cid = setup(&env);
        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);

        let t1 = env.as_contract(&cid, || save(&env, &owner1, "pub1", "d", 50, true));
        let t2 = env.as_contract(&cid, || save(&env, &owner1, "priv", "d", 50, false));
        let t3 = env.as_contract(&cid, || save(&env, &owner2, "pub2", "d", 50, true));

        env.as_contract(&cid, || {
            let has = |v: &Vec<u64>, id: u64| v.iter().any(|x| x == id);

            let by_owner1 = templates_by_owner(&env, owner1.clone());
            assert_eq!(by_owner1.len(), 2);
            assert!(has(&by_owner1, t1) && has(&by_owner1, t2));

            let public = public_templates(&env, 10);
            assert_eq!(public.len(), 2);
            assert!(has(&public, t1) && has(&public, t3));
            assert!(!has(&public, t2));
        });
    }
}
