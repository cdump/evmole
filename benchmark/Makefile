PROVIDER_BASE = etherscan
PROVIDERS_SELECTORS ?= whatsabi evm-hound-rs sevm evmole-rs evmole-js evmole-py evmole-go
PROVIDERS_ARGUMENTS ?= evmole-rs evmole-js evmole-py evmole-go
PROVIDERS_MUTABILITY ?= whatsabi sevm evmole-rs evmole-js evmole-py evmole-go
PROVIDERS_STORAGE ?= evmole-rs evmole-js evmole-py evmole-go smlxl
PROVIDERS_BLOCKS ?= evmole-rs evmole-js evmole-py evmole-go
PROVIDERS_FLOW ?= evmole-rs evmole-js evmole-py evmole-go evm-cfg ethersolve sevm evm-cfg-builder heimdall-rs

DATASETS ?= largest1k random50k vyper
DATASETS_STORAGE ?= storage3k
DATASETS_FLOW ?= largest1k
DOCKER ?= docker
DOCKER_CPUS ?= 1
DOCKER_PREFIX ?= evmole-bench
RSYNC_EXCLUDES := --exclude benchmark --exclude target --exclude dist \
    --exclude javascript/examples --exclude javascript/tests \
    --exclude javascript/node_modules --exclude .git

PROVIDERS_SELECTORS := $(PROVIDER_BASE) $(PROVIDERS_SELECTORS)
PROVIDERS_ARGUMENTS := $(PROVIDER_BASE) $(PROVIDERS_ARGUMENTS)
PROVIDERS_MUTABILITY := $(PROVIDER_BASE) $(PROVIDERS_MUTABILITY)
PROVIDERS_STORAGE := $(PROVIDER_BASE) $(PROVIDERS_STORAGE)
PROVIDERS_UNIQ := $(sort $(PROVIDERS_SELECTORS) $(PROVIDERS_ARGUMENTS) $(PROVIDERS_MUTABILITY) $(PROVIDERS_STORAGE) $(PROVIDERS_BLOCKS) $(PROVIDERS_FLOW))

DATASET := $(shell pwd)/datasets
RES := $(shell pwd)/results

BUILD_TARGETS := $(addsuffix .build, $(PROVIDERS_UNIQ))
UNPACK_TARGETS := $(foreach d,$(DATASETS) $(DATASETS_STORAGE),$(addprefix datasets/, $(d)))
RUN_SELECTORS_TARGETS := $(foreach p,$(PROVIDERS_SELECTORS),$(addprefix $(p).selectors/, $(DATASETS)))
RUN_ARGUMENTS_TARGETS := $(foreach p,$(PROVIDERS_ARGUMENTS),$(addprefix $(p).arguments/, $(DATASETS)))
RUN_MUTABILITY_TARGETS := $(foreach p,$(PROVIDERS_MUTABILITY),$(addprefix $(p).mutability/, $(DATASETS)))
RUN_STORAGE_TARGETS := $(foreach p,$(PROVIDERS_STORAGE),$(addprefix $(p).storage/, $(DATASETS_STORAGE)))
RUN_BLOCKS_TARGETS := $(foreach p,$(PROVIDERS_BLOCKS),$(addprefix $(p).blocks/, $(DATASETS_FLOW)))
RUN_FLOW_TARGETS := $(foreach p,$(PROVIDERS_FLOW),$(addprefix $(p).flow/, $(DATASETS_FLOW)))

RUN_TARGETS := $(RUN_SELECTORS_TARGETS) $(RUN_ARGUMENTS_TARGETS) $(RUN_MUTABILITY_TARGETS) $(RUN_STORAGE_TARGETS) $(RUN_BLOCKS_TARGETS) $(RUN_FLOW_TARGETS)

benchmark-selectors: $(addsuffix .build, $(PROVIDERS_SELECTORS)) run-selectors
benchmark-arguments: $(addsuffix .build, $(PROVIDERS_ARGUMENTS)) run-arguments
benchmark-mutability: $(addsuffix .build, $(PROVIDERS_MUTABILITY)) run-mutability
benchmark-storage: $(addsuffix .build, $(PROVIDERS_STORAGE)) run-storage
benchmark-flow: $(addsuffix .build, $(PROVIDERS_FLOW)) run-blocks run-flow

build: $(BUILD_TARGETS)
run-selectors: $(RUN_SELECTORS_TARGETS)
run-arguments: $(RUN_ARGUMENTS_TARGETS)
run-mutability: $(RUN_MUTABILITY_TARGETS)
run-storage: $(RUN_STORAGE_TARGETS)
run-blocks: $(RUN_BLOCKS_TARGETS)
run-flow: $(RUN_FLOW_TARGETS)

$(BUILD_TARGETS):
	$(info [*] Building $(basename $@)...)
	@case "$(basename $@)" in \
		evmole-go) \
			rsync -av ../ ./providers/$(basename $@)/rust $(RSYNC_EXCLUDES); \
			rsync -av ../go/ ./providers/evmole-go/evmole --exclude wasm/evmole.wasm.gz;; \
		evmole-*) \
			rsync -av ../ ./providers/$(basename $@)/rust $(RSYNC_EXCLUDES);; \
	esac
	DOCKER_BUILDKIT=1 $(DOCKER) build -t $(DOCKER_PREFIX)-$(basename $@) providers/$(basename $@)
	@case "$(basename $@)" in \
		evmole-go) \
			rm -rf ./providers/$(basename $@)/rust ./providers/evmole-go/evmole;; \
		evmole-*) \
			rm -rf ./providers/$(basename $@)/rust;; \
	esac

$(UNPACK_TARGETS):
	$(info [*] Unpacking $@...)
	tar -C datasets/ -zxf $@.tar.gz

.SECONDEXPANSION:
$(RUN_TARGETS): datasets/$$(notdir $$@)
	$(info [*] Running $@...)
		$(DOCKER) run --init --network=none --cpus=$(DOCKER_CPUS) --rm \
		-v $(DATASET)/$(notdir $@):/dataset \
		-v $(RES):/mnt \
		$(DOCKER_PREFIX)-$(basename $(subst /,,$(dir $@))) \
		$(subst .,,$(suffix $(subst /,,$(dir $@)))) \
		/dataset \
		/mnt/$(subst /,_,$@).json \
		/mnt/$(PROVIDER_BASE).selectors_$(notdir $@).json

.PHONY: benchmark-selectors benchmark-arguments benchmark-mutability build run-selectors run-arguments run-mutability $(BUILD_TARGETS) $(RUN_TARGETS)
