// Part of the Concrete Compiler Project, under the BSD3 License with Zama
// Exceptions. See
// https://github.com/zama-ai/concrete/blob/main/LICENSE.txt
// for license information.

#ifndef CONCRETELANG_COMMON_KEYSETS_H
#define CONCRETELANG_COMMON_KEYSETS_H

#include "concrete-protocol.capnp.h"
#include "concretelang/Common/Csprng.h"
#include "concretelang/Common/Error.h"
#include "concretelang/Common/Keys.h"
#include <functional>
#include <memory>
#include <stdlib.h>
#include <string>

using concretelang::error::Result;
using concretelang::error::StringError;
using concretelang::keys::LweBootstrapKey;
using concretelang::keys::LweKeyswitchKey;
using concretelang::keys::LweSecretKey;
using concretelang::keys::PackingKeyswitchKey;

namespace concretelang {
namespace keysets {

struct ClientPublicKeyset {
  std::vector<keys::LwePublicKey> lwePublicKeys;

  static ClientPublicKeyset
  fromProto(const Message<concreteprotocol::ClientPublicKeyset> &proto);

  Message<concreteprotocol::ClientPublicKeyset> toProto() const;

  Result<keys::LwePublicKey> getLwePublicKey(uint32_t secretKeyId) const;
};

struct ClientKeyset {
  std::vector<LweSecretKey> lweSecretKeys;

  static ClientKeyset
  fromProto(const Message<concreteprotocol::ClientKeyset> &proto);

  Message<concreteprotocol::ClientKeyset> toProto() const;

  Result<LweSecretKey> getLweSecretKey(uint32_t secretKeyId) const;
};

struct ServerKeyset {
  std::vector<LweBootstrapKey> lweBootstrapKeys;
  std::vector<LweKeyswitchKey> lweKeyswitchKeys;
  std::vector<PackingKeyswitchKey> packingKeyswitchKeys;

  static ServerKeyset
  fromProto(const Message<concreteprotocol::ServerKeyset> &proto);

  Message<concreteprotocol::ServerKeyset> toProto() const;
};

struct Keyset {
  ServerKeyset server;
  ClientKeyset client;

  /// Generates a fresh keyset from infos.
  Keyset(const Message<concreteprotocol::KeysetInfo> &info,
         concretelang::csprng::SecretCSPRNG &secretCsprng,
         csprng::EncryptionCSPRNG &encryptionCsprng);
  Keyset(const Message<concreteprotocol::KeysetInfo> &info, ServerKeyset server,
         ClientKeyset client)
      : server(server), client(client), info(info) {}

  static Keyset fromProto(const Message<concreteprotocol::Keyset> &proto);

  Message<concreteprotocol::Keyset> toProto() const;

  Result<ClientPublicKeyset>
  generateClientPublicKeyset(csprng::EncryptionCSPRNG &encryptionCsprng);

private:
  const Message<concreteprotocol::KeysetInfo> info;
};

class KeysetCache {
  std::string backingDirectoryPath;

public:
  KeysetCache(std::string backingDirectoryPath);

  Result<Keyset>
  getKeyset(const Message<concreteprotocol::KeysetInfo> &keysetInfo,
            __uint128_t secret_seed, __uint128_t encryption_seed);

private:
  KeysetCache() = default;
};

} // namespace keysets
} // namespace concretelang

#endif
