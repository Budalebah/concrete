// Part of the Concrete Compiler Project, under the BSD3 License with Zama
// Exceptions. See
// https://github.com/zama-ai/concrete-compiler-internal/blob/master/LICENSE.txt
// for license information.

#ifndef CONCRETELANG_RUNTIME_WRAPPERS_H
#define CONCRETELANG_RUNTIME_WRAPPERS_H

#include "concrete-ffi.h"

ForeignPlaintextList_u64 *
runtime_foreign_plaintext_list_u64(int *err, uint64_t *allocated,
                                   uint64_t *aligned, uint64_t offset,
                                   uint64_t size_dim0, uint64_t stride_dim0,
                                   uint64_t size, uint32_t precision);

#endif
