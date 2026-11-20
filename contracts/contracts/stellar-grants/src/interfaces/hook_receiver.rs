use soroban_sdk::{contractclient, Bytes, Env};

/// Hook receiver interface invoked by Phasora on lifecycle events.
///
/// Expected signature: `fn on_hook(env: Env, event: u32, payload: Bytes)`.
#[contractclient(name = "HookReceiverClient")]
pub trait HookReceiver {
    fn on_hook(env: Env, event: u32, payload: Bytes);
}
