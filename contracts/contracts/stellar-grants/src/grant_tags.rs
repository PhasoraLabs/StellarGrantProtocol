use crate::storage::Storage;
use crate::types::{ContractError, GrantCategory, GrantTag};
use soroban_sdk::{Address, Bytes, Env, String, Vec};

const MAX_FREEFORM_TAGS: u32 = 10;

pub fn create_category(
    env: &Env,
    admin: &Address,
    name: String,
    subcategories: Vec<String>,
) -> Result<u32, ContractError> {
    admin.require_auth();

    if let Some(current_admin) = crate::Storage::get_global_admin(env) {
        if current_admin != *admin {
            return Err(ContractError::Unauthorized);
        }
    }

    let mut categories = Storage::get_category_list(env);
    let id = categories.len();

    let category = GrantCategory {
        id,
        name,
        subcategories,
    };

    categories.push_back(category);
    Storage::set_category_list(env, &categories);
    Ok(id)
}

pub fn tag_grant(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    category_id: Option<u32>,
    subcategory: Option<String>,
    freeform_tags: Vec<String>,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant_v(env, grant_id);
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    if freeform_tags.len() > MAX_FREEFORM_TAGS {
        return Err(ContractError::InvalidInput);
    }

    if let Some(cat_id) = category_id {
        let categories = Storage::get_category_list(env);
        if !categories.iter().any(|c| c.id == cat_id) {
            return Err(ContractError::InvalidInput);
        }
    }

    let tag = GrantTag {
        grant_id,
        category_id,
        subcategory,
        freeform_tags: freeform_tags.clone(),
        tagged_by: owner.clone(),
        tagged_at: env.ledger().timestamp(),
    };

    // Clean up old indices if re-tagging
    if let Some(existing) = Storage::get_grant_tags(env, grant_id) {
        for old_tag in existing.freeform_tags.iter() {
            let hash = hash_tag(env, &old_tag);
            let index = Storage::get_tag_index(env, hash);
            let mut filtered = Vec::new(env);
            for idx_grant_id in index.iter() {
                if idx_grant_id != grant_id {
                    filtered.push_back(idx_grant_id);
                }
            }
            Storage::set_tag_index(env, hash, &filtered);
        }
        if let Some(old_cat) = existing.category_id {
            let mut cat_index = Storage::get_category_index(env, old_cat);
            let mut filtered = Vec::new(env);
            for idx_grant_id in cat_index.iter() {
                if idx_grant_id != grant_id {
                    filtered.push_back(idx_grant_id);
                }
            }
            Storage::set_category_index(env, old_cat, &filtered);
        }
    }

    Storage::set_grant_tags(env, &tag);

    if let Some(cat_id) = category_id {
        let mut cat_index = Storage::get_category_index(env, cat_id);
        let has_grant = cat_index.iter().any(|id| id == grant_id);
        if !has_grant {
            cat_index.push_back(grant_id);
            Storage::set_category_index(env, cat_id, &cat_index);
        }
    }

    for tag_str in freeform_tags.iter() {
        let hash = hash_tag(env, &tag_str);
        let mut index = Storage::get_tag_index(env, hash);
        let has_grant = index.iter().any(|id| id == grant_id);
        if !has_grant {
            index.push_back(grant_id);
            Storage::set_tag_index(env, hash, &index);
        }
    }

    Ok(())
}

pub fn update_tags(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    freeform_tags: Vec<String>,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant_v(env, grant_id);
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    if freeform_tags.len() > MAX_FREEFORM_TAGS {
        return Err(ContractError::InvalidInput);
    }

    let mut tag = Storage::get_grant_tags(env, grant_id).ok_or(ContractError::InvalidState)?;
    tag.freeform_tags = freeform_tags;
    Storage::set_grant_tags(env, &tag);
    Ok(())
}

pub fn get_tags(env: &Env, grant_id: u64) -> Option<GrantTag> {
    Storage::get_grant_tags(env, grant_id)
}

pub fn find_by_tag(env: &Env, tag: &String, offset: u32, limit: u32) -> Vec<u64> {
    let hash = hash_tag(env, tag);
    let index = Storage::get_tag_index(env, hash);
    let limit = (limit as usize).min(50);
    let offset = offset as usize;
    let mut result = Vec::new(env);

    for i in offset..offset + limit {
        if (i as u32) < index.len() {
            result.push_back(index.get(i as u32).unwrap());
        } else {
            break;
        }
    }

    result
}

pub fn find_by_category(env: &Env, category_id: u32, offset: u32, limit: u32) -> Vec<u64> {
    let categories = Storage::get_category_list(env);
    if !categories.iter().any(|c| c.id == category_id) {
        return Vec::new(env);
    }

    let index = Storage::get_category_index(env, category_id);
    let limit = (limit as usize).min(50);
    let offset = offset as usize;
    let mut result = Vec::new(env);

    for i in offset..offset + limit {
        if (i as u32) < index.len() {
            result.push_back(index.get(i as u32).unwrap());
        } else {
            break;
        }
    }

    result
}

pub fn list_categories(env: &Env) -> Vec<GrantCategory> {
    Storage::get_category_list(env)
}

pub fn remove_tag(
    env: &Env,
    owner: &Address,
    grant_id: u64,
    tag: &String,
) -> Result<(), ContractError> {
    owner.require_auth();

    let grant = Storage::get_grant_v(env, grant_id);
    if grant.owner != *owner {
        return Err(ContractError::Unauthorized);
    }

    let mut tag_obj = Storage::get_grant_tags(env, grant_id).ok_or(ContractError::InvalidState)?;

    let mut filtered_tags = Vec::new(env);
    for existing_tag in tag_obj.freeform_tags.iter() {
        if existing_tag != *tag {
            filtered_tags.push_back(existing_tag);
        }
    }
    tag_obj.freeform_tags = filtered_tags;
    Storage::set_grant_tags(env, &tag_obj);

    let hash = hash_tag(env, tag);
    let index = Storage::get_tag_index(env, hash);
    let mut filtered_index = Vec::new(env);
    for idx_grant_id in index.iter() {
        if idx_grant_id != grant_id {
            filtered_index.push_back(idx_grant_id);
        }
    }
    Storage::set_tag_index(env, hash, &filtered_index);

    Ok(())
}

fn hash_tag(env: &Env, tag: &String) -> u32 {
    let bytes: Bytes = tag.clone().into();
    let hash = env.crypto().keccak256(&bytes);
    let hash_bytes = hash.to_array();
    u32::from_be_bytes([hash_bytes[0], hash_bytes[1], hash_bytes[2], hash_bytes[3]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Grant, GrantStatus};
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, String, Vec};

    fn setup_grant(env: &Env, grant_id: u64, owner: &Address) {
        let grant = Grant {
            id: grant_id,
            owner: owner.clone(),
            title: String::from_str(env, "Test Grant"),
            description: String::from_str(env, "Description"),
            token: Address::generate(env),
            status: GrantStatus::Active,
            total_amount: 1000,
            milestone_amount: 1000,
            reviewers: Vec::new(env),
            total_milestones: 1,
            milestones_paid_out: 0,
            escrow_balance: 1000,
            funders: Vec::new(env),
            reason: None,
            timestamp: 0,
            require_compliance: None,
        };
        Storage::set_grant(env, grant_id, &grant);
    }

    #[test]
    fn test_hash_tag_same_length_different_content_no_collision() {
        let env = Env::default();
        let tag_a = String::from_str(&env, "rust");
        let tag_b = String::from_str(&env, "defi");
        assert_eq!(tag_a.len(), tag_b.len());
        assert_ne!(hash_tag(&env, &tag_a), hash_tag(&env, &tag_b));
    }

    #[test]
    fn test_hash_tag_identical_strings_same_hash() {
        let env = Env::default();
        let tag_a = String::from_str(&env, "soroban");
        let tag_b = String::from_str(&env, "soroban");
        assert_eq!(hash_tag(&env, &tag_a), hash_tag(&env, &tag_b));
    }

    #[test]
    fn test_find_by_tag_no_collision_for_same_length_tags() {
        let env = Env::default();
        env.mock_all_auths();

        let owner = Address::generate(&env);
        setup_grant(&env, 1, &owner);
        setup_grant(&env, 2, &owner);

        let mut tags1 = Vec::new(&env);
        tags1.push_back(String::from_str(&env, "rust"));
        tag_grant(&env, &owner, 1, None, None, tags1).unwrap();

        let mut tags2 = Vec::new(&env);
        tags2.push_back(String::from_str(&env, "defi"));
        tag_grant(&env, &owner, 2, None, None, tags2).unwrap();

        let rust_results = find_by_tag(&env, &String::from_str(&env, "rust"), 0, 50);
        let defi_results = find_by_tag(&env, &String::from_str(&env, "defi"), 0, 50);

        assert_eq!(rust_results.len(), 1);
        assert_eq!(rust_results.get(0).unwrap(), 1);
        assert_eq!(defi_results.len(), 1);
        assert_eq!(defi_results.get(0).unwrap(), 2);
    }

    #[test]
    fn test_find_by_category_returns_tagged_grants() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let owner = Address::generate(&env);

        let subs = Vec::new(&env);
        let cat_id =
            create_category(&env, &admin, String::from_str(&env, "Infrastructure"), subs).unwrap();

        setup_grant(&env, 10, &owner);
        setup_grant(&env, 20, &owner);
        setup_grant(&env, 30, &owner);

        let tags = Vec::new(&env);
        tag_grant(&env, &owner, 10, Some(cat_id), None, tags.clone()).unwrap();
        tag_grant(&env, &owner, 20, Some(cat_id), None, tags.clone()).unwrap();

        let results = find_by_category(&env, cat_id, 0, 50);
        assert_eq!(results.len(), 2);
        assert_eq!(results.get(0).unwrap(), 10);
        assert_eq!(results.get(1).unwrap(), 20);

        let results_30 = find_by_category(&env, cat_id, 0, 50);
        assert!(!results_30.iter().any(|id| id == 30));
    }

    #[test]
    fn test_find_by_category_pagination() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let owner = Address::generate(&env);

        let subs = Vec::new(&env);
        let cat_id = create_category(&env, &admin, String::from_str(&env, "DeFi"), subs).unwrap();

        let tags = Vec::new(&env);
        for i in 0..5u64 {
            setup_grant(&env, i, &owner);
            tag_grant(&env, &owner, i, Some(cat_id), None, tags.clone()).unwrap();
        }

        let page1 = find_by_category(&env, cat_id, 0, 2);
        assert_eq!(page1.len(), 2);
        assert_eq!(page1.get(0).unwrap(), 0);
        assert_eq!(page1.get(1).unwrap(), 1);

        let page2 = find_by_category(&env, cat_id, 2, 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page2.get(0).unwrap(), 2);
        assert_eq!(page2.get(1).unwrap(), 3);
    }

    #[test]
    fn test_find_by_category_invalid_category_returns_empty() {
        let env = Env::default();
        let results = find_by_category(&env, 999, 0, 50);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_category_index_cleaned_on_retag() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        Storage::set_global_admin(&env, &admin);
        let owner = Address::generate(&env);

        let subs = Vec::new(&env);
        let cat_a =
            create_category(&env, &admin, String::from_str(&env, "CatA"), subs.clone()).unwrap();
        let cat_b = create_category(&env, &admin, String::from_str(&env, "CatB"), subs).unwrap();

        setup_grant(&env, 1, &owner);
        let tags = Vec::new(&env);

        tag_grant(&env, &owner, 1, Some(cat_a), None, tags.clone()).unwrap();
        assert_eq!(find_by_category(&env, cat_a, 0, 50).len(), 1);

        tag_grant(&env, &owner, 1, Some(cat_b), None, tags).unwrap();
        assert_eq!(find_by_category(&env, cat_a, 0, 50).len(), 0);
        assert_eq!(find_by_category(&env, cat_b, 0, 50).len(), 1);
    }
}
