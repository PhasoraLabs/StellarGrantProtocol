use soroban_sdk::{Address, Env, String, Vec};
use crate::types::{TemplateCategory, MilestoneTemplate, ContractError};
use crate::storage::keys::DataKey;

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
    
    env.storage().persistent().set(&DataKey::MilestoneTemplate(id), &template);
    
    let mut owner_templates: Vec<u64> = env.storage().persistent().get(&DataKey::TemplatesByOwner(owner.clone())).unwrap_or_else(|| Vec::new(env));
    owner_templates.push_back(id);
    env.storage().persistent().set(&DataKey::TemplatesByOwner(owner), &owner_templates);
    
    Ok(id)
}

pub fn create_from_templates(env: &Env, caller: Address, template_ids: Vec<u64>, total_amount: i128) -> Result<Vec<(String, i128)>, ContractError> {
    caller.require_auth();
    
    let mut results = Vec::new(env);
    
    for id in template_ids.iter() {
        let mut template = get_template(env, id).ok_or(ContractError::InvalidState)?; 
        if !template.is_public && template.owner != caller {
            return Err(ContractError::Unauthorized);
        }
        
        template.use_count += 1;
        env.storage().persistent().set(&DataKey::MilestoneTemplate(id), &template);
        
        let amount = (total_amount * (template.default_amount_pct as i128)) / 100;
        results.push_back((template.description.clone(), amount));
    }
    
    Ok(results)
}

pub fn get_template(env: &Env, id: u64) -> Option<MilestoneTemplate> {
    env.storage().persistent().get(&DataKey::MilestoneTemplate(id))
}

pub fn templates_by_owner(env: &Env, owner: Address) -> Vec<u64> {
    env.storage().persistent().get(&DataKey::TemplatesByOwner(owner)).unwrap_or_else(|| Vec::new(env))
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
    
    env.storage().persistent().remove(&DataKey::MilestoneTemplate(id));
    
    let owner_templates = templates_by_owner(env, caller.clone());
    let mut new_templates = Vec::new(env);
    for tid in owner_templates.iter() {
        if tid != id {
            new_templates.push_back(tid);
        }
    }
    env.storage().persistent().set(&DataKey::TemplatesByOwner(caller), &new_templates);
    
    Ok(())
}
