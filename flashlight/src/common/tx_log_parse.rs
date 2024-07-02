use regex::Regex;

pub fn is_ix_log(log: &str) -> bool {
    let re = Regex::new(r"Program log: Instruction:").unwrap();

    re.is_match(log)
}

pub fn is_end_ix_log(program_id: &str, log: &str) -> bool {
    let regex_pattern = format!(
        r"Program {} consumed ([0-9]+) of ([0-9]+) compute units",
        program_id
    );
    let re = Regex::new(&regex_pattern).unwrap();

    re.is_match(log)
}

pub fn is_fill_ix_log(log: &str) -> bool {
    let re = Regex::new(r"Program log: Instruction: Fill(.*)Order").unwrap();

    re.is_match(log)
}

pub fn is_arb_ix_log(log: &str) -> bool {
    let re = Regex::new(r"Program log: Instruction: ArbPerp").unwrap();

    re.is_match(log)
}

pub fn is_order_does_not_exist_log(log: &str) -> Option<u32> {
    let re = Regex::new(r".*Order does not exist ([0-9]+)").unwrap();

    if let Some(captures) = re.captures(log) {
        if let Some(matched) = captures.get(1) {
            return matched.as_str().parse().ok();
        }
    }

    None
}

pub fn is_maker_order_does_not_exist_log(log: &str) -> Option<u32> {
    let re = Regex::new(r".*Maker has no order id ([0-9]+)").unwrap();

    if let Some(captures) = re.captures(log) {
        if let Some(matched) = captures.get(1) {
            return matched.as_str().parse().ok();
        }
    }

    None
}

pub fn is_maker_breached_maintainance_margin_log(log: &str) -> Option<String> {
    let re = Regex::new(
        r".*maker \(([1-9A-HJ-NP-Za-km-z]+)\) breached (maintenance|fill) requirements.*$",
    )
    .unwrap();

    if let Some(captures) = re.captures(log) {
        if let Some(matched) = captures.get(1) {
            return matched.as_str().parse().ok();
        }
    }

    None
}

pub fn is_taker_breached_maintainance_margin_log(log: &str) -> bool {
    let re = Regex::new(r".*taker breached (maintenance|fill) requirements.*").unwrap();

    re.is_match(log)
}

pub fn is_err_filling_log(log: &str) -> (Option<u32>, Option<&str>) {
    let re = Regex::new(r".*Err filling order id ([0-9]+) for user ([a-zA-Z0-9]+)").unwrap();

    if let Some(captures) = re.captures(log) {
        match (captures.get(1), captures.get(2)) {
            (Some(order_id), Some(user)) => {
                let order_id = order_id.as_str().parse().ok();

                return (order_id, Some(user.as_str()));
            }
            _ => return (None, None),
        }
    }

    (None, None)
}

pub fn is_err_arb(log: &str) -> bool {
    let re = Regex::new(r".*NoArbOpportunity*").unwrap();

    re.is_match(log)
}

pub fn is_err_arb_no_bid(log: &str) -> bool {
    let re = Regex::new(r".*NoBestBid*").unwrap();

    re.is_match(log)
}

pub fn is_err_arb_no_ask(log: &str) -> bool {
    let re = Regex::new(r".*NoBestAsk*").unwrap();

    re.is_match(log)
}

pub fn is_err_stale_oracle(log: &str) -> bool {
    let re = Regex::new(r".*Invalid Oracle: Stale.*").unwrap();

    re.is_match(log)
}
