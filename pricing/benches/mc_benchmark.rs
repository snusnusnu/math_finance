// https://florianovictor.medium.com/rust-adventures-criterion-50754cb3295
// https://bheisler.github.io/criterion.rs/book/getting_started.html

extern crate pricing;
use pricing::simulation::monte_carlo::{MonteCarloPathSimulator, PathEvaluator};
use pricing::simulation::GeometricBrownianMotion;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// criterion_group!{
//     name = benches;
//     config = Criterion::default().measurement_time(std::time::Duration::from_secs(100));
//     target = criterion_stock_price_simulation;
// }
criterion_group!(benches, criterion_stock_price_simulation);
criterion_main!(benches);

pub fn criterion_stock_price_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Stock price Monte Carlo simulation");

    group.bench_function(
        "apply a path function to each path directly (do not store paths)",
        |b| b.iter(|| simulate_paths_with_path_fn(black_box((10_000, 500)))),
    );
    group.bench_function("apply a path function on the stored paths", |b| {
        b.iter(|| simulate_paths_with_path_generator(black_box((10_000, 500))))
    });

    group.finish()
}

fn simulate_paths_with_path_fn((nr_paths, nr_steps): (usize, usize)) {
    let vola = 50.0 / 365.0;
    let drift = 0.01;
    let dt = 0.1;
    let s0 = 300.0;

    let stock_gbm = GeometricBrownianMotion::new(drift, vola, dt);
    let mc_simulator = MonteCarloPathSimulator::new(nr_paths, nr_steps);
    let paths = mc_simulator.simulate_paths_with(stock_gbm, s0, |path| path.last().cloned());
    let sum = paths
        .iter()
        .fold(0.0, |acc, last_p| acc + last_p.unwrap_or(0.0));
    assert!(sum > 0.0);
}

fn simulate_paths_with_path_generator((nr_paths, nr_steps): (usize, usize)) {
    let vola = 50.0 / 365.0;
    let drift = 0.01;
    let dt = 0.1;
    let s0 = 300.0;

    let stock_gbm = GeometricBrownianMotion::new(drift, vola, dt);
    let mc_simulator = MonteCarloPathSimulator::new(nr_paths, nr_steps);
    let paths = mc_simulator.simulate_paths(stock_gbm, s0);
    let path_eval = PathEvaluator::new(&paths);
    let avg_price = path_eval.evaluate_average(|path| path.last().cloned());
    assert!(avg_price.is_some());
}
