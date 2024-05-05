use cosmwasm_std::{BankMsg, DepsMut, Env, StdResult};

pub fn get_recover_msg(
    deps: DepsMut,
    env: Env,
    denoms: Vec<String>,
    to_address: String,
) -> StdResult<BankMsg> {
    // collect the coins we want to recover
    let mut recover_coins = vec![];
    for denom in denoms {
        let balance = deps
            .querier
            .query_balance(env.contract.address.to_string(), denom.as_str())?;
        if !balance.amount.is_zero() {
            recover_coins.push(balance);
        }
    }

    Ok(BankMsg::Send {
        to_address,
        amount: recover_coins,
    })
}
