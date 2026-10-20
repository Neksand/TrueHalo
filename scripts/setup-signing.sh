#!/usr/bin/env bash
#
# Creates a stable, local (self-signed) code signing identity for TrueHalo.
#
# Why: macOS binds the Screen Recording permission (TCC) to the exact signed
# code. An ad-hoc signature is identified by its CDHash, which changes on every
# rebuild, so the user has to re-grant the permission after each build — the
# "granted, but asks again" loop. A certificate based signature gives a stable
# designated requirement:
#
#   identifier "com.truehalo.desktop" and certificate leaf = H"..."
#
# which keeps the permission valid across rebuilds.
#
set -euo pipefail

IDENTITY_NAME="${SIGN_IDENTITY:-TrueHalo Local Dev}"
KEYCHAIN="${HOME}/Library/Keychains/login.keychain-db"

info() { printf '\033[1;34m→\033[0m %s\n' "$1"; }
ok() { printf '\033[1;32m✓\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33m!\033[0m %s\n' "$1"; }

if security find-identity -v -p codesigning 2>/dev/null | grep -q "${IDENTITY_NAME}"; then
  ok "Signing identity '${IDENTITY_NAME}' is already installed."
  exit 0
fi

if security find-certificate -c "${IDENTITY_NAME}" "${KEYCHAIN}" >/dev/null 2>&1; then
  warn "Certificate '${IDENTITY_NAME}' exists in the keychain but is not marked as valid for signing."
fi

info "Creating a local signing certificate '${IDENTITY_NAME}'..."

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

cat > "${TMP_DIR}/openssl.cnf" <<EOF
[ req ]
distinguished_name = dn
x509_extensions = ext
prompt = no

[ dn ]
CN = ${IDENTITY_NAME}
O = TrueHalo
C = US

[ ext ]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
subjectKeyIdentifier = hash
EOF

# 1. Self-signed certificate with the codeSigning extended key usage.
openssl req -x509 -newkey rsa:2048 -nodes -sha256 -days 3650 \
  -keyout "${TMP_DIR}/key.pem" \
  -out "${TMP_DIR}/cert.pem" \
  -config "${TMP_DIR}/openssl.cnf" >/dev/null 2>&1

# 2. Bundle key + certificate into a PKCS#12 that the keychain can import.
#    macOS Security.framework only understands the legacy PBE algorithms, so we
#    ask OpenSSL for them explicitly (and fall back to `-legacy`).
if ! openssl pkcs12 -export \
  -inkey "${TMP_DIR}/key.pem" \
  -in "${TMP_DIR}/cert.pem" \
  -name "${IDENTITY_NAME}" \
  -out "${TMP_DIR}/identity.p12" \
  -passout pass:truehalo \
  -certpbe PBE-SHA1-3DES -keypbe PBE-SHA1-3DES -macalg sha1 >/dev/null 2>&1; then
  openssl pkcs12 -export \
    -inkey "${TMP_DIR}/key.pem" \
    -in "${TMP_DIR}/cert.pem" \
    -name "${IDENTITY_NAME}" \
    -out "${TMP_DIR}/identity.p12" \
    -passout pass:truehalo -legacy >/dev/null 2>&1 || {
      warn "Could not create the PKCS#12 file."
      exit 1
    }
fi

# 3. Import it and let codesign/security use the private key without prompts.
security import "${TMP_DIR}/identity.p12" \
  -k "${KEYCHAIN}" \
  -P truehalo \
  -T /usr/bin/codesign \
  -T /usr/bin/security >/dev/null

# Partition list may be locked; if it fails codesign will simply ask once and
# the user can click "Always Allow".
security set-key-partition-list \
  -S apple-tool:,apple:,codesign: \
  -s -k "" "${KEYCHAIN}" >/dev/null 2>&1 || \
  warn "Could not grant key access automatically — the first signature may make macOS ask you to «Always Allow»."

# 4. Trust the certificate for code signing (user domain, may prompt once).
security add-trusted-cert -r trustRoot -p codeSign -k "${KEYCHAIN}" \
  "${TMP_DIR}/cert.pem" >/dev/null 2>&1 || \
  warn "Could not add the certificate to the trusted list. Signing will most likely still work."

if security find-identity -v -p codesigning 2>/dev/null | grep -q "${IDENTITY_NAME}"; then
  ok "Done: identity '${IDENTITY_NAME}' is available for codesign."
else
  warn "Identity created but not shown as valid. The build script will fall back to an ad-hoc signature."
  exit 1
fi
