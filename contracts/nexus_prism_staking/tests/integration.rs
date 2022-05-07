use cosmwasm_std::{Addr, MessageInfo, Response, to_binary, Uint128};
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage};
use terra_multi_test::{App, BankKeeper, ContractWrapper, Executor, TerraMockQuerier};
use nexus_prism_staking::contract::{execute, instantiate, query};
use nexus_prism_staking::commands::claim_rewards;
use nexus_prism_protocol::{
    common::query_token_balance,
    staking::{
        AnyoneMsg, ExecuteMsg, GovernanceMsg, InstantiateMsg, MigrateMsg, QueryMsg,
        RewardOperatorMsg, StakeOperatorMsg,
    },
};
use nexus_prism_protocol::staking::StateResponse;
use std::{println as info, println as warn};

fn mock_app() -> App {
    let api = MockApi::default();
    let env = mock_env();
    let bank = BankKeeper::new();
    let storage = MockStorage::new();
    let tmq = TerraMockQuerier::new(MockQuerier::new(&[]));

    App::new(api, env.block, bank, storage, tmq)
}

fn init_contracts(app: &mut App) -> (Addr, InstantiateMsg) {
    let owner = Addr::unchecked("contract_owner");

    // instantiate staking
    let staking_contract = Box::new(ContractWrapper::new(
        execute,
        instantiate,
        query,
    ));

    let staking_code_id = app.store_code(staking_contract);

    let msg = InstantiateMsg{
        governance: "governance_addr".to_string(),
        staking_token: "governance_addr".to_string(),
        stake_operator: None,
        reward_token: "governance_addr".to_string(),
        reward_operator: "governance_addr".to_string(),
        xprism_token: None,
        prism_governance: Option::from("governance_addr".to_string()),
        nexprism_xprism_pair: None,
    };

    let staking_instance = app
        .instantiate_contract(
            staking_code_id,
            owner.clone(),
            &msg,
            &[],
            String::from("STAKING"),
            None,
        )
        .unwrap();

    (
        staking_instance,
        msg
    )
}

#[test]
fn proper_initialization() {
    let mut app = mock_app();
    let (staking_instance, _) = init_contracts(&mut app);

    // check state
    let resp: StateResponse = app
        .wrap()
        .query_wasm_smart(&staking_instance, &QueryMsg::State {})
        .unwrap();

    assert_eq!(Uint128::zero(), resp.staking_total_balance);
    assert_eq!(Uint128::zero(), resp.virtual_reward_balance);
}

// #[test]
// fn test_claim_rewards_logic() {
//     let mut deps = mock_dependencies(&[]);
//     let msg_info = mock_info("owner", &[]);
//
//     // msg = MessageInfo{
//     //     sender: (Addr("some_addr".parse().unwrap())),
//     //     funds: vec![]
//     // };
//
//     claim_rewards(
//         deps.as_mut(),
//         mock_env(),
//         msg_info,
//         Some("some_recipient".parse().unwrap())
//     ).unwrap();
//
//     // info!("{}", result.unwrap().data.unwrap().to_string());
//     assert_eq!(true, true)
// }

// TODO:
#[test]
fn claim_rewards_test() {
    let mut app = mock_app();
    let (staking_instance, init_msg) = init_contracts(&mut app);

    // claim rewards
    app.execute_contract(
        Addr::unchecked("some_sender"),
        staking_instance.clone(),
        &ExecuteMsg::Anyone {
            anyone_msg: AnyoneMsg::ClaimRewards { recipient: None }
        },
        &[],
    )
    .unwrap();
}