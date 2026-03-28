.PHONY: all run setup start stop serve help

all run: setup start

setup:
	./setup.sh --no-start

start:
	./start-rex.sh $(ARGS)

stop:
	uv run rex stop

serve:
	uv run python -m hypercorn app.main:app --bind 0.0.0.0:8000

help:
	@echo "make, make all, make run  deps + TLS certs, then start Rex (HTTPS via start-rex.sh)"
	@echo "make setup                ./setup.sh --no-start only"
	@echo "make start                ./start-rex.sh only; pass ARGS= e.g. ARGS=--http"
	@echo "make stop                 uv run rex stop (instance started via rex start / start-rex.sh)"
	@echo "make serve                uv run hypercorn in foreground (same ASGI server as rex start; logs to terminal)"
