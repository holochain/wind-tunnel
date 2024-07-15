
bump:
	@if [ "$(ver)x" = "x" ]; then \
		echo "USAGE: make bump ver=0.1.0-alpha.1"; \
		exit 1; \
	fi
	./scripts/bump.sh $(ver)
	cargo build


