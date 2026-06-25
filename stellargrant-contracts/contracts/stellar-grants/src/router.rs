#![allow(non_snake_case)]

use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionCategory {
    GrantLifecycle,
    MilestoneManagement,
    EscrowOperation,
    GovernanceVote,
    ContributorManagement,
    AdminOperation,
    QueryOnly,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionContext {
    pub caller: Address,
    pub category: ActionCategory,
    pub grant_id: u32,
    pub milestone_id: u32,
    pub timestamp: u64,
}

pub trait RouterTrait {
    fn build_context(env: Env, caller: Address, category: ActionCategory) -> ActionContext;
    fn pre_call(env: Env, ctx: ActionContext);
    fn post_call(env: Env, ctx: ActionContext);
    fn dispatch(env: Env, ctx: ActionContext);
}

pub struct Router;

impl RouterTrait for Router {
    fn build_context(env: Env, caller: Address, category: ActionCategory) -> ActionContext {
        ActionContext {
            caller,
            category,
            grant_id: 0,
            milestone_id: 0,
            timestamp: env.ledger().timestamp(),
        }
    }
    
    fn pre_call(_env: Env, _ctx: ActionContext) {}
    
    fn post_call(_env: Env, _ctx: ActionContext) {}
    
    fn dispatch(env: Env, ctx: ActionContext) {
        Self::pre_call(env.clone(), ctx.clone());
        // execute
        Self::post_call(env, ctx);
    }
}
