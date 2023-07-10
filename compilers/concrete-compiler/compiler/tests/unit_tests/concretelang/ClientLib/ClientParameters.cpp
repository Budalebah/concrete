#include <cassert>
#include <gtest/gtest.h>

#include "concretelang/ClientLib/ClientParameters.h"
#include "concretelang/ClientLib/EncryptedArguments.h"
#include "tests_tools/assert.h"

namespace clientlib = concretelang::clientlib;

TEST(Support, client_parameters_json_serde) {
  clientlib::ClientParameters params0;
  assert(params0.secretKeys.size() == clientlib::BIG_KEY);
  params0.secretKeys.push_back({14});

  assert(params0.secretKeys.size() == clientlib::SMALL_KEY);
  params0.secretKeys.push_back({12});

  params0.bootstrapKeys.push_back({
      /*.inputSecretKeyID = */ clientlib::SMALL_KEY,
      /*.outputSecretKeyID = */ clientlib::BIG_KEY,
      /*.level = */ 1,
      /*.baseLog = */ 2,
      /*.glweDimension = */ 3,
      /*.variance = */ 0.001,
      /*.polynomialSize = */ 1024,
      /*.inputLweDimension = */ 600,
  });

  params0.bootstrapKeys.push_back({
      /*.inputSecretKeyID = */ clientlib::BIG_KEY,
      /*.outputSecretKeyID = */ clientlib::SMALL_KEY,
      /*.level = */ 3,
      /*.baseLog = */ 2,
      /*.glweDimension = */ 1,
      /*.variance = */ 0.0001,
      /*.polynomialSize = */ 1024,
      /*.inputLweDimension = */ 600,
  });
  params0.keyswitchKeys.push_back({
      /*.inputSecretKeyID = */
      clientlib::BIG_KEY,
      /*.outputSecretKeyID = */
      clientlib::SMALL_KEY,
      /*.level = */ 1,
      /*.baseLog = */ 2,
      /*.variance = */ 3,
  });
  params0.inputs = {
      {
          /*.encryption = */ {
              {clientlib::SMALL_KEY, 0.00, {4, {1, 2, 3, 4}, false}}},
          /*.shape = */ {32, {1, 2, 3, 4}, 1 * 2 * 3 * 4, false},
          /*.chunkInfo = */ std::nullopt,
          /* compression = */ false,
      },
      {
          /*.encryption = */ {
              {clientlib::SMALL_KEY, 0.00, {5, {1, 2, 3, 4}, false}}},
          /*.shape = */ {8, {4, 4, 4, 4}, 4 * 4 * 4 * 4, false},
          /*.chunkInfo = */ std::nullopt,
          /* compression = */ false,
      },
  };
  params0.outputs = {
      {
          /*.encryption = */ {
              {clientlib::SMALL_KEY, 0.00, {5, {1, 2, 3, 4}, false}}},
          /*.shape = */ {8, {4, 4, 4, 4}, 4 * 4 * 4 * 4, false},
          /*.chunkInfo = */ std::nullopt,
          /* compression = */ false,
      },
  };
  auto json = clientlib::toJSON(params0);
  std::string jsonStr;
  llvm::raw_string_ostream os(jsonStr);
  os << json;
  auto parseResult = llvm::json::parse<clientlib::ClientParameters>(jsonStr);
  ASSERT_EXPECTED_VALUE(parseResult, params0);
}
