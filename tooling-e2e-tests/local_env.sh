# ======================================================================================================================
# Node configuration
# ======================================================================================================================
NODE_RPC_PORT=8545
export NODE_RPC_PORT
NODE_RPC_URL="http://localhost:${NODE_RPC_PORT}"
export NODE_RPC_URL
CHAIN_ID=31337
export CHAIN_ID

# ======================================================================================================================
# Contract configuration
# ======================================================================================================================
DEPLOYER_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 # Pre-funded anvil account
export DEPLOYER_PRIVATE_KEY
PROTOCOL_DEPOSIT_FEE_BPS=0
export PROTOCOL_DEPOSIT_FEE_BPS
PROTOCOL_WITHDRAW_FEE_BPS=0
export PROTOCOL_WITHDRAW_FEE_BPS

# ======================================================================================================================
# User configuration
# ======================================================================================================================
ALICE_PUBLIC_KEY=0xCaCa0634D1CEF7BD98c07e65C14Dd1B619906dD4 # Random address without any funds by default
export ALICE_PUBLIC_KEY
ALICE_PRIVATE_KEY=0xd5a92218e15fd2854d458af1a50c902e6ababaa34c3dfea239a5ef5eba651250 # Corresponding private key
export ALICE_PRIVATE_KEY

BOB_PUBLIC_KEY=0xCAcA0eBb138B57A84EAF49B38dA122e507CE9a2f # Random address without any funds by default
export BOB_PUBLIC_KEY
BOB_PRIVATE_KEY=0x708e5a9d43a2a5b8eb4ccdd44ee7faebb559b5454a58f719dbb0ff904d047648 # Corresponding private key
export BOB_PRIVATE_KEY

CHARLIE_PUBLIC_KEY=0xcACA0B734B779c97fc25BF9723e622649cFCDDfe # Random address without any funds by default
export CHARLIE_PUBLIC_KEY
CHARLIE_PRIVATE_KEY=0xa68e4f75a36d07db56c06b1103c9158801f0f1f24a07deae9324ee86b0753494 # Corresponding private key
export CHARLIE_PRIVATE_KEY

TS_SDK_PUBLIC_KEY=0xC881A90D50c4F267AdD6e94720299E31b214aA5C # Random address without any funds by default
export TS_SDK_PUBLIC_KEY
TS_SDK_PRIVATE_KEY=0xbdb9193adbb1dc104b51c09f9cb4456d395ac334324d72c477039bca4a6cad5e # Corresponding private key
export TS_SDK_PRIVATE_KEY

WITHDRAWAL_PUBLIC_KEY=0xCaCA0cf7Ad10377313e391E8eF365c0ED0C51057 # Random address without any funds by default
export WITHDRAWAL_PUBLIC_KEY

ALICE_STATE_FILE=~/.shielder-state-alice
BOB_STATE_FILE=~/.shielder-state-bob
CHARLIE_STATE_FILE=~/.shielder-state-charlie

# ======================================================================================================================
# Relayer configuration
# ======================================================================================================================
RELAYER_PORT=4141 # Relayer service port
export RELAYER_PORT

FEE_DESTINATION=0xcaca0a3147bcaf6d7B706Fc5F5c325E6b0e7fb34 # Random address without any funds by default.
FEE_DESTINATION_KEY=0x11bc58beea7f9baab53bbef30a478ebc1657b475869b0d966e8c17a02218e529 # Corresponding signing key.
export FEE_DESTINATION_KEY

RELAYER_SIGNING_KEYS=0x547a81fc1782a6f29613dd15fe0f97321379875fe5a99e2a9d8258b4d51ac660,b466c488864884d64daf2ff0a117d4a39c10e6b294cb9199ff70730dcd84dcc0,ba07224f2bf545f5409be9fb09fd55d95ef9f8a2567461146b94a9a9e09ec1e2
export RELAYER_SIGNING_KEYS
RELAYER_SIGNER_ADDRESSES=("0xCacA011152e011634cFC7f663998af44BC55FF4c" "0xCaCa0Bd0baFbea855b0Bb2776F689b0f46cFA592" "0xCAcA018473E24A5d1B993C26e88943C49b63ED98")
export RELAYER_SIGNER_ADDRESSES

RELAYER_URL="http://localhost:${RELAYER_PORT}"
export RELAYER_URL

NONCE_POLICY=caching
export NONCE_POLICY
DRY_RUNNING=optimistic
export DRY_RUNNING

RECHARGE_THRESHOLD=100000000000000000 # 0.1
export RECHARGE_THRESHOLD
BALANCE_MONITOR_INTERVAL_SECS=5
export BALANCE_MONITOR_INTERVAL_SECS

# ======================================================================================================================
# Scheduler configuration
# ======================================================================================================================
SCHEDULER_PORT=3000
export SCHEDULER_PORT

SCHEDULER_INTERVAL_SECS=1
export SCHEDULER_INTERVAL_SECS

# Buffer time to account for delays in processing and relaying the request.
# Added to the relay_after_seconds + SCHEDULER_INTERVAL_SECS when sleeping before checking the result of a scheduled request.
SCHEDULER_REQUEST_BUFFER_SECS=2
export SCHEDULER_REQUEST_BUFFER_SECS

SCHEDULER_URL="http://localhost:${SCHEDULER_PORT}"
export SCHEDULER_URL

SCHEDULER_PUBLIC_KEY=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAv38LvBZx2IkVt0rvDFOWUUgF/kqXbV38NY6MCNBsC7wuqbNTztP8z4jcZSwpMcgisNijRfKY3DotoIw7EaxAIeQmcmrDIT3ALFLXbvLeEzyiGJe9m4ZZiTfQJGhgcotAyQzBDsNhgpxj7QWh/HCqXRYv6crKYfrbI3uoURY0o9xXiq2N6dz0WC8xpYA/+NqusXjqkj6JJJtDAIK1QOlKv23LR3Ht7ZKOruiHnlQ7PJ6xaP0yNVXyy41kagcJ+wBW96Rj4T4aJ2aZuB+YMA5aKnoJEiex66SvoqNaprjZxkgPiVHsjZZh4DFrGyggEN7Wnsab6yGsEWoIp5pGT0/8LwIDAQAB
export SCHEDULER_PUBLIC_KEY
SCHEDULER_PRIVATE_KEY=MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC/fwu8FnHYiRW3Su8MU5ZRSAX+SpdtXfw1jowI0GwLvC6ps1PO0/zPiNxlLCkxyCKw2KNF8pjcOi2gjDsRrEAh5CZyasMhPcAsUtdu8t4TPKIYl72bhlmJN9AkaGByi0DJDMEOw2GCnGPtBaH8cKpdFi/pysph+tsje6hRFjSj3FeKrY3p3PRYLzGlgD/42q6xeOqSPokkm0MAgrVA6Uq/bctHce3tko6u6IeeVDs8nrFo/TI1VfLLjWRqBwn7AFb3pGPhPhonZpm4H5gwDloqegkSJ7HrpK+io1qmuNnGSA+JUeyNlmHgMWsbKCAQ3taexpvrIawRaginmkZPT/wvAgMBAAECggEAXroW8PTdXjL5xhDQTXWvvwd0x4TYgWZjk46vINEREYEGSDYIOCDPR045T+bXfeO1nWuc4MYP7ajt1yRnN+fQVbOJfUHowrHMNWDJZv3G8xckTCwLxDpiYil00f278A2If6vKeGYX/0cjeyU1Ky4EaEXIb8s779nquxO5gFsOsRs8NwI/UxgBBTSkGL6P5mK9Bxo2OFmcCJ62eKQg8hsLT95/XNVai+IjdTzxFaWpeh//sv+koqefoxPpmgw5Jg4IHoa4JA0vmPR2zgsCEQPuAokSRFhNbYo3sm3lCALkAay3P37AnWN0/IKFz/JrIvSHRzDrOOqFuGkTdLSGArk78QKBgQDfVlt2VLRE0jE7M8POtRQb/ajfSNAhqv+f30FaZi6Jjr16Hq+Hh3ArWthNfhmhhPeL/X8iKaJX+XfmbBjFH+hKs7lJmXh3l4B3bdYTpb1f5NfqRX+DKBFoGs2DZFm3hOb82f5elmWVDnk5Jvzyr2syIrwavhHSUFCDX1J002CsuQKBgQDbgJMUGv7xe2nL2te0Tq0EMZJ0t8WYcKe77ircWVX5kpejaFpb9zaVtrBWByCqwovleSSY2BhhAQpsdDEQlgVa81e7IMqzEqJ4R7rrhQyf2iryQmIH8gy0IKNIfUnuaONaDX5OT1kFvPqgcHmlKUsxu7nAKyUig55LkVsaouUMJwKBgQDN3nHC2wOs6MBe3TGiikJ2JtWStTC9yD2IJii85rIBNOWt+zk8Sfl4zW3TyOhhfgseLNcTK6OtSpVefsg6dVYP/ilPEbv9KVdPP5z1UXMGQRFSryuCP94Dw6krc8ZLp6Ow8lVK5YHP656Ms0a5gj/i4Cfa0tfksQn2f1inve1AaQKBgF7I6ron3idQ76KD4dmOj8/byC52nPDy1LumYQO94A5havG5QiusWPnVZdgAQ+75C4gQA+mshvp8hI/IrNdIsQ9RPXUk9+LUDSx7bQ/+6DCu+RSb7dEYiLvPzDKlR6OXz3U3lm8ifIKp1GArsZvMdRgIxHFjMCnzRAvFvN2HsfjJAoGBAJDSrek5gt4LusjHwi7MV4FZ61iACzsr7KOJhJiGR2uY+o+cNo3sgeTs8erh85Ulto/GlNR6dMjSjaS9kUJ3aLlgVbgQepEH/pIC5lQcz2aJGuS78mF19oO/Yh8nXUPr4vGFo4wo9DmurluhIF7Cpr4YqceYoGozpcMZn9EoPUvL
export SCHEDULER_PRIVATE_KEY
