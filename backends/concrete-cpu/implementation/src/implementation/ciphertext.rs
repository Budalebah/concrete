use super::types::*;
use super::Split;

impl<'a> GlweCiphertext<&'a [u64]> {
    pub fn sample_extract(self, lwe: LweCiphertext<&mut [u64]>, n_th: usize) {
        debug_assert!(self.glwe_params.lwe_dimension() <= lwe.lwe_dimension);

        let polynomial_size = self.glwe_params.polynomial_size;

        // We retrieve the bodies and masks of the two ciphertexts.
        let (lwe_body, lwe_full_mask) = lwe.into_data().split_last_mut().unwrap();

        let (lwe_common_mask, additional_mask) =
            lwe_full_mask.split_at_mut(self.glwe_params.lwe_dimension());

        additional_mask.fill(0);

        let glwe_index = self.glwe_params.dimension * polynomial_size;
        let (glwe_mask, glwe_body) = self.into_data().split_at(glwe_index);

        // We copy the body
        *lwe_body = glwe_body[n_th];

        // We copy the mask (each polynomial is in the wrong order)
        lwe_common_mask.copy_from_slice(glwe_mask);

        // We compute the number of elements which must be
        // turned into their opposite
        let opposite_count = polynomial_size - n_th - 1;

        // We loop through the polynomials (as mut tensors)
        for lwe_mask_poly in lwe_common_mask.into_chunks(polynomial_size) {
            // We reverse the polynomial
            lwe_mask_poly.reverse();
            // We compute the opposite of the proper coefficients
            for x in lwe_mask_poly[0..opposite_count].iter_mut() {
                *x = x.wrapping_neg()
            }
            // We rotate the polynomial properly
            lwe_mask_poly.rotate_left(opposite_count);
        }
    }
}
