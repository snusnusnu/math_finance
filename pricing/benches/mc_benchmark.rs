// https://florianovictor.medium.com/rust-adventures-criterion-50754cb3295
// https://bheisler.github.io/criterion.rs/book/getting_started.html

extern crate pricing;
use pricing::simulation::distributions::MultivariateNormalDistribution;
use pricing::simulation::monte_carlo::{MonteCarloPathSimulator, PathEvaluator};
use pricing::simulation::sde::gbm::GeometricBrownianMotion;
use pricing::simulation::sde::multivariate_gbm::MultivariateGeometricBrownianMotion;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ndarray::{arr1, arr2, Array2};
use rand_distr::StandardNormal;

// criterion_group!{
//     name = benches;
//     config = Criterion::default().measurement_time(std::time::Duration::from_secs(100));
//     target = criterion_stock_price_simulation;
// }
criterion_group!(
    benches,
    criterion_stock_price_simulation,
    criterion_basket_stock_price_simulation,
    criterion_multivariate_normal_distr
);
criterion_main!(benches);

pub fn criterion_stock_price_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Stock price Monte Carlo simulation");

    group.bench_function(
        "apply a path function on the stored paths with Hc128 RNG",
        |b| {
            b.iter(|| {
                simulate_paths_with_path_generator::<rand_hc::Hc128Rng>(black_box((30_000, 200)))
            })
        },
    );

    group.bench_function(
        "apply a path function on the stored paths with Isaac64 RNG",
        |b| {
            b.iter(|| {
                simulate_paths_with_path_generator::<rand_isaac::Isaac64Rng>(black_box((
                    30_000, 200,
                )))
            })
        },
    );

    group.bench_function(
        "apply a path function on the stored paths with ChaCha RNG",
        |b| {
            b.iter(|| {
                simulate_paths_with_path_generator::<rand_chacha::ChaCha12Rng>(black_box((
                    30_000, 200,
                )))
            })
        },
    );

    group.bench_function(
        "apply a path function (in place) on the stored paths",
        |b| b.iter(|| simulate_paths_with_path_generator_in_place(black_box((30_000, 200)))),
    );

    group.bench_function("direct gbm sampler", |b| {
        b.iter(|| simulate_paths_with_path_generator_gbm(black_box((30_000, 200))))
    });

    group.finish()
}

fn simulate_paths_with_path_generator<SeedRng>((nr_paths, nr_steps): (usize, usize))
where
    SeedRng: rand::SeedableRng + rand::RngCore,
{
    let vola = 50.0 / 365.0;
    let drift = 0.01;
    let dt = 0.1;
    let s0 = 300.0;

    let stock_gbm = GeometricBrownianMotion::new(s0, drift, vola, dt);
    let mc_simulator: MonteCarloPathSimulator<_, SeedRng, Vec<f64>> =
        MonteCarloPathSimulator::new(StandardNormal, Some(42));

    let paths = mc_simulator.simulate_paths_with(nr_paths, nr_steps, |random_normals| {
        stock_gbm.generate_path(s0, random_normals)
    });

    let path_eval = PathEvaluator::new(&paths);
    let avg_price = path_eval.evaluate_average(|path| path.last().cloned());
    assert!(avg_price.is_some());
}

fn simulate_paths_with_path_generator_in_place((nr_paths, nr_steps): (usize, usize)) {
    let vola = 50.0 / 365.0;
    let drift = 0.01;
    let dt = 0.1;
    let s0 = 300.0;

    let stock_gbm = GeometricBrownianMotion::new(s0, drift, vola, dt);

    let mc_simulator: MonteCarloPathSimulator<_, rand_hc::Hc128Rng, Vec<f64>> =
        MonteCarloPathSimulator::new(StandardNormal, Some(42));

    let paths = mc_simulator.simulate_paths_apply_in_place(nr_paths, nr_steps, |random_normals| {
        stock_gbm.generate_in_place(random_normals)
    });

    let path_eval = PathEvaluator::new(&paths);
    let avg_price = path_eval.evaluate_average(|path| path.last().cloned());
    assert!(avg_price.is_some());
}

fn simulate_paths_with_path_generator_gbm((nr_paths, nr_steps): (usize, usize)) {
    let vola = 50.0 / 365.0;
    let drift = 0.01;
    let dt = 0.1;
    let s0 = 300.0;

    let stock_gbm = GeometricBrownianMotion::new(s0, drift, vola, dt);
    let mc_simulator: MonteCarloPathSimulator<_, rand_hc::Hc128Rng, Vec<f64>> =
        MonteCarloPathSimulator::new(stock_gbm, Some(42));
    let paths = mc_simulator.simulate_paths(nr_paths, nr_steps);

    let path_eval = PathEvaluator::new(&paths);
    let avg_price = path_eval.evaluate_average(|path| path.last().cloned());
    assert!(avg_price.is_some());
}

pub fn criterion_basket_stock_price_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Basket stock price Monte Carlo simulation");

    group.bench_function("direct multivariate gbm sampler", |b| {
        b.iter(|| basket_stock_price_simulation(black_box((5_000, 200))))
    });

    group.finish()
}

fn basket_stock_price_simulation((nr_paths, nr_steps): (usize, usize)) {
    let initial_values = arr1(&[110.0, 120.0, 130.0]);
    let drifts = arr1(&[0.1, 0.2, 0.3]);
    let cholesky_factor = arr2(&[[1.0, 0.05, 0.1], [0.0, 0.6, 0.7], [0.0, 0.0, 0.8]]);
    let dt = 1.0;

    let mv_gbm =
        MultivariateGeometricBrownianMotion::new(initial_values, drifts, cholesky_factor, dt);

    let mc_simulator: MonteCarloPathSimulator<_, rand_hc::Hc128Rng, Array2<f64>> =
        MonteCarloPathSimulator::new(mv_gbm, Some(42));
    let paths = mc_simulator.simulate_paths(nr_paths, nr_steps);

    assert_eq!(paths.len(), nr_paths);

    let path_eval = PathEvaluator::new(&paths);

    let avg_price = path_eval.evaluate_average(|path| {
        path.axis_iter(ndarray::Axis(1)).last().map(|a| a.sum())
        // path.last()
        //     .cloned()
        //     .map(|p| p.iter().fold(0.0, |acc, x| acc + x))
    });
    assert!(avg_price.is_some());
}

pub fn criterion_multivariate_normal_distr(c: &mut Criterion) {
    let mut group =
        c.benchmark_group("Monte Carlo simulation for Multivariate Normal Distribution paths");

    group.bench_function("modelled with array2 and Hc128 RNG", |b| {
        b.iter(|| {
            multivariate_normal_distr_array2::<rand_hc::Hc128Rng>(black_box((5_000, 300, 42)))
        })
    });

    group.bench_function("modelled with vec<array1> and Hc128 RNG", |b| {
        b.iter(|| {
            multivariate_normal_distr_vec_of_array1::<rand_hc::Hc128Rng>(black_box((
                5_000, 300, 42,
            )))
        })
    });

    group.bench_function("modelled with array2 and Isaac64 RNG", |b| {
        b.iter(|| {
            multivariate_normal_distr_array2::<rand_isaac::Isaac64Rng>(black_box((5_000, 300, 42)))
        })
    });

    group.bench_function("modelled with array2 and ChaCha RNG", |b| {
        b.iter(|| {
            multivariate_normal_distr_array2::<rand_chacha::ChaCha12Rng>(black_box((
                5_000, 300, 42,
            )))
        })
    });

    group.bench_function("modelled with vec<array1> and Isaac64 RNG", |b| {
        b.iter(|| {
            multivariate_normal_distr_vec_of_array1::<rand_isaac::Isaac64Rng>(black_box((
                5_000, 300, 42,
            )))
        })
    });

    group.finish()
}

fn multivariate_normal_distr_array2<SeedRng>((nr_paths, nr_steps, seed): (usize, usize, u64))
where
    SeedRng: rand::SeedableRng + rand::RngCore,
{
    let mu = arr1(&[0.1, 0.2, 0.3]);
    let cholesky_factor = arr2(&[[1.0, 0.5, 0.1], [0.0, 0.6, 0.7], [0.0, 0.0, 0.8]]);
    let mv_normal = MultivariateNormalDistribution::new(mu, cholesky_factor);

    let mc_simulator: MonteCarloPathSimulator<_, SeedRng, Array2<f64>> =
        MonteCarloPathSimulator::new(mv_normal, Some(seed));
    let paths = mc_simulator.simulate_paths(nr_paths, nr_steps);
    assert_eq!(paths.len(), nr_paths);
}

fn multivariate_normal_distr_vec_of_array1<SeedRng>((nr_paths, nr_steps, seed): (usize, usize, u64))
where
    SeedRng: rand::SeedableRng + rand::RngCore,
{
    let mu = arr1(&[0.1, 0.2, 0.3]);
    let cholesky_factor = arr2(&[[1.0, 0.5, 0.1], [0.0, 0.6, 0.7], [0.0, 0.0, 0.8]]);
    let mv_normal = MultivariateNormalDistribution::new(mu, cholesky_factor);

    let mc_simulator: MonteCarloPathSimulator<_, SeedRng, Vec<_>> =
        MonteCarloPathSimulator::new(mv_normal, Some(seed));
    let paths = mc_simulator.simulate_paths(nr_paths, nr_steps);
    assert_eq!(paths.len(), nr_paths);
}

// fn multivariate_normal_distr_slice_path_allocated((nr_paths, nr_steps, seed): (usize, usize, u64)) {
//     let mu = arr1(&[0.1, 0.2, 0.3]);
//     let cholesky_factor = arr2(&[[1.0, 0.5, 0.1], [0.0, 0.6, 0.7], [0.0, 0.0, 0.8]]);
//     let mv_normal = MultivariateNormalDistribution::new(mu, cholesky_factor);

//     let mc_simulator: MonteCarloPathSimulator<SlicePath> =
//         MonteCarloPathSimulator::new(nr_paths, nr_steps);
//     let paths = mc_simulator.simulate_paths(seed, mv_normal);
//     let array_paths = paths.into_iter().map(|path| path.iter().collect());
//     assert_eq!(paths.len(), nr_paths);
// }
