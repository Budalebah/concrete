use crate::ciphertext::Degree;
use crate::engine::{EngineResult, ShortintEngine};
use crate::{Ciphertext, ServerKey};
use concrete_core::prelude::*;

impl ShortintEngine {
    pub(crate) fn unchecked_scalar_mul(
        &mut self,
        ct: &Ciphertext,
        scalar: u8,
    ) -> EngineResult<Ciphertext> {
        let mut ct_result = ct.clone();
        self.unchecked_scalar_mul_assign(&mut ct_result, scalar)?;

        Ok(ct_result)
    }

    pub(crate) fn unchecked_scalar_mul_assign(
        &mut self,
        ct: &mut Ciphertext,
        scalar: u8,
    ) -> EngineResult<()> {
        let scalar = u64::from(scalar);
        let cleartext_scalar = self.engine.create_cleartext_from(&scalar).unwrap();
        self.engine
            .fuse_mul_lwe_ciphertext_cleartext(&mut ct.ct, &cleartext_scalar)?;

        ct.degree = Degree(ct.degree.0 * scalar as usize);
        Ok(())
    }

    pub(crate) fn smart_scalar_mul(
        &mut self,
        server_key: &ServerKey,
        ctxt: &mut Ciphertext,
        scalar: u8,
    ) -> EngineResult<Ciphertext> {
        let mut ct_result = ctxt.clone();
        self.smart_scalar_mul_assign(server_key, &mut ct_result, scalar)?;

        Ok(ct_result)
    }

    pub(crate) fn smart_scalar_mul_assign(
        &mut self,
        server_key: &ServerKey,
        ctxt: &mut Ciphertext,
        scalar: u8,
    ) -> EngineResult<()> {
        let modulus = server_key.message_modulus.0 as u64;
        // Direct scalar computation is possible
        if server_key.is_scalar_mul_possible(ctxt, scalar) {
            self.unchecked_scalar_mul_assign(ctxt, scalar)?;
            ctxt.degree = Degree(ctxt.degree.0 * scalar as usize);
        }
        // If the ciphertext cannot be multiplied without exceeding the degree max
        else {
            let acc = self.generate_accumulator(server_key, |x| (scalar as u64 * x) % modulus)?;
            self.programmable_bootstrap_keyswitch_assign(server_key, ctxt, &acc)?;
            ctxt.degree = Degree(server_key.message_modulus.0 - 1);
        }
        Ok(())
    }
}
