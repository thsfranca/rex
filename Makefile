ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
WEB_DIR := $(ROOT)/apps/rex-web

.PHONY: help run build build-ui build-rex

.DEFAULT_GOAL := help

help:
	@echo "Rex developer targets (macOS desktop):"
	@echo "  make run   — rebuild web UI + rex, then launch desktop"
	@echo "  make build — rebuild web UI + rex only"
	@echo ""
	@echo "Optional session flags: make run ARGS='--debug'"

build-ui:
	cd "$(WEB_DIR)" && npm ci && npm run build

build-rex:
	cargo build -p rex --locked

build: build-ui build-rex

run: build
	@if [ "$$(uname -s)" != "Darwin" ]; then \
		echo "Error: Rex desktop requires macOS." >&2; \
		exit 1; \
	fi
	cargo run -p rex --locked -- $(ARGS)
