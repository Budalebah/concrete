//! # WARNING: this module is experimental.
use crate::ciphertext::Degree;
use crate::engine::{EngineResult, ShortintEngine};
use crate::wopbs::WopbsKey;
use crate::{Ciphertext, ClientKey, Parameters, ServerKey};

use crate::server_key::MaxDegree;
use concrete_core::prelude::*;

impl ShortintEngine {
    //Fills with functional packing key switch keys.
    pub(crate) fn new_wopbs_key(
        &mut self,
        cks: &ClientKey,
        sks: &ServerKey,
        parameters: &Parameters,
    ) -> EngineResult<WopbsKey> {
        //Independent client key generation dedicated to the WoPBS
        let small_lwe_secret_key: LweSecretKey64 = self
            .engine
            .generate_new_lwe_secret_key(parameters.lwe_dimension)?;

        let glwe_secret_key: GlweSecretKey64 = self
            .engine
            .generate_new_glwe_secret_key(parameters.glwe_dimension, parameters.polynomial_size)?;

        let large_lwe_secret_key = self
            .engine
            .transform_glwe_secret_key_to_lwe_secret_key(glwe_secret_key.clone())?;

        //BSK dedicated to the WoPBS
        let var_rlwe = Variance(parameters.glwe_modular_std_dev.get_variance());

        let bootstrap_key: LweBootstrapKey64 = self.par_engine.generate_new_lwe_bootstrap_key(
            &small_lwe_secret_key,
            &glwe_secret_key,
            parameters.pbs_base_log,
            parameters.pbs_level,
            var_rlwe,
        )?;

        // Creation of the bootstrapping key in the Fourier domain
        let small_bsk: FftFourierLweBootstrapKey64 =
            self.fft_engine.convert_lwe_bootstrap_key(&bootstrap_key)?;

        // Convert into a variance for lwe context
        let var_lwe = Variance(parameters.lwe_modular_std_dev.get_variance());
        //KSK encryption_key -> small WoPBS key (used in the 1st KS in the extract bit)
        let ksk_wopbs_large_to_wopbs_small = self.engine.generate_new_lwe_keyswitch_key(
            &large_lwe_secret_key,
            &small_lwe_secret_key,
            parameters.ks_level,
            parameters.ks_base_log,
            var_lwe,
        )?;

        //KSK to convert from input ciphertext key to the wopbs input one
        //let var_lwe = Variance(cks.parameters.lwe_modular_std_dev.get_variance());
        let ksk_pbs_large_to_wopbs_large = self.engine.generate_new_lwe_keyswitch_key(
            &cks.lwe_secret_key,
            &large_lwe_secret_key,
            cks.parameters.ks_level,
            cks.parameters.ks_base_log,
            var_lwe,
        )?;

        //KSK large_wopbs_key -> small PBS key (used after the WoPBS computation to compute a
        // classical PBS. This allows compatibility between PBS and WoPBS
        let var_lwe_pbs = Variance(cks.parameters.lwe_modular_std_dev.get_variance());
        let ksk_wopbs_large_to_pbs_small = self.engine.generate_new_lwe_keyswitch_key(
            &large_lwe_secret_key,
            &cks.lwe_secret_key_after_ks,
            cks.parameters.ks_level,
            cks.parameters.ks_base_log,
            var_lwe_pbs,
        )?;

        let cbs_pfpksk = self
            .engine
            .generate_new_lwe_circuit_bootstrap_private_functional_packing_keyswitch_keys(
                &large_lwe_secret_key,
                &glwe_secret_key,
                parameters.pfks_base_log,
                parameters.pfks_level,
                Variance(parameters.pfks_modular_std_dev.get_variance()),
            )?;

        let wopbs_server_key = ServerKey {
            key_switching_key: ksk_wopbs_large_to_wopbs_small,
            bootstrapping_key: small_bsk,
            message_modulus: parameters.message_modulus,
            carry_modulus: parameters.carry_modulus,
            max_degree: MaxDegree(parameters.message_modulus.0 * parameters.carry_modulus.0 - 1),
        };

        let pbs_server_key = ServerKey {
            key_switching_key: ksk_wopbs_large_to_pbs_small,
            bootstrapping_key: sks.bootstrapping_key.clone(),
            message_modulus: cks.parameters.message_modulus,
            carry_modulus: cks.parameters.carry_modulus,
            max_degree: MaxDegree(
                cks.parameters.message_modulus.0 * cks.parameters.carry_modulus.0 - 1,
            ),
        };

        let wopbs_key = WopbsKey {
            wopbs_server_key,
            pbs_server_key,
            cbs_pfpksk,
            ksk_pbs_to_wopbs: ksk_pbs_large_to_wopbs_large,
            param: *parameters,
        };
        Ok(wopbs_key)
    }

    pub(crate) fn extract_bits(
        &mut self,
        delta_log: DeltaLog,
        lwe_in: &LweCiphertext64,
        wopbs_key: &WopbsKey,
        extracted_bit_count: ExtractedBitsCount,
    ) -> EngineResult<LweCiphertextVector64> {
        let server_key = &wopbs_key.wopbs_server_key;

        let lwe_size = server_key
            .key_switching_key
            .output_lwe_dimension()
            .to_lwe_size();
        let mut output = self.engine.create_lwe_ciphertext_vector_from(
            vec![0u64; lwe_size.0 * extracted_bit_count.0],
            lwe_size,
        )?;

        self.fft_engine.discard_extract_bits_lwe_ciphertext(
            &mut output,
            &lwe_in,
            &server_key.bootstrapping_key,
            &server_key.key_switching_key,
            extracted_bit_count,
            DeltaLog(delta_log.0),
        )?;
        Ok(output)
    }

    pub(crate) fn circuit_bootstrap_with_bits(
        &mut self,
        wopbs_key: &WopbsKey,
        extracted_bits: &LweCiphertextVectorView64<'_>,
        lut: &PlaintextVector64,
        count: LweCiphertextCount,
    ) -> EngineResult<LweCiphertextVector64> {
        let sks = &wopbs_key.wopbs_server_key;
        let mut output_cbs_vp_ct_container =
            vec![0u64; sks.bootstrapping_key.output_lwe_dimension().to_lwe_size().0 * count.0];

        let mut output_cbs_vp_ct = self.engine.create_lwe_ciphertext_vector_from(
            output_cbs_vp_ct_container.as_mut_slice(),
            sks.bootstrapping_key.output_lwe_dimension().to_lwe_size(),
        )?;

        println!(
            "IN circuit_bootstrap_with_bits: param = {:?}",
            wopbs_key.param
        );

        self.fft_engine
            .discard_circuit_bootstrap_boolean_vertical_packing_lwe_ciphertext_vector(
                &mut output_cbs_vp_ct,
                extracted_bits,
                &sks.bootstrapping_key,
                lut,
                wopbs_key.param.cbs_level,
                wopbs_key.param.cbs_base_log,
                &wopbs_key.cbs_pfpksk,
            )?;

        let output_vector = self.engine.create_lwe_ciphertext_vector_from(
            output_cbs_vp_ct_container,
            sks.bootstrapping_key.output_lwe_dimension().to_lwe_size(),
        )?;

        Ok(output_vector)
    }

    pub(crate) fn extract_bits_circuit_bootstrapping(
        &mut self,
        wopbs_key: &WopbsKey,
        ct_in: &Ciphertext,
        lut: &[u64],
        delta_log: DeltaLog,
        nb_bit_to_extract: ExtractedBitsCount,
    ) -> EngineResult<Ciphertext> {
        let extracted_bits =
            self.extract_bits(delta_log, &ct_in.ct, wopbs_key, nb_bit_to_extract)?;

        let extracted_bit_size = extracted_bits.lwe_dimension().to_lwe_size();
        let data = self
            .engine
            .consume_retrieve_lwe_ciphertext_vector(extracted_bits)?;
        let extrated_bits_view = self
            .engine
            .create_lwe_ciphertext_vector_from(data.as_slice(), extracted_bit_size)?;

        let plaintext_lut = self.engine.create_plaintext_vector_from(lut)?;

        let ciphertext = self.circuit_bootstrap_with_bits(
            wopbs_key,
            &extrated_bits_view,
            &plaintext_lut,
            LweCiphertextCount(1),
        )?;

        let container = self
            .engine
            .consume_retrieve_lwe_ciphertext_vector(ciphertext)?;
        let ct = self.engine.create_lwe_ciphertext_from(container)?;

        let ct_out = self.keyswitch_to_pbs_params(wopbs_key, &ct)?;
        let sks = &wopbs_key.wopbs_server_key;
        let ct_out = Ciphertext {
            ct: ct_out,
            degree: Degree(sks.message_modulus.0 - 1),
            message_modulus: sks.message_modulus,
            carry_modulus: sks.carry_modulus,
        };

        Ok(ct_out)
    }

    pub(crate) fn programmable_bootstrapping_without_padding(
        &mut self,
        wopbs_key: &WopbsKey,
        ct_in: &Ciphertext,
        lut: &[u64],
    ) -> EngineResult<Ciphertext> {
        let sks = &wopbs_key.wopbs_server_key;
        let delta = (1_usize << 63) / (sks.message_modulus.0 * sks.carry_modulus.0) * 2;
        let delta_log = DeltaLog(f64::log2(delta as f64) as usize);

        let nb_bit_to_extract =
            f64::log2((sks.message_modulus.0 * sks.carry_modulus.0) as f64) as usize;

        let ciphertext = self.extract_bits_circuit_bootstrapping(
            wopbs_key,
            ct_in,
            lut,
            delta_log,
            ExtractedBitsCount(nb_bit_to_extract),
        )?;

        Ok(ciphertext)
    }

    pub(crate) fn keyswitch_to_wopbs_params(
        &mut self,
        wopbs_key: &WopbsKey,
        ct_in: &LweCiphertext64,
    ) -> EngineResult<LweCiphertext64> {
        // move to pbs parameters to wopbs parameters
        // let acc = self.generate_accumulator(&wopbs_key.wopbs_server_key, |x| x)?;
        // let pol_size = wopbs_key.param.polynomial_size.0;
        // let glwe_dim = wopbs_key.param.glwe_dimension.0;
        // let mut tmp = self
        //     .engine
        //     .create_lwe_ciphertext_from(vec![0; pol_size * glwe_dim + 1])?;
        //let mut tmp = ct_in.clone();
        let (buffers, engine, fftw_engine) = self.buffers_for_key(&wopbs_key.wopbs_server_key);
        // Compute a key switch
        engine.discard_keyswitch_lwe_ciphertext(
            &mut buffers.buffer_lwe_after_ks,
            ct_in,
            &wopbs_key.ksk_pbs_to_wopbs,
        )?;

        // fftw_engine.discard_bootstrap_lwe_ciphertext(
        //     &mut tmp,
        //     &buffers.buffer_lwe_after_ks,
        //     &acc,
        //     &wopbs_key.wopbs_server_key.bootstrapping_key,
        // )?;

        Ok(buffers.buffer_lwe_after_ks.clone())
    }

    pub(crate) fn keyswitch_to_pbs_params(
        &mut self,
        wopbs_key: &WopbsKey,
        ct_in: &LweCiphertext64,
    ) -> EngineResult<LweCiphertext64> {
        // move to wopbs parameters to pbs parameters
        //Keyswitch-PBS:
        // 1. KS to go back to the original encryption key
        // 2. PBS to remove the noise added by the previous KS
        //
        let acc = self.generate_accumulator(&wopbs_key.pbs_server_key, |x| x)?;
        let (buffers, engine, fftw_engine) = self.buffers_for_key(&wopbs_key.pbs_server_key);
        // Compute a key switch
        engine.discard_keyswitch_lwe_ciphertext(
            &mut buffers.buffer_lwe_after_ks,
            ct_in,
            &wopbs_key.pbs_server_key.key_switching_key,
        )?;

        let out_lwe_size = wopbs_key
            .pbs_server_key
            .bootstrapping_key
            .output_lwe_dimension()
            .to_lwe_size();
        let mut ct_out = engine.create_lwe_ciphertext_from(vec![0; out_lwe_size.0])?;

        // Compute a bootstrap
        fftw_engine.discard_bootstrap_lwe_ciphertext(
            &mut ct_out,
            &buffers.buffer_lwe_after_ks,
            &acc,
            &wopbs_key.pbs_server_key.bootstrapping_key,
        )?;

        Ok(ct_out)
    }

    pub(crate) fn programmable_bootstrapping(
        &mut self,
        wopbs_key: &WopbsKey,
        sks: &ServerKey,
        ct_in: &Ciphertext,
        lut: &[u64],
    ) -> EngineResult<Ciphertext> {

        // First PBS to remove the noise
        let acc = self.generate_accumulator(&sks, |x| x)?;
        println!("GENERATION ACC OK ");
        let ct_clean = self.programmable_bootstrap_keyswitch(&sks, &ct_in, &acc)?;
        println!("PBS OK  ");



        //KS from encryption key to wopbs
        // let pol_size = wopbs_key.param.polynomial_size.0;
        // let glwe_dim = wopbs_key.param.glwe_dimension.0;
        // let mut tmp = self
        //     .engine
        //     .create_lwe_ciphertext_from(vec![0; pol_size * glwe_dim + 1])?;
        //let mut tmp = ct_in.clone();

        // To make borrow checker happy
        let engine = &mut self.engine;
        let buffers_map = &mut self.buffers;

        //let (buffers, engine, fftw_engine) = self.buffers_for_key(&wopbs_key.wopbs_server_key);

        let zero_plaintext = engine.create_plaintext_from(&0_u64).unwrap();
        let mut buffer_lwe_after_ks = engine
            .trivially_encrypt_lwe_ciphertext(
                wopbs_key.ksk_pbs_to_wopbs
                    .output_lwe_dimension()
                    .to_lwe_size(),
                &zero_plaintext,
            )
            .unwrap();

        println!("Before KS: ct_in dim = {}, ct_out dim = {}", ct_clean.ct.lwe_dimension()
            .to_lwe_size().0, buffer_lwe_after_ks.lwe_dimension().to_lwe_size().0 );

        println!("Before KS: ksk_in dim = {}, ksk_out dim = {}",
            wopbs_key.ksk_pbs_to_wopbs.input_lwe_dimension().0, wopbs_key.ksk_pbs_to_wopbs
                     .output_lwe_dimension().to_lwe_size().0);

        // Compute a key switch
        engine.discard_keyswitch_lwe_ciphertext(
            &mut buffer_lwe_after_ks,
            &ct_clean.ct,
            &wopbs_key.ksk_pbs_to_wopbs,
        )?;

        println!("AFTER KS OK");
        println!("AFTER KS: ct_in dim = {}, ct_out dim = {}", buffer_lwe_after_ks.lwe_dimension()
            .to_lwe_size().0, buffer_lwe_after_ks.lwe_dimension().to_lwe_size().0 );

        let mut ct_to_wopbs = Ciphertext{
            ct: buffer_lwe_after_ks.clone(),
            degree: ct_in.degree,
            message_modulus: ct_in.message_modulus,
            carry_modulus: ct_in.carry_modulus
        };

        let sks = &wopbs_key.wopbs_server_key;
        let delta = (1_usize << 63) / (sks.message_modulus.0 * sks.carry_modulus.0);
        let delta_log = DeltaLog(f64::log2(delta as f64) as usize);
        let nb_bit_to_extract =
            f64::log2((sks.message_modulus.0 * sks.carry_modulus.0) as f64) as usize;

        let ct_out = self.extract_bits_circuit_bootstrapping(
            wopbs_key,
            &ct_to_wopbs,
            lut,
            delta_log,
            ExtractedBitsCount(nb_bit_to_extract),
        )?;

        Ok(ct_out)
    }

    pub(crate) fn programmable_bootstrapping_native_crt(
        &mut self,
        wopbs_key: &WopbsKey,
        ct_in: &mut Ciphertext,
        lut: &[u64],
    ) -> EngineResult<Ciphertext> {
        let nb_bit_to_extract =
            f64::log2((ct_in.message_modulus.0 * ct_in.carry_modulus.0) as f64).ceil() as usize;
        let delta_log = DeltaLog(64 - nb_bit_to_extract);

        // trick ( ct - delta/2 + delta/2^4  )
        let lwe_size = ct_in.ct.lwe_dimension().to_lwe_size().0;
        let mut cont = vec![0u64; lwe_size];
        cont[lwe_size - 1] =
            (1 << (64 - nb_bit_to_extract - 1)) - (1 << (64 - nb_bit_to_extract - 5));
        let tmp = self.engine.create_lwe_ciphertext_from(cont)?;
        self.engine.fuse_sub_lwe_ciphertext(&mut ct_in.ct, &tmp)?;

        let ciphertext = self.extract_bits_circuit_bootstrapping(
            wopbs_key,
            ct_in,
            lut,
            delta_log,
            ExtractedBitsCount(nb_bit_to_extract),
        )?;

        Ok(ciphertext)
    }

    /// Temporary wrapper used in `concrete-integer`.
    ///
    /// # Warning Experimental
    pub fn circuit_bootstrapping_vertical_packing(
        &mut self,
        wopbs_key: &WopbsKey,
        vec_lut: Vec<Vec<u64>>,
        extracted_bits_blocks: Vec<LweCiphertextVector64>,
    ) -> Vec<LweCiphertext64> {
        let lwe_size = extracted_bits_blocks[0].lwe_dimension().to_lwe_size();

        let mut all_datas = vec![];
        for lwe_vec in extracted_bits_blocks.into_iter() {
            let data = self
                .engine
                .consume_retrieve_lwe_ciphertext_vector(lwe_vec)
                .unwrap();

            all_datas.extend_from_slice(data.as_slice());
        }

        let flatenned_extracted_bits_view = self
            .engine
            .create_lwe_ciphertext_vector_from(all_datas.as_slice(), lwe_size)
            .unwrap();

        let flattened_lut: Vec<u64> = vec_lut.iter().flatten().copied().collect();
        let plaintext_lut = self
            .engine
            .create_plaintext_vector_from(&flattened_lut)
            .unwrap();
        let output = self
            .circuit_bootstrap_with_bits(
                wopbs_key,
                &flatenned_extracted_bits_view,
                &plaintext_lut,
                LweCiphertextCount(vec_lut.len()),
            )
            .unwrap();

        assert_eq!(output.lwe_ciphertext_count().0, vec_lut.len());

        let output_container = self
            .engine
            .consume_retrieve_lwe_ciphertext_vector(output)
            .unwrap();
        let lwes: Result<Vec<_>, Box<dyn std::error::Error>> = output_container
            .chunks_exact(output_container.len() / vec_lut.len())
            .map(|s| {
                let ct_before_ks = self.engine.create_lwe_ciphertext_from(s.to_vec())?;
                let ct_after_ks = self
                    .keyswitch_to_pbs_params(wopbs_key, &ct_before_ks)
                    .unwrap();
                Ok(ct_after_ks)
            })
            .collect();

        let lwes = lwes.unwrap();

        assert_eq!(lwes.len(), vec_lut.len());
        lwes
    }
}
