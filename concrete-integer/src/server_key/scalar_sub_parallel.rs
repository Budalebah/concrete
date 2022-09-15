use crate::ciphertext::Ciphertext;
use crate::ServerKey;

impl ServerKey {
    /// Computes homomorphically a subtraction of a ciphertext by a scalar.
    ///
    /// # Example
    ///
    /// ```rust
    /// use concrete_integer::gen_keys;
    /// use concrete_shortint::parameters::PARAM_MESSAGE_2_CARRY_2;
    ///
    /// // We have 4 * 2 = 8 bits of message
    /// let size = 4;
    /// let (cks, sks) = gen_keys(&PARAM_MESSAGE_2_CARRY_2, size);
    ///
    /// let msg = 165;
    /// let scalar = 112;
    ///
    /// let mut ct = cks.encrypt(msg);
    ///
    /// // Compute homomorphically an addition:
    /// let ct_res = sks.smart_scalar_sub_parallelized(&mut ct, scalar);
    ///
    /// // Decrypt:
    /// let dec = cks.decrypt(&ct_res);
    /// assert_eq!(msg - scalar, dec);
    /// ```
    pub fn smart_scalar_sub_parallelized(&self, ct: &mut Ciphertext, scalar: u64) -> Ciphertext {
        if !self.is_scalar_sub_possible(ct, scalar) {
            self.full_propagate_parallelized(ct);
        }
        self.unchecked_scalar_sub(ct, scalar)
    }

    pub fn smart_scalar_sub_assign_parallelized(&self, ct: &mut Ciphertext, scalar: u64) {
        if !self.is_scalar_sub_possible(ct, scalar) {
            self.full_propagate_parallelized(ct);
        }
        self.unchecked_scalar_sub_assign(ct, scalar);
    }
}
