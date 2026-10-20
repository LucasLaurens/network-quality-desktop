# Shortcuts only: every command is defined once, in package.json (works on the three OSes).
.PHONY: dev build install test test-back test-front lint fmt ci

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
	pnpm lint && pnpm lint:back

fmt:
	pnpm format

ci:
	pnpm verify
