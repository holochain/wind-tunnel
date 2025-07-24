
bump:
	@if [ "$(ver)x" = "x" ]; then \
		echo "USAGE: make bump ver=0.1.0-alpha.1"; \
		exit 1; \
	fi
	./scripts/bump.sh $(ver)
	cargo build


# Helpers for Domino tests
# start_influx:
# 	influxd

# configure_influx:
# 	configure_influx

# use_influx:
# 	use_influx

# start_telegraf:
# 	start_telegraf

run_hc:
	hc s clean && echo "1234" | hc s --piped create && echo "1234" | RUST_LOG=warn hc s --piped -f 8888 run

current_test:
	RUST_LOG=debug MIN_AGENTS=3 cargo run --package domino -- --connection-string ws://localhost:8888 --agents 3 --behaviour initiate:1 --behaviour spend:2 --duration 30

build:
	cd domino && yarn build:happ
	mkdir -p happs/domino
	cp domino/workdir/domino.happ happs/domino/domino.happ
