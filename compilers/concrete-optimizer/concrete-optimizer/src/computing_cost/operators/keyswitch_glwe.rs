use super::super::complexity::Complexity;
use crate::parameters::KeyswitchGlweParameters;

#[derive(Default, Clone)]
pub struct KsGlweComplexity;

impl KsGlweComplexity {
    #[allow(clippy::cast_possible_wrap)]
    #[allow(clippy::unused_self)]
    pub fn complexity(
        &self,
        params: KeyswitchGlweParameters,
        _ciphertext_modulus_log: u32,
    ) -> Complexity {
        let input_glwe_dimension = params.input_glwe_params.glwe_dimension as i64;
        let output_glwe_dimension = params.output_glwe_params.glwe_dimension as i64;
        let level = params.ks_decomposition_parameter.level as i64;

        let output_glwe_size = output_glwe_dimension + 1;
        let count_decomposition = input_glwe_dimension * level;
        let count_mul = input_glwe_dimension * level * output_glwe_size;
        let count_add = (input_glwe_dimension * level - 1) * output_glwe_size + 1;
        (count_decomposition + count_mul + count_add) as Complexity
    }
}