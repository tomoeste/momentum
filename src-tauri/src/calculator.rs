use crate::models::DebtAccount;

#[derive(Debug, Clone)]
pub struct Scenario {
    pub monthly_cut: f64,
    pub months_saved: f64,
    pub interest_saved: f64,
    pub new_payoff_months: f64,
}

pub fn calculate_payoff_months(balance: f64, apr: f64, monthly_payment: f64) -> f64 {
    if apr == 0.0 || monthly_payment == 0.0 {
        return if balance > 0.0 { balance / monthly_payment.max(1.0) } else { 0.0 };
    }

    let monthly_rate = apr / 12.0;
    if monthly_payment <= balance * monthly_rate {
        return f64::INFINITY; // Payment doesn't cover interest
    }

    // n = -ln(1 - r*P/PMT) / ln(1+r)
    let numerator = 1.0 - (monthly_rate * balance / monthly_payment);
    if numerator <= 0.0 {
        return f64::INFINITY;
    }

    -numerator.ln() / (1.0 + monthly_rate).ln()
}

pub fn calculate_monthly_interest(balance: f64, apr: f64) -> f64 {
    balance * (apr / 12.0)
}

pub fn calculate_total_interest(balance: f64, apr: f64, monthly_payment: f64) -> f64 {
    let months = calculate_payoff_months(balance, apr, monthly_payment);
    if months.is_infinite() {
        return f64::INFINITY;
    }

    let mut remaining = balance;
    let mut total_interest = 0.0;
    let monthly_rate = apr / 12.0;

    for _ in 0..(months.ceil() as i32) {
        let interest_charge = remaining * monthly_rate;
        total_interest += interest_charge;
        remaining -= monthly_payment - interest_charge;
        if remaining <= 0.0 {
            break;
        }
    }

    total_interest
}

pub fn calculate_scenarios(
    debt_accounts: &[DebtAccount],
    default_cuts: &[f64],
) -> Vec<Scenario> {
    let mut scenarios = Vec::new();

    for &cut in default_cuts {
        let mut total_months_current = 0.0;
        let mut total_months_new = 0.0;
        let mut total_interest_current = 0.0;
        let mut total_interest_new = 0.0;

        for account in debt_accounts {
            if account.interest_rate == 0.0 {
                continue;
            }

            let current_payment = account.minimum_payment.unwrap_or(account.current_balance * 0.02);
            let new_payment = current_payment + cut;

            let months_current = calculate_payoff_months(account.current_balance, account.interest_rate, current_payment);
            let months_new = calculate_payoff_months(account.current_balance, account.interest_rate, new_payment);

            let interest_current = calculate_total_interest(account.current_balance, account.interest_rate, current_payment);
            let interest_new = calculate_total_interest(account.current_balance, account.interest_rate, new_payment);

            if !months_current.is_infinite() {
                total_months_current += months_current;
            }
            if !months_new.is_infinite() {
                total_months_new += months_new;
            }
            if !interest_current.is_infinite() {
                total_interest_current += interest_current;
            }
            if !interest_new.is_infinite() {
                total_interest_new += interest_new;
            }
        }

        scenarios.push(Scenario {
            monthly_cut: cut,
            months_saved: (total_months_current - total_months_new).max(0.0),
            interest_saved: (total_interest_current - total_interest_new).max(0.0),
            new_payoff_months: total_months_new,
        });
    }

    scenarios
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payoff_calculation() {
        // $5000 balance, 22% APR, $150/month payment
        let months = calculate_payoff_months(5000.0, 0.22, 150.0);
        assert!((months - 35.0).abs() < 2.0); // Should be ~35 months
    }

    #[test]
    fn test_monthly_interest() {
        // $5000 balance, 22% APR
        let interest = calculate_monthly_interest(5000.0, 0.22);
        assert!((interest - 91.67).abs() < 0.1); // Should be ~$91.67/month
    }
}
