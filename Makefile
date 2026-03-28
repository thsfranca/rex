.PHONY: all run setup start stop serve help

all run: setup start

setup:
	./setup.sh

start:
	./start-rex.sh $(ARGS)

stop:
	uv run rex stop

serve:
	uv run python -m uvicorn app.main:app --host 127.0.0.1 --port 8000

help:
	@echo "make, make all, make run  deps, then start Rex (HTTP via start-rex.sh)"
	@echo "make setup                ./setup.sh only (deps; does not start Rex)"
	@echo "make start                ./start-rex.sh only; pass ARGS= e.g. ARGS='--port 9000'"
	@echo "make stop                 uv run rex stop (instance started via rex start / start-rex.sh)"
	@echo "make serve                uvicorn on 127.0.0.1:8000 (foreground, logs to terminal)"
