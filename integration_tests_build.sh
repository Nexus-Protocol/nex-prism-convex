# Set `CONTRACTS_SCRIPTS_PATH` environement variable.
# It should points to the contracts scripts directory on your machine

if [[ -z "${CONTRACTS_SCRIPTS_PATH}" ]]; then
    echo "Error: you must set \`CONTRACTS_SCRIPTS_PATH\` env variable"
    exit 1
fi

# You can add more contracts that are built with integration tests to this array
contracts=(
    "nexus_prism_autocompounder"
    "nexus_prism_staking"
    "nexus_prism_vault"
)

# Copy artifacts
for contract in "${contracts[@]}"
do
    cd "contracts/${contract}"
    cargo build --release --features integration_tests_build --target=wasm32-unknown-unknown
    cd ../..
    wasm-strip "target/wasm32-unknown-unknown/release/${contract}.wasm"
    cp "target/wasm32-unknown-unknown/release/${contract}.wasm" "${CONTRACTS_SCRIPTS_PATH}/wasm_artifacts/nexus/nexprism/${contract}.wasm"
done