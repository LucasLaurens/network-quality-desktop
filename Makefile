# Shortcuts only: every command is defined once, in package.json (works on the three OSes).
.PHONY: dev build install test test-back test-front lint fmt ci coverage audit mutants

dev:
	pnpm dev

build:
	pnpm build

install:
	pnpm app:install

test:
	pnpm test

test-back:
	pnpm test:back

test-front:
	pnpm test:front

lint:
	pnpm lint:all

fmt:
	pnpm format

ci:
	pnpm verify

coverage:
	pnpm coverage:back

audit:
	pnpm deps:audit

mutants:
	pnpm mutants:domain
