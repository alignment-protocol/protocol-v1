
/// Calculates the square root of a number for quadratic voting power
pub fn calculate_quadratic_voting_power(amount: u64) -> u64 {
    (amount as f64).sqrt() as u64
}