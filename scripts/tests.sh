#!/usr/bin/env bash

unit_tests() {
    set -eux
    cargo test --workspace --all-targets
}

smoke_test() {
    set -eux
    local test_name=$1
    shift

    case "$test_name" in
        app_install)
            RUST_LOG=info cargo run --package app_install -- --agents 2 --behaviour minimal:1 --behaviour large:1 --duration 5 --no-progress
            ;;

        dht_sync_lag)
            RUST_LOG=info cargo run --package dht_sync_lag -- --agents 2 --behaviour write:1 --behaviour record_lag:1 --duration 5 --no-progress
            ;;
        
        first_call)
            RUST_LOG=info cargo run --package first_call -- --duration 5 --no-progress
            ;;
        
        kitsune_continuous_flow)
            local kitsune_host="${1:-127.0.0.1}"
            RUST_LOG=warn cargo run --package kitsune_continuous_flow -- --bootstrap-server-url "http://$kitsune_host" --signal-server-url "ws://$kitsune_host" --duration 15 --agents 2
            ;;

        local_signals)
            RUST_LOG=info cargo run --package local_signals -- --duration 5 --no-progress
            ;;
        
        remote_call_rate)
            RUST_LOG=warn MIN_AGENTS=2 cargo run --package remote_call_rate -- --agents 2 --duration 30 --no-progress
            ;;
        
        remote_signals)
            RUST_LOG=warn MIN_AGENTS=2 cargo run --package remote_signals -- --agents 2 --duration 30 --no-progress
            ;;

        single_write_many_read)
            RUST_LOG=info cargo run --package single_write_many_read -- --duration 5 --no-progress
            ;;
        
        two_party_countersigning)
            RUST_LOG=warn MIN_AGENTS=2 cargo run --package two_party_countersigning -- --agents 2 --behaviour initiate:1 --behaviour participate:1 --duration 30
            ;;
        
        validation_receipts)
            RUST_LOG=warn MIN_AGENTS=2 cargo run --package validation_receipts -- --duration 45 --no-progress
            ;;

        write_query)
            RUST_LOG=info cargo run --package write_query -- --duration 5 --no-progress
            ;;

        write_read)
            RUST_LOG=info cargo run --package write_read -- --duration 5 --no-progress
            ;;
        
        write_validated)
            RUST_LOG=info cargo run --package write_validated -- --duration 5 --no-progress
            ;;

        zome_call_single_value)
            RUST_LOG=info cargo run --package zome_call_single_value -- --duration 5 --no-progress
            ;;

        *)
            echo "Unknown smoke test: $test_name"
            return 1
            ;;
    esac

}
