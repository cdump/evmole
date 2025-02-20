PROVIDER_BASE = etherscan
PROVIDERS_SELECTORS ?= simple whatsabi evm-hound-rs sevm evmole-rs evmole-js evmole-py
PROVIDERS_ARGUMENTS ?= simple evmole-rs evmole-js evmole-py
PROVIDERS_MUTABILITY ?= simple whatsabi sevm evmole-rs evmole-js evmole-py
PROVIDERS_STORAGE ?= evmole-rs smlxl
PROVIDERS_FLOW ?= evmole-rs evm-cfg ethersolve sevm evm-cfg-builder heimdall-rs

DATASETS ?= largest1k random50k vyper
DATASETS_STORAGE ?= storage3k
DOCKER ?= docker
DOCKER_CPUS ?= 1
DOCKER_PREFIX ?= evmole-bench

PROVIDERS_SELECTORS := $(PROVIDER_BASE) $(PROVIDERS_SELECTORS)
PROVIDERS_ARGUMENTS := $(PROVIDER_BASE) $(PROVIDERS_ARGUMENTS)
PROVIDERS_MUTABILITY := $(PROVIDER_BASE) $(PROVIDERS_MUTABILITY)
PROVIDERS_STORAGE := $(PROVIDER_BASE) $(PROVIDERS_STORAGE)
PROVIDERS_UNIQ := $(sort $(PROVIDERS_SELECTORS) $(PROVIDERS_ARGUMENTS) $(PROVIDERS_MUTABILITY) $(PROVIDERS_STORAGE) $(PROVIDERS_FLOW))

DATASET := $(shell pwd)/datasets
RES := $(shell pwd)/results

BUILD_TARGETS := $(addsuffix .build, $(PROVIDERS_UNIQ))
UNPACK_TARGETS := $(foreach d,$(DATASETS) $(DATASETS_STORAGE),$(addprefix datasets/, $(d)))
RUN_SELECTORS_TARGETS := $(foreach p,$(PROVIDERS_SELECTORS),$(addprefix $(p).selectors/, $(DATASETS)))
RUN_ARGUMENTS_TARGETS := $(foreach p,$(PROVIDERS_ARGUMENTS),$(addprefix $(p).arguments/, $(DATASETS)))
RUN_MUTABILITY_TARGETS := $(foreach p,$(PROVIDERS_MUTABILITY),$(addprefix $(p).mutability/, $(DATASETS)))
RUN_STORAGE_TARGETS := $(foreach p,$(PROVIDERS_STORAGE),$(addprefix $(p).storage/, $(DATASETS_STORAGE)))
RUN_FLOW_TARGETS := $(foreach p,$(PROVIDERS_FLOW),$(addprefix $(p).flow/, $(DATASETS)))

RUN_TARGETS := $(RUN_SELECTORS_TARGETS) $(RUN_ARGUMENTS_TARGETS) $(RUN_MUTABILITY_TARGETS) $(RUN_STORAGE_TARGETS) $(RUN_FLOW_TARGETS)

benchmark-selectors: $(addsuffix .build, $(PROVIDERS_SELECTORS)) run-selectors
benchmark-arguments: $(addsuffix .build, $(PROVIDERS_ARGUMENTS)) run-arguments
benchmark-mutability: $(addsuffix .build, $(PROVIDERS_MUTABILITY)) run-mutability
benchmark-storage: $(addsuffix .build, $(PROVIDERS_STORAGE)) run-storage
benchmark-flow: $(addsuffix .build, $(PROVIDERS_FLOW)) run-flow

build: $(BUILD_TARGETS)
run-selectors: $(RUN_SELECTORS_TARGETS)
run-arguments: $(RUN_ARGUMENTS_TARGETS)
run-mutability: $(RUN_MUTABILITY_TARGETS)
run-storage: $(RUN_STORAGE_TARGETS)
run-flow: $(RUN_FLOW_TARGETS)

$(BUILD_TARGETS):
	$(info [*] Building $(basename $@)...)
	# for evmole - use latest version, from current git branch
	if [ "$@" = "evmole-py.build" ] || [ "$@" = "evmole-js.build" ] || [ "$@" = "evmole-rs.build" ]; then \
		rsync -av ../ ./providers/$(basename $@)/rust --exclude benchmark --exclude target --exclude dist --exclude javascript/examples --exclude javascript/tests --exclude javascript/node_modules --exclude .git; \
	fi;
	DOCKER_BUILDKIT=1 $(DOCKER) build -t $(DOCKER_PREFIX)-$(basename $@) providers/$(basename $@)
	if [ "$@" = "evmole-py.build" ] || [ "$@" = "evmole-js.build" ] || [ "$@" = "evmole-rs.build" ]; then \
		rm -rf ./providers/$(basename $@)/rust; \
	fi;

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

.PHONY: benchmark-selectors benchmark-arguments benchmark-mutability build run-selectors run-arguments run-mutability $(BUILD_TARGETS) $(RUN_TARGETS)
