
use rand::prelude::IteratorRandom;
use rand::{self, prelude::ThreadRng};
use rand_distr::{DistIter, Distribution, Normal};



// use nalgebra::point;
// use nalgebra::ma;
use ndarray::prelude::*;
// use ndarray_linalg::cholesky::*;
use ndarray::arr1;

pub struct MultivariateGeometricBrownianMotion {
    initial_values: Array1<f64>,
    /// drift term
    drifts: Array1<f64>,
    /// volatility
    cholesky_factor: Array2<f64>,
    /// change in time
    dt: f64,
}

impl MultivariateGeometricBrownianMotion {
    pub fn new(
        initial_values: Array1<f64>,
        drifts: Array1<f64>,
        cholesky_factor: Array2<f64>,
        dt: f64,
    ) -> Self {
        let iv_shape = initial_values.shape();
        let drifts_shape = drifts.shape();
        let matrix_shape = cholesky_factor.shape();

        assert_eq!(iv_shape, drifts_shape);
        assert_eq!(matrix_shape, &[drifts_shape[0], drifts_shape[0]]);

        // TODO: add a check that cholesky_factor is triangular; oR provide only a constructor using the correlation matrix

        Self {
            initial_values,
            drifts,
            cholesky_factor,
            dt,
        }
    }

    /// See https://en.wikipedia.org/wiki/Geometric_Brownian_motion
    pub(crate) fn sample(&self, st: &Array1<f64>, z: &Array1<f64>) -> Array1<f64> {
        let d_st_s0: Array1<f64> =
            self.dt * &self.drifts + self.dt.sqrt() * self.cholesky_factor.dot(z);

        let d_st: Array1<f64> = d_st_s0
            .iter()
            .zip(st.iter())
            .map(|(dst, st)| dst * st)
            .collect();
        st + d_st // d_St = S_t+1 - St
    }

    pub fn sample_path(
        &self,
        nr_steps: usize,
        normal_distr: DistIter<Normal<f64>, &mut ThreadRng, f64>,
    ) -> Vec<Vec<f64>> {
        let mut path = Vec::with_capacity(nr_steps + 1);

        path.push(self.initial_values.to_vec());

        let dim = self.initial_values.shape()[0];
        let mut rng = rand::thread_rng();

        // create the random normal numbers for the whole path and all dimensions
        let path_zs = normal_distr.choose_multiple(&mut rng, dim * nr_steps);

        for (idx, _) in path_zs.iter().enumerate().step_by(dim) {
            let zs_slice = arr1(&path_zs[idx..idx + dim]);
            let curr_p = arr1(&path.last().unwrap());
            let sample = self.sample(&curr_p, &zs_slice);
            path.push(sample.to_vec());
        }

        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::{arr1, arr2};

    #[test]
    fn sample() {
        let initial_values = arr1(&[1.0, 2.0, 3.0]);
        let drifts = arr1(&[0.1, 0.2, 0.3]);
        let cholesky_factor = arr2(&[[1.0, 0.5, 0.1], [0.0, 0.6, 0.7], [0.0, 0.0, 0.8]]);
        let dt = 4.0;

        let mv_gbm =
            MultivariateGeometricBrownianMotion::new(initial_values, drifts, cholesky_factor, dt);

        let rand_normals = arr1(&[0.1, -0.1, 0.05]);
        let sample = mv_gbm.sample(&mv_gbm.initial_values, &rand_normals);
        assert_eq!(sample, arr1(&[1.51, 3.5, 6.84]));
    }
}
