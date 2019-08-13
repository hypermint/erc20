extern crate hmc;

static TOTAL_SUPPLY: i64 = 100000 * 10;

#[no_mangle]
pub fn init() -> i32 {
    let sender = hmc::get_sender().unwrap();
    hmc::write_state(&sender, &TOTAL_SUPPLY.to_be_bytes());
    0
}

#[no_mangle]
pub fn transfer() -> i32 {
    match call_transfer() {
        Ok(v) => v,
        Err(e) => {
            hmc::revert(e);
            -1
        }
    }
}

#[no_mangle]
pub fn approve() -> i32 {
    match _approve() {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => {
            hmc::revert(e);
            -1
        }
    };
    0
}

#[no_mangle]
pub fn allowance() -> i32 {
    match _allowance() {
        Ok(v) => hmc::return_value(&v.to_be_bytes()),
        Err(e) => {
            hmc::revert(e);
            -1
        }
    }
}

fn bytes_to_i64(bs: &[u8]) -> i64 {
    let mut v: [u8; 8] = Default::default();
    v.copy_from_slice(bs);

    i64::from_be_bytes(v)
}

fn _allowance() -> Result<i64, String> {
    let owner = hmc::hex_to_bytes(hmc::get_arg_str(0)?.as_ref());
    let spender = hmc::hex_to_bytes(hmc::get_arg_str(1)?.as_ref());

    let key = make_approve_key(&owner, &spender);
    Ok(bytes_to_i64(&hmc::read_state(&key)?))
}

fn _approve() -> Result<bool, String> {
    let sender = hmc::get_sender()?;
    let spender = hmc::hex_to_bytes(hmc::get_arg_str(0)?.as_ref());
    let value = hmc::get_arg_str(1)?.parse::<i64>().unwrap();
    let key = make_approve_key(&sender, &spender);
    hmc::write_state(&key, &value.to_be_bytes());

    Ok(true)
}

fn make_approve_key(owner: &[u8], spender: &[u8]) -> Vec<u8> {
    make_key_by_parts(vec!["allowed".as_bytes(), &owner, &spender])
}

fn make_key_by_parts(parts: Vec<&[u8]>) -> Vec<u8> {
    parts.join(&('/' as u8))
}

fn call_transfer() -> Result<i32, String> {
    let to = hmc::hex_to_bytes(hmc::get_arg_str(0)?.as_ref());
    let amount = hmc::get_arg_str(1)?.parse::<i64>().unwrap();
    let sender = hmc::get_sender()?;

    _transfer(&sender, &to, amount)
}

fn _transfer(sender: &[u8], to: &[u8], amount: i64) -> Result<i32, String> {
    let from_balance = _balance_of(&sender)?;
    if from_balance < amount {
        return Err(format!("error: {} < {}", from_balance, amount));
    }
    hmc::write_state(&sender, &(from_balance - amount).to_be_bytes());
    let to_balance = _balance_of(&to).unwrap_or(0);
    let to_amount = (to_balance + amount).to_be_bytes();
    hmc::write_state(&to, &to_amount);
    hmc::emit_event(
        "Transfer",
        format!("from={:X?} to={:X?} amount={}", sender, to, amount).as_bytes(),
    )
    .unwrap();

    Ok(hmc::return_value(&to_amount))
}

#[no_mangle]
#[allow(non_snake_case)]
pub fn balanceOf() -> i32 {
    let sender = hmc::get_sender().unwrap();
    match _balance_of(&sender) {
        Ok(v) => hmc::return_value(&v.to_be_bytes()),
        Err(e) => {
            hmc::log(e.as_bytes());
            -1
        }
    }
}

fn _balance_of(addr: &[u8]) -> Result<i64, String> {
    match hmc::read_state(addr) {
        Ok(v) => Ok(bytes_to_i64(&v)),
        Err(e) => Err(e),
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub fn transferFrom() -> i32 {
    match _transfer_from() {
        Ok(v) => v,
        Err(e) => {
            hmc::log(e.as_bytes());
            -1
        }
    }
}

fn _transfer_from() -> Result<i32, String> {
    let sender = hmc::get_sender()?;
    let from = hmc::hex_to_bytes(hmc::get_arg_str(0)?.as_ref());
    let to = hmc::hex_to_bytes(hmc::get_arg_str(1)?.as_ref());
    let value = hmc::get_arg_str(2)?.parse::<i64>().unwrap();

    let key = make_approve_key(&from, &sender);
    let allowed = bytes_to_i64(&hmc::read_state(&key)?);

    if value > allowed {
        return Err("allowed value is insuficient".to_string());
    }

    hmc::write_state(&key, &(allowed - value).to_be_bytes());

    _transfer(&from, &to, value)
}

#[cfg(test)]
mod tests {
    extern crate hmemu;
    use super::*;

    const SENDER1_ADDR: &str = "0x1221a0726d56aEdeA9dBe2522DdAE3Dd8ED0f36c";
    const SENDER2_ADDR: &str = "0xD8eba1f372b9e0D378259F150d52C2e6C2e4109a";

    #[test]
    fn test_init() {
        let sender = hmc::hex_to_bytes(SENDER1_ADDR);
        hmemu::run_process(|| {
            hmemu::call_contract(&sender, Vec::<String>::new(), || Ok(init())).unwrap();
            {
                let v = bytes_to_i64(&hmc::read_state(&sender)?);
                assert_eq!(TOTAL_SUPPLY, v);
            }
            Ok(0)
        })
        .unwrap();
    }

    #[test]
    fn test_transfer() {
        let sender1 = hmc::hex_to_bytes(SENDER1_ADDR);
        let sender2 = hmc::hex_to_bytes(SENDER2_ADDR);

        hmemu::run_process(|| {
            hmemu::call_contract(&sender1, Vec::<String>::new(), || Ok(init())).unwrap();
            hmemu::call_contract(&sender1, vec![SENDER2_ADDR, "100"], || Ok(call_transfer()?))
                .unwrap();

            {
                let b1 = _balance_of(&sender1)?;
                assert_eq!(TOTAL_SUPPLY - 100, b1);

                let b2 = _balance_of(&sender2)?;
                assert_eq!(100, b2);
            }

            hmemu::call_contract(&sender2, vec![SENDER1_ADDR, "100"], || Ok(call_transfer()?))
                .unwrap();

            {
                let b1 = _balance_of(&sender1)?;
                assert_eq!(TOTAL_SUPPLY, b1);

                let b2 = _balance_of(&sender2)?;
                assert_eq!(0, b2);
            }

            Ok(0)
        })
        .unwrap();
    }

    #[test]
    fn test_approve() {
        let sender1 = hmc::hex_to_bytes(SENDER1_ADDR);

        hmemu::run_process(|| {
            hmemu::call_contract(&sender1, Vec::<String>::new(), || Ok(init())).unwrap();

            hmemu::call_contract(&sender1, vec![SENDER2_ADDR, "100"], || {
                assert_eq!(true, _approve()?);
                Ok(0)
            })
            .unwrap();

            hmemu::call_contract(&sender1, vec![SENDER1_ADDR, SENDER2_ADDR], || {
                assert_eq!(100, _allowance()?);
                Ok(0)
            })
            .unwrap();

            Ok(0)
        })
        .unwrap();
    }

    #[test]
    fn test_transfer_from() {
        let sender1 = hmc::hex_to_bytes(SENDER1_ADDR);
        let sender2 = hmc::hex_to_bytes(SENDER2_ADDR);

        hmemu::run_process(|| {
            hmemu::call_contract(&sender1, Vec::<String>::new(), || Ok(init())).unwrap();

            hmemu::call_contract(&sender1, vec![SENDER2_ADDR, "100"], || {
                assert_eq!(true, _approve()?);
                Ok(0)
            })
            .unwrap();

            hmemu::call_contract(&sender2, vec![SENDER1_ADDR, SENDER2_ADDR, "100"], || {
                Ok(_transfer_from()?)
            })
            .unwrap();

            {
                let b1 = _balance_of(&sender1)?;
                assert_eq!(TOTAL_SUPPLY - 100, b1);

                let b2 = _balance_of(&sender2)?;
                assert_eq!(100, b2);
            }

            Ok(0)
        })
        .unwrap();
    }

}
