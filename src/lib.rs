extern crate hmcdk;
use hmcdk::api;
use hmcdk::error;
use hmcdk::prelude::*;

pub static TOTAL_SUPPLY: i64 = 100_000 * 10;

#[contract]
pub fn init() -> R<i32> {
    let sender = api::get_sender()?;
    api::write_state(&sender, &TOTAL_SUPPLY.to_bytes());
    Ok(None)
}

#[contract]
pub fn transfer() -> R<i64> {
    let to: Address = api::get_arg(0)?;
    let amount: i64 = api::get_arg(1)?;
    let sender = api::get_sender()?;

    Ok(Some(_transfer(&sender, &to, amount)?))
}

#[contract]
pub fn approve() -> R<bool> {
    let sender = api::get_sender()?;
    let spender: Address = api::get_arg(0)?;
    let value: i64 = api::get_arg(1)?;
    let key = make_approve_key(&sender, &spender);
    api::write_state(&key, &value.to_be_bytes());

    Ok(Some(true))
}

#[contract]
pub fn allowance() -> R<i64> {
    let owner: Address = api::get_arg(0)?;
    let spender: Address = api::get_arg(1)?;

    let key = make_approve_key(&owner, &spender);
    Ok(Some(api::read_state(&key)?))
}

fn make_approve_key(owner: &Address, spender: &Address) -> Vec<u8> {
    make_key_by_parts(vec![b"allowed", &owner.to_bytes(), &spender.to_bytes()])
}

fn make_key_by_parts(parts: Vec<&[u8]>) -> Vec<u8> {
    parts.join(&b'/')
}

fn _transfer(sender: &Address, to: &Address, amount: i64) -> Result<i64, Error> {
    let from_balance = _balance_of(&sender)?;
    if from_balance < amount {
        return Err(error::from_str(format!(
            "error: {} < {}",
            from_balance, amount
        )));
    }
    api::write_state(&sender.to_bytes(), &(from_balance - amount).to_bytes());
    let to_balance = _balance_of(&to).unwrap_or(0);
    let to_amount = to_balance + amount;
    api::write_state(&to.to_bytes(), &to_amount.to_bytes());
    api::emit_event(
        "Transfer",
        format!("from={:X?} to={:X?} amount={}", sender, to, amount).as_bytes(),
    )?;

    Ok(to_amount)
}

#[allow(non_snake_case)]
#[contract]
pub fn balanceOf() -> R<i64> {
    let sender = api::get_sender()?;
    Ok(Some(_balance_of(&sender)?))
}

fn _balance_of(addr: &Address) -> Result<i64, Error> {
    api::read_state(addr)
}

#[allow(non_snake_case)]
#[contract]
pub fn transferFrom() -> R<i64> {
    let sender = api::get_sender()?;
    let from: Address = api::get_arg(0)?;
    let to: Address = api::get_arg(1)?;
    let value: i64 = api::get_arg(2)?;

    let key = make_approve_key(&from, &sender);
    let allowed: i64 = api::read_state(&key)?;

    if value > allowed {
        return Err(error::from_str("allowed value is insuficient"));
    }

    api::write_state(&key, &(allowed - value).to_bytes());
    Ok(Some(_transfer(&from, &to, value)?))
}

#[cfg(test)]
mod tests {
    extern crate hmemu;
    use super::*;
    use hmemu::types::ArgsBuilder;

    const SENDER1_ADDR: Address = *b"00000000000000000001";
    const SENDER2_ADDR: Address = *b"00000000000000000002";

    #[test]
    fn test_init() {
        hmemu::run_process(|| {
            let _ =
                hmemu::call_contract(&SENDER1_ADDR, ArgsBuilder::new().convert_to_vec(), || {
                    Ok(init())
                })?;
            {
                let v: i64 = api::read_state(&SENDER1_ADDR)?;
                assert_eq!(TOTAL_SUPPLY, v);
            }
            Ok(0)
        })
        .unwrap();
    }

    #[test]
    fn test_transfer() {
        hmemu::run_process(|| {
            {
                let _ = hmemu::call_contract(&SENDER1_ADDR, vec![], || Ok(init())).unwrap();
            }

            {
                let args = {
                    let mut args = ArgsBuilder::new();
                    args.push(SENDER2_ADDR);
                    args.push(100i64);
                    args.convert_to_vec()
                };
                let balance: i64 = hmemu::call_contract(&SENDER1_ADDR, args, || Ok(transfer()?))
                    .unwrap()
                    .unwrap();
                assert_eq!(100, balance);
            }

            {
                let b1: i64 = hmemu::call_contract(&SENDER1_ADDR, vec![], || Ok(balanceOf()?))
                    .unwrap()
                    .unwrap();
                assert_eq!(TOTAL_SUPPLY - 100, b1);

                let b2: i64 = hmemu::call_contract(&SENDER2_ADDR, vec![], || Ok(balanceOf()?))
                    .unwrap()
                    .unwrap();
                assert_eq!(100, b2);
            }

            {
                let args = {
                    let mut args = ArgsBuilder::new();
                    args.push(SENDER1_ADDR);
                    args.push(100i64);
                    args.convert_to_vec()
                };
                let balance: i64 = hmemu::call_contract(&SENDER2_ADDR, args, || Ok(transfer()?))
                    .unwrap()
                    .unwrap();
                assert_eq!(TOTAL_SUPPLY, balance);
            }

            {
                let b1: i64 = hmemu::call_contract(&SENDER1_ADDR, vec![], || Ok(balanceOf()?))
                    .unwrap()
                    .unwrap();
                assert_eq!(TOTAL_SUPPLY, b1);

                let b2: i64 = hmemu::call_contract(&SENDER2_ADDR, vec![], || Ok(balanceOf()?))
                    .unwrap()
                    .unwrap();
                assert_eq!(0, b2);
            }

            Ok(0)
        })
        .unwrap();
    }

    #[test]
    fn test_approve() {
        hmemu::run_process(|| {
            let _ = hmemu::call_contract(&SENDER1_ADDR, vec![], || Ok(init()))?;

            {
                let args = {
                    let mut args = ArgsBuilder::new();
                    args.push(SENDER2_ADDR);
                    args.push(100i64);
                    args.convert_to_vec()
                };
                hmemu::call_contract(&SENDER1_ADDR, args, || {
                    assert_eq!(Some(true), approve()?);
                    Ok(0)
                })
                .unwrap();
            }

            {
                let args = {
                    let mut args = ArgsBuilder::new();
                    args.push(SENDER1_ADDR);
                    args.push(SENDER2_ADDR);
                    args.convert_to_vec()
                };
                hmemu::call_contract(&SENDER1_ADDR, args, || {
                    assert_eq!(Some(100), allowance()?);
                    Ok(0)
                })
                .unwrap();
            }

            Ok(0)
        })
        .unwrap();
    }

    #[test]
    fn test_transfer_from() {
        hmemu::run_process(|| {
            let _ = hmemu::call_contract(&SENDER1_ADDR, vec![], || Ok(init()))?;

            {
                let args = {
                    let mut args = ArgsBuilder::new();
                    args.push(SENDER2_ADDR);
                    args.push(100i64);
                    args.convert_to_vec()
                };
                hmemu::call_contract(&SENDER1_ADDR, args, || {
                    assert_eq!(Some(true), approve()?);
                    Ok(0)
                })
                .unwrap();
            }

            {
                let args = {
                    let mut args = ArgsBuilder::new();
                    args.push(SENDER1_ADDR);
                    args.push(SENDER2_ADDR);
                    args.push(100i64);
                    args.convert_to_vec()
                };
                hmemu::call_contract(&SENDER2_ADDR, args, || Ok(transferFrom()?)).unwrap();
            }

            {
                let b1: i64 = hmemu::call_contract(&SENDER1_ADDR, vec![], || Ok(balanceOf()?))
                    .unwrap()
                    .unwrap();
                assert_eq!(TOTAL_SUPPLY - 100, b1);

                let b2: i64 = hmemu::call_contract(&SENDER2_ADDR, vec![], || Ok(balanceOf()?))
                    .unwrap()
                    .unwrap();
                assert_eq!(100, b2);
            }

            Ok(0)
        })
        .unwrap();
    }

}
