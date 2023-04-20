#include <cstdint>
#include <gtest/gtest.h>
#include <setup_and_teardown.h>
#include <stdio.h>
#include <stdlib.h>

const unsigned REPETITIONS = 2;
const unsigned SAMPLES = 50;

typedef struct {
  int lwe_dimension;
  int glwe_dimension;
  int polynomial_size;
  double lwe_modular_variance;
  double glwe_modular_variance;
  int pbs_base_log;
  int pbs_level;
  int pksk_base_log;
  int pksk_level;
  int cbs_base_log;
  int cbs_level;
  int number_of_inputs;
} CircuitBootstrapTestParams;

class CircuitBootstrapTestPrimitives_u64
    : public ::testing::TestWithParam<CircuitBootstrapTestParams> {
protected:
  int lwe_dimension;
  int glwe_dimension;
  int polynomial_size;
  double lwe_modular_variance;
  double glwe_modular_variance;
  int pbs_base_log;
  int pbs_level;
  int pksk_base_log;
  int pksk_level;
  int cbs_base_log;
  int cbs_level;
  int number_of_inputs;
  int number_of_bits_of_message_including_padding;
  int ggsw_size;
  uint64_t delta;
  int delta_log;
  Csprng *csprng;
  cudaStream_t *stream;
  int gpu_index = 0;
  uint64_t *lwe_sk_in_array;
  uint64_t *lwe_sk_out_array;
  uint64_t *plaintexts;
  double *d_fourier_bsk_array;
  uint64_t *d_pksk_array;
  uint64_t *d_lwe_ct_in_array;
  uint64_t *d_ggsw_ct_out_array;
  uint64_t *d_lut_vector_indexes;
  int8_t *cbs_buffer;

public:
  // Test arithmetic functions
  void SetUp() {
    stream = cuda_create_stream(0);

    // TestParams
    lwe_dimension = (int)GetParam().lwe_dimension;
    glwe_dimension = (int)GetParam().glwe_dimension;
    polynomial_size = (int)GetParam().polynomial_size;
    lwe_modular_variance = (double)GetParam().lwe_modular_variance;
    glwe_modular_variance = (double)GetParam().glwe_modular_variance;
    pbs_base_log = (int)GetParam().pbs_base_log;
    pbs_level = (int)GetParam().pbs_level;
    pksk_base_log = (int)GetParam().pksk_base_log;
    pksk_level = (int)GetParam().pksk_level;
    cbs_base_log = (int)GetParam().cbs_base_log;
    cbs_level = (int)GetParam().cbs_level;
    number_of_inputs = (int)GetParam().number_of_inputs;

    // We generate binary messages
    number_of_bits_of_message_including_padding = 2;
    ggsw_size = cbs_level * (glwe_dimension + 1) * (glwe_dimension + 1) *
                polynomial_size;

    circuit_bootstrap_setup(
        stream, &csprng, &lwe_sk_in_array, &lwe_sk_out_array,
        &d_fourier_bsk_array, &d_pksk_array, &plaintexts, &d_lwe_ct_in_array,
        &d_ggsw_ct_out_array, &d_lut_vector_indexes, &cbs_buffer, lwe_dimension,
        glwe_dimension, polynomial_size, lwe_modular_variance,
        glwe_modular_variance, pksk_base_log, pksk_level, pbs_base_log,
        pbs_level, cbs_level, number_of_bits_of_message_including_padding,
        ggsw_size, &delta_log, &delta, number_of_inputs, REPETITIONS, SAMPLES,
        gpu_index);
  }

  void TearDown() {
    circuit_bootstrap_teardown(
        stream, csprng, lwe_sk_in_array, lwe_sk_out_array, d_fourier_bsk_array,
        d_pksk_array, plaintexts, d_lwe_ct_in_array, d_lut_vector_indexes,
        d_ggsw_ct_out_array, cbs_buffer, gpu_index);
  }
};

TEST_P(CircuitBootstrapTestPrimitives_u64, circuit_bootstrap) {
  void *v_stream = (void *)stream;
  uint64_t *ggsw_ct_out = (uint64_t *)malloc(ggsw_size * sizeof(uint64_t));
  for (uint r = 0; r < REPETITIONS; r++) {
    int bsk_size = (glwe_dimension + 1) * (glwe_dimension + 1) * pbs_level *
                   polynomial_size * (lwe_dimension + 1);
    double *d_fourier_bsk = d_fourier_bsk_array + (ptrdiff_t)(bsk_size * r);
    int pksk_list_size = pksk_level * (glwe_dimension + 1) * polynomial_size *
                         (glwe_dimension * polynomial_size + 1) *
                         (glwe_dimension + 1);
    uint64_t *d_pksk_list = d_pksk_array + (ptrdiff_t)(pksk_list_size * r);
    uint64_t *lwe_sk_out =
        lwe_sk_out_array + (ptrdiff_t)(r * glwe_dimension * polynomial_size);
    for (uint s = 0; s < SAMPLES; s++) {
      uint64_t *d_lwe_ct_in =
          d_lwe_ct_in_array +
          (ptrdiff_t)((r * SAMPLES * number_of_inputs + s * number_of_inputs) *
                      (lwe_dimension + 1));

      // Execute circuit bootstrap
      cuda_circuit_bootstrap_64(
          stream, gpu_index, (void *)d_ggsw_ct_out_array, (void *)d_lwe_ct_in,
          (void *)d_fourier_bsk, (void *)d_pksk_list,
          (void *)d_lut_vector_indexes, cbs_buffer, delta_log, polynomial_size,
          glwe_dimension, lwe_dimension, pbs_level, pbs_base_log, pksk_level,
          pksk_base_log, cbs_level, cbs_base_log, number_of_inputs,
          cuda_get_max_shared_memory(gpu_index));

      for (int i = 0; i < number_of_inputs; i++) {
        uint64_t plaintext = plaintexts[r * SAMPLES * number_of_inputs +
                                        s * number_of_inputs + i];
        uint64_t *decrypted =
            (uint64_t *)malloc(polynomial_size * (glwe_dimension + 1) *
                               cbs_level * sizeof(uint64_t));
        // Copy result back
        cuda_memcpy_async_to_cpu(
            ggsw_ct_out, d_ggsw_ct_out_array + i * ggsw_size,
            ggsw_size * sizeof(uint64_t), stream, gpu_index);
        cuda_synchronize_stream(v_stream);

        uint64_t multiplying_factor = -(plaintext >> delta_log);
        for (int l = 1; l < cbs_level + 1; l++) {
          for (int j = 0; j < glwe_dimension; j++) {
            uint64_t *res = decrypted + (ptrdiff_t)((l - 1) * polynomial_size *
                                                        (glwe_dimension + 1) +
                                                    j * polynomial_size);
            uint64_t *glwe_ct_out =
                ggsw_ct_out +
                (ptrdiff_t)((l - 1) * polynomial_size * (glwe_dimension + 1) *
                                (glwe_dimension + 1) +
                            j * polynomial_size * (glwe_dimension + 1));
            concrete_cpu_decrypt_glwe_ciphertext_u64(
                lwe_sk_out, res, glwe_ct_out, glwe_dimension, polynomial_size);

            for (int k = 0; k < polynomial_size; k++) {
              uint64_t expected_decryption =
                  lwe_sk_out[j * polynomial_size + k] * multiplying_factor;
              expected_decryption >>= (64 - cbs_base_log * l);
              uint64_t decoded_plaintext =
                  closest_representable(res[k], l, cbs_base_log) >>
                  (64 - cbs_base_log * l);
              EXPECT_EQ(expected_decryption, decoded_plaintext);
            }
          }
        }
        // Check last glwe on last level
        uint64_t *res =
            decrypted + (ptrdiff_t)((cbs_level - 1) * polynomial_size *
                                        (glwe_dimension + 1) +
                                    glwe_dimension * polynomial_size);
        uint64_t *glwe_ct_out =
            ggsw_ct_out +
            (ptrdiff_t)((cbs_level - 1) * polynomial_size *
                            (glwe_dimension + 1) * (glwe_dimension + 1) +
                        glwe_dimension * polynomial_size *
                            (glwe_dimension + 1));
        concrete_cpu_decrypt_glwe_ciphertext_u64(
            lwe_sk_out, res, glwe_ct_out, glwe_dimension, polynomial_size);

        for (int k = 0; k < polynomial_size; k++) {
          uint64_t expected_decryption = (k == 0) ? plaintext / delta : 0;
          uint64_t decoded_plaintext =
              closest_representable(res[k], cbs_level, cbs_base_log) >>
              (64 - cbs_base_log * cbs_level);
          EXPECT_EQ(expected_decryption, decoded_plaintext);
        }
        free(decrypted);
      }
    }
  }
  free(ggsw_ct_out);
}

// Defines for which parameters set the PBS will be tested.
// It executes each test for all pairs on phis X qs (Cartesian product)
::testing::internal::ParamGenerator<CircuitBootstrapTestParams> cbs_params_u64 =
    ::testing::Values(
        // n, k, N, lwe_variance, glwe_variance, pbs_base_log, pbs_level,
        // pksk_base_log, pksk_level, cbs_base_log, cbs_level, number_of_inputs
        (CircuitBootstrapTestParams){760, 2, 1024, 7.52316384526264e-37,
                                     7.52316384526264e-37, 12, 3, 17, 2, 8, 2,
                                     13});

std::string
printParamName(::testing::TestParamInfo<CircuitBootstrapTestParams> p) {
  CircuitBootstrapTestParams params = p.param;

  return "n_" + std::to_string(params.lwe_dimension) + "_k_" +
         std::to_string(params.glwe_dimension) + "_N_" +
         std::to_string(params.polynomial_size) + "_pbs_base_log_" +
         std::to_string(params.pbs_base_log) + "_pbs_level_" +
         std::to_string(params.pbs_level) + "_pksk_base_log_" +
         std::to_string(params.pksk_base_log) + "_pksk_level_" +
         std::to_string(params.pksk_level) + "_cbs_base_log_" +
         std::to_string(params.cbs_base_log) + "_cbs_level_" +
         std::to_string(params.cbs_level);
}

INSTANTIATE_TEST_CASE_P(CircuitBootstrapInstantiation,
                        CircuitBootstrapTestPrimitives_u64, cbs_params_u64,
                        printParamName);
