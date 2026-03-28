.PHONY: all run setup start help

all run: setup start

setup:
	./setup.sh --no-start

start:
	./start-rex.sh $(ARGS)

help:
	@echo "make, make all, make run  deps + TLS certs, then start Rex (HTTPS via start-rex.sh)"
	@echo "make setup                ./setup.sh --no-start only"
	@echo "make start                ./start-rex.sh only; pass ARGS= e.g. ARGS=--http"
