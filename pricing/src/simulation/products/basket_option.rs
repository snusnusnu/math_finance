use std::marker::PhantomData;

use ndarray::prelude::*;
use ndarray::Array2;

use crate::simulation::monte_carlo::MonteCarloPathSimulator;
use crate::simulation::sde::multivariate_gbm::MultivariateGeometricBrownianMotion;
use crate::simulation::PathEvaluator;

// https://backtick.se/blog/options-mc-2/
// https://jbhender.github.io/Stats506/F18/GP/Group21.html
/// Indices of cholesky matrix must be aligned with the indices in weights, asset_proces, rf_rates
pub struct MonteCarloEuropeanBasketOption<SeedRng>
where
    SeedRng: rand::SeedableRng + rand::RngCore,
{
    weights: Array1<f64>,
    asset_prices: Array1<f64>,
    rf_rates: Array1<f64>,
    cholesky_factor: Array2<f64>,

    /// the strike or exercise price of the basket
    strike: f64,
    /// (T - t) in years, where T is the time of the option's expiration and t is the current time
    time_to_expiration: f64,

    seed_nr: u64,
    nr_paths: usize,
    nr_steps: usize,
    _phantom_rng: PhantomData<SeedRng>,
}

impl<SeedRng> MonteCarloEuropeanBasketOption<SeedRng>
where
    SeedRng: rand::SeedableRng + rand::RngCore,
{
    pub fn new(
        // underlying_map: HashMap<Underlying, usize>,
        weights: Array1<f64>,
        asset_prices: Array1<f64>,
        rf_rates: Array1<f64>,
        cholesky_factor: Array2<f64>,
        strike: f64,
        time_to_expiration: f64,

        nr_paths: usize,
        nr_steps: usize,
        seed_nr: u64,
    ) -> Self {
        let weight_sum = weights.iter().fold(0.0, |acc, c| acc + c);
        assert_eq!(weight_sum, 1.0);
        Self {
            time_to_expiration,
            strike,
            cholesky_factor,
            rf_rates,
            asset_prices,
            weights,
            nr_paths,
            nr_steps,
            seed_nr,
            _phantom_rng: PhantomData::<SeedRng>,
        }
    }

    pub fn dt(&self) -> f64 {
        self.time_to_expiration / self.nr_steps as f64
    }

    fn sample_payoffs(&self, pay_off: impl Fn(&Array2<f64>) -> Option<f64>) -> Option<f64> {
        let gbm: MultivariateGeometricBrownianMotion = self.into();
        let mc_simulator: MonteCarloPathSimulator<_, SeedRng, _> =
            MonteCarloPathSimulator::new(gbm, Some(self.seed_nr));
        let paths = mc_simulator.simulate_paths(self.nr_paths, self.nr_steps);
        let path_evaluator = PathEvaluator::new(&paths);
        path_evaluator.evaluate_average(pay_off)
    }

    fn call_payoff(
        &self,
        strike: f64,
        weights: &Array1<f64>,
        disc_factor: f64,
        path: &Array2<f64>,
    ) -> Option<f64> {
        path.axis_iter(Axis(0))
            .last()
            .map(|p| (p.dot(weights) - strike).max(0.0) * disc_factor)
    }

    fn put_payoff(
        &self,
        strike: f64,
        weights: &Array1<f64>,
        disc_factor: f64,
        path: &Array2<f64>,
    ) -> Option<f64> {
        path.axis_iter(Axis(0))
            .last()
            .map(|p| (strike - p.dot(weights)).max(0.0) * disc_factor)
    }

    fn discount_factor(&self, t: f64) -> f64 {
        (-t * self.rf_rates.dot(&self.weights)).exp()
    }

    /// The price (theoretical value) of the standard European call option (optimized version).
    pub fn call(&self) -> Option<f64> {
        let disc_factor = self.discount_factor(self.time_to_expiration);
        self.sample_payoffs(|path| self.call_payoff(self.strike, &self.weights, disc_factor, path))
    }

    /// The price (theoretical value) of the standard European put option (optimized version).
    pub fn put(&self) -> Option<f64> {
        let disc_factor = self.discount_factor(self.time_to_expiration);
        self.sample_payoffs(|path| self.put_payoff(self.strike, &self.weights, disc_factor, path))
    }
}

impl<R> From<&MonteCarloEuropeanBasketOption<R>> for MultivariateGeometricBrownianMotion
where
    R: rand::SeedableRng + rand::RngCore,
{
    fn from(mceo: &MonteCarloEuropeanBasketOption<R>) -> Self {
        MultivariateGeometricBrownianMotion::new(
            mceo.asset_prices.to_owned(),
            mceo.rf_rates.to_owned(),
            mceo.cholesky_factor.to_owned(),
            mceo.dt(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn european_basket_call() {
        let asset_prices = arr1(&[40.0, 60.0, 100.0]);
        let rfrs = arr1(&[0.01, 0.02, -0.01]);
        let cholesky_factor = arr2(&[[1.0, 0.05, 0.1], [0.0, 0.06, 0.17], [0.0, 0.0, 0.8]]);
        let weights = arr1(&[0.25, 0.25, 0.5]);

        let mc_option: MonteCarloEuropeanBasketOption<rand_hc::Hc128Rng> =
            MonteCarloEuropeanBasketOption::new(
                weights,
                asset_prices,
                rfrs,
                cholesky_factor,
                230.0,
                2.0,
                10_000,
                300,
                42,
            );
        let call_price = mc_option.call().unwrap();
        dbg!(call_price);
        // TODO: fix unit test
        // assert_eq!(call_price, 5.59601793502129);
        // assert_approx_eq!(call_price, 29.47, TOLERANCE);
    }

    #[test]
    #[ignore]
    fn european_basket_call_iid() {
        let asset_prices = arr1(&[102.0, 102.0]);
        let rfrs = arr1(&[0.02, 0.02]);
        let weights = arr1(&[0.5, 0.5]);

        // no correlation between assets
        let cholesky_factor = arr2(&[[0.2, 0.0], [0.0, 0.2]]);

        let mc_option: MonteCarloEuropeanBasketOption<rand_hc::Hc128Rng> =
            MonteCarloEuropeanBasketOption::new(
                weights,
                asset_prices,
                rfrs,
                cholesky_factor,
                100.0,
                0.5,
                10_000,
                100,
                42,
            );
        let call_price = mc_option.call().unwrap();
        dbg!(&call_price);
        // TODO: fix unit test
        // assert_approx_eq!(call_price, 7.290738, TOLERANCE);
    }

    #[test]
    #[ignore]
    fn european_basket_put() {
        let asset_prices = arr1(&[50.0, 60.0, 100.0]);
        let rfrs = arr1(&[0.01, 0.02, -0.01]);
        let cholesky_factor = arr2(&[[1.0, 0.05, 0.1], [0.0, 0.06, 0.17], [0.0, 0.0, 0.8]]);
        let weights = arr1(&[0.25, 0.25, 0.5]);

        let mc_option: MonteCarloEuropeanBasketOption<rand_hc::Hc128Rng> =
            MonteCarloEuropeanBasketOption::new(
                weights,
                asset_prices,
                rfrs,
                cholesky_factor,
                180.0,
                2.0,
                10_000,
                300,
                42,
            );
        let call_price = mc_option.put().unwrap();
        assert_eq!(call_price, 8.96589328828396);
        // assert_approx_eq!(call_price, 29.47, TOLERANCE);
    }

    /// https://predictivehacks.com/pricing-of-european-options-with-monte-carlo/
    /// Example from https://ch.mathworks.com/help/fininst/basketsensbyls.html
    #[test]
    #[ignore]
    fn european_basket_put_reference() {
        let _corr = arr2(&[[1.0, 0.15], [0.15, 1.0]]);

        // todo: check cholesky of corr rather than cov?
        let cholesky_factor = arr2(&[[1.0, 0.15], [0.0, 1.0 - 0.15_f64.powi(2)]]);

        let asset_prices = arr1(&[90.0, 75.0]);
        let rfrs = arr1(&[0.05, 0.05]);
        let weights = arr1(&[0.5, 0.5]);

        let mc_option: MonteCarloEuropeanBasketOption<rand_hc::Hc128Rng> =
            MonteCarloEuropeanBasketOption::new(
                weights,
                asset_prices,
                rfrs,
                cholesky_factor,
                80.0,
                1.0,
                10_000,
                300,
                42,
            );

        // PriceSens = 0.9822
        // Delta = -0.0995

        let call_price = mc_option.put().unwrap();
        assert_eq!(call_price, 0.9822);
        // assert_approx_eq!(call_price, 29.47, TOLERANCE);
    }
}
