PROVIDER_BASE = etherscan
PROVIDERS_SELECTORS ?= simple whatsabi evm-hound-rs evmole-rs evmole-js evmole-py
PROVIDERS_ARGUMENTS ?= simple evmole-rs evmole-js evmole-py

DATASETS ?= largest1k random50k vyper
DOCKER ?= docker
DOCKER_CPUS ?= 1
DOCKER_PREFIX ?= evmole-bench

PROVIDERS_SELECTORS := $(PROVIDER_BASE) $(PROVIDERS_SELECTORS)
PROVIDERS_ARGUMENTS := $(PROVIDER_BASE) $(PROVIDERS_ARGUMENTS)
PROVIDERS_UNIQ := $(sort $(PROVIDERS_SELECTORS) $(PROVIDERS_ARGUMENTS))

DATASET := $(shell pwd)/datasets
RES := $(shell pwd)/results

BUILD_TARGETS := $(addsuffix .build, $(PROVIDERS_UNIQ))
UNPACK_TARGETS := $(foreach d,$(DATASETS),$(addprefix datasets/, $(d)))
RUN_SELECTORS_TARGETS := $(foreach p,$(PROVIDERS_SELECTORS),$(addprefix $(p).selectors/, $(DATASETS)))
RUN_ARGUMENTS_TARGETS := $(foreach p,$(PROVIDERS_ARGUMENTS),$(addprefix $(p).arguments/, $(DATASETS)))

RUN_TARGETS := $(RUN_SELECTORS_TARGETS) $(RUN_ARGUMENTS_TARGETS)

benchmark-selectors: build run-selectors
benchmark-arguments: build run-arguments

build: $(BUILD_TARGETS)
run-selectors: $(RUN_SELECTORS_TARGETS)
run-arguments: $(RUN_ARGUMENTS_TARGETS)

$(BUILD_TARGETS):
	$(info [*] Building $(basename $@)...)
	# for evmole - use latest version, from current git branch
	[ "$@" = "evmole-py.build" ] && cp -r ../evmole providers/evmole-py/ || true
	[ "$@" = "evmole-js.build" ] && cp -r ../js providers/evmole-js/ || true
	[ "$@" = "evmole-rs.build" ] && cp -r ../rust providers/evmole-rs/ || true
	DOCKER_BUILDKIT=1 $(DOCKER) build -t $(DOCKER_PREFIX)-$(basename $@) providers/$(basename $@)
	[ "$@" = "evmole-py.build" ] && rm -rf providers/evmole-py/evmole || true
	[ "$@" = "evmole-js.build" ] && rm -rf providers/evmole-js/js || true
	[ "$@" = "evmole-rs.build" ] && rm -rf providers/evmole-rs/rust || true

$(UNPACK_TARGETS):
	$(info [*] Unpacking $@...)
	tar -C datasets/ -zxf $@.tar.gz

.SECONDEXPANSION:
$(RUN_TARGETS): datasets/$$(notdir $$@)
	$(info [*] Running $@...)
	/bin/time -f '%e' \
		$(DOCKER) run --network=none --cpus=$(DOCKER_CPUS) --rm \
		-v $(DATASET)/$(notdir $@):/dataset \
		-v $(RES):/mnt \
		-it $(DOCKER_PREFIX)-$(basename $(subst /,,$(dir $@))) \
		$(subst .,,$(suffix $(subst /,,$(dir $@)))) \
		/dataset \
		/mnt/$(subst /,_,$@).json \
		/mnt/$(PROVIDER_BASE).selectors_$(notdir $@).json \
		2> $(RES)/$(subst /,_,$@).time

.PHONY: benchmark-selectors benchmark-arguments build run-selectors run-arguments $(BUILD_TARGETS) $(RUN_TARGETS)
