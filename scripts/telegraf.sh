
start_telegraf() {
    if [[ -z "$INFLUX_TOKEN" ]]; then
        echo "INFLUX_TOKEN is not set, please run \`use_influx\` first"
        return 1
    fi

    telegraf --config "$(pwd)/telegraf/telegraf.conf"
}
