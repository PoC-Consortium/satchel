# Local security patch

Source: PoC-Consortium/btcx revision `7a6ec87fdd6c2aae5bb7db9f773d6ff7ca961247`, electrum-btcx crate. It is a member of the pact workspace so its security tests, lint and lockfile audit run in the normal gate.

TLS validates public CA certificates against webpki-roots, including hostname and validity. A valid CA certificate can rotate without a pin reset. Once CA trust has been recorded, an endpoint cannot downgrade to a self-signed identity without an explicit reset.

Only cryptographically self-signed certificates may use TOFU; they must still pass hostname and validity checks, and the TLS peer must prove possession of the certificate key before the fingerprint is saved. First self-signed contact can be intercepted. A changed self-signed certificate fails closed until verified out of band and reset. `tcp://` remains plaintext.

Pins live in `PACT_TLS_PIN_DIR`, otherwise the user profile's `.pact/tls-pins`. With pactd running, `pact-cli call tlspin ssl://host:port inspect` displays the current trust marker/fingerprint. After out-of-band verification, use action `forget` to reset; restart pactd to discard any existing connections. Forgetting also removes a remembered CA-only trust requirement. Never reset merely because a handshake fails.

Raw and batched Electrum transaction responses are checked against the requested txid. The patch also depends on x509-parser to recognize genuinely self-signed certificates; rcgen is used only in tests. Loopback tests exercise CA rotation, downgrade rejection, hostname checks and self-signed rotation/reset.
