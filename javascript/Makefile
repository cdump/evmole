DOCKER ?= docker

EXAMPLES = node esbuild parcel parcel_packageExports vite webpack

EXAMPLES_DIR = $(addprefix examples/,$(EXAMPLES))
NODE_MODULES_TARGETS = $(addsuffix /node_modules,$(EXAMPLES_DIR))

%/node_modules: evmole.latest.tgz
	@echo "Installing dependencies in $(@D) $@"
	npm --prefix $(@D) install --no-save
	touch $@

evmole-%.tgz:
	npm run build
	npm pack

evmole.latest.tgz: evmole-%.tgz
	ln -f -s evmole-*.tgz evmole.latest.tgz

test: evmole.latest.tgz $(NODE_MODULES_TARGETS)
	for dir in $(EXAMPLES_DIR); do \
		npm --prefix $$dir install --force --no-save ./evmole.latest.tgz; \
	done

	node examples/node/with_import.mjs

	node examples/node/with_require.cjs

ifeq ($(GITHUB_ACTIONS),true)
	cd tests && npx playwright test
else
	$(DOCKER) run --network=none --rm \
		-v $(shell pwd)/:/mnt \
		-it mcr.microsoft.com/playwright:v1.46.0-jammy \
		/bin/bash -c \
		'cd /mnt/tests && npx playwright test'
endif

clean:
	rm -f *.tgz
	for dir in $(EXAMPLES_DIR); do \
		rm -rf $$dir/node_modules; \
		rm -rf $$dir/dist; \
		rm -rf $$dir/.parcel-cache; \
	done
