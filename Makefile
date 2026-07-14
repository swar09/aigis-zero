.PHONY: check fix clean clean-deep seed seed-reset ci setup precommit

check:
	@./scripts/check.sh

fix:
	@./scripts/check.sh --fix

clean:
	@./scripts/clean.sh

clean-deep:
	@./scripts/clean.sh --deep

seed:
	@./scripts/seed.sh

seed-reset:
	@./scripts/seed.sh --reset

ci:
	@./scripts/ci.sh

setup:
	@./scripts/setup.sh

precommit:
	@./scripts/precommit.sh
