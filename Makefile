.PHONY: all run setup start stop serve serve-http help

REX_TLS_DIR := $(HOME)/.rex/tls
REX_CERT := $(REX_TLS_DIR)/localhost.pem
REX_KEY := $(REX_TLS_DIR)/localhost-key.pem

all run: setup start

setup:
	./setup.sh --no-start

start:
	./start-rex.sh $(ARGS)

stop:
	uv run rex stop

serve:
	test -f "$(REX_CERT)" && test -f "$(REX_KEY)" || (echo >&2 "make serve: missing TLS files under ~/.rex/tls/ — run: make setup"; exit 1)
	uv run python -m hypercorn app.main:app --bind 127.0.0.1:8000 --certfile "$(REX_CERT)" --keyfile "$(REX_KEY)"

serve-http:
	uv run python -m hypercorn app.main:app --bind 0.0.0.0:8000

help:
	@echo "make, make all, make run  deps + TLS certs, then start Rex (HTTPS via start-rex.sh)"
	@echo "make setup                ./setup.sh --no-start only"
	@echo "make start                ./start-rex.sh only; pass ARGS= e.g. ARGS=--http"
	@echo "make stop                 uv run rex stop (instance started via rex start / start-rex.sh)"
	@echo "make serve                HTTPS Hypercorn on 127.0.0.1:8000 (~/.rex/tls certs; HTTP/2 via TLS; logs to terminal)"
	@echo "make serve-http           cleartext Hypercorn on 0.0.0.0:8000 (foreground)"
