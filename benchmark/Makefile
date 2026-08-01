REFERENCE_PROVIDER = reference
PROVIDERS_SELECTORS ?= lutetia whatsabi evm-hound-rs sevm heimdall-rs evmole-rs evmole-js evmole-py evmole-go
PROVIDERS_ARGUMENTS ?= lutetia heimdall-rs evmole-rs evmole-js evmole-py evmole-go
PROVIDERS_MUTABILITY ?= whatsabi sevm heimdall-rs evmole-rs evmole-js evmole-py evmole-go
PROVIDERS_STORAGE ?= evmole-rs evmole-js evmole-py evmole-go smlxl
PROVIDERS_BLOCKS ?= evmole-rs evmole-js evmole-py evmole-go
PROVIDERS_FLOW ?= evmole-rs evmole-js evmole-py evmole-go evm-cfg ethersolve sevm evm-cfg-builder heimdall-rs

DATASETS ?= solidity-coverage2k solidity-random10k vyper-coverage1k
DATASETS_STORAGE ?= storage-coverage1k
DATASETS_FLOW ?= flow-challenge500
DOCKER ?= docker
DOCKER_CPUS ?= 1
DOCKER_PREFIX ?= evmole-bench
RSYNC_EXCLUDES := --exclude benchmark --exclude target --exclude dist \
    --exclude javascript/examples --exclude javascript/tests \
    --exclude javascript/node_modules --exclude .git

PROVIDERS_SELECTORS := $(REFERENCE_PROVIDER) $(PROVIDERS_SELECTORS)
PROVIDERS_ARGUMENTS := $(REFERENCE_PROVIDER) $(PROVIDERS_ARGUMENTS)
PROVIDERS_MUTABILITY := $(REFERENCE_PROVIDER) $(PROVIDERS_MUTABILITY)
PROVIDERS_STORAGE := $(REFERENCE_PROVIDER) $(PROVIDERS_STORAGE)
PROVIDERS_UNIQ := $(sort $(REFERENCE_PROVIDER) $(PROVIDERS_SELECTORS) $(PROVIDERS_ARGUMENTS) $(PROVIDERS_MUTABILITY) $(PROVIDERS_STORAGE) $(PROVIDERS_BLOCKS) $(PROVIDERS_FLOW))

DATASET := $(shell pwd)/datasets
RES := $(shell pwd)/results

BUILD_TARGETS := $(addsuffix .build, $(PROVIDERS_UNIQ))
UNPACK_TARGETS := $(foreach d,$(sort $(DATASETS) $(DATASETS_STORAGE) $(DATASETS_FLOW)),$(addprefix datasets/, $(d)))
RUN_SELECTORS_TARGETS := $(foreach p,$(PROVIDERS_SELECTORS),$(addprefix $(p).selectors/, $(DATASETS)))
RUN_ARGUMENTS_TARGETS := $(foreach p,$(PROVIDERS_ARGUMENTS),$(addprefix $(p).arguments/, $(DATASETS)))
RUN_MUTABILITY_TARGETS := $(foreach p,$(PROVIDERS_MUTABILITY),$(addprefix $(p).mutability/, $(DATASETS)))
RUN_STORAGE_TARGETS := $(foreach p,$(PROVIDERS_STORAGE),$(addprefix $(p).storage/, $(DATASETS_STORAGE)))
RUN_BLOCKS_TARGETS := $(foreach p,$(PROVIDERS_BLOCKS),$(addprefix $(p).blocks/, $(DATASETS_FLOW)))
RUN_FLOW_TARGETS := $(foreach p,$(PROVIDERS_FLOW),$(addprefix $(p).flow/, $(DATASETS_FLOW)))

RUN_TARGETS := $(RUN_SELECTORS_TARGETS) $(RUN_ARGUMENTS_TARGETS) $(RUN_MUTABILITY_TARGETS) $(RUN_STORAGE_TARGETS) $(RUN_BLOCKS_TARGETS) $(RUN_FLOW_TARGETS)
REFERENCE_SELECTOR_TARGETS := $(addprefix $(REFERENCE_PROVIDER).selectors/,$(DATASETS))
ALL_RUN_TARGETS := $(sort $(RUN_TARGETS) $(REFERENCE_SELECTOR_TARGETS))

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
$(RUN_ARGUMENTS_TARGETS) $(RUN_MUTABILITY_TARGETS): $$(RES)/$(REFERENCE_PROVIDER).selectors_$$(notdir $$@).json

$(RES)/$(REFERENCE_PROVIDER).selectors_%.json: datasets/% $(REFERENCE_PROVIDER).build
	$(MAKE) $(REFERENCE_PROVIDER).selectors/$*

$(ALL_RUN_TARGETS): datasets/$$(notdir $$@)
	$(info [*] Running $@...)
		$(DOCKER) run --init --network=none --cpus=$(DOCKER_CPUS) --rm \
		-v $(DATASET)/$(notdir $@):/dataset \
		-v $(RES):/mnt \
		$(DOCKER_PREFIX)-$(basename $(subst /,,$(dir $@))) \
		$(subst .,,$(suffix $(subst /,,$(dir $@)))) \
		/dataset \
		/mnt/$(subst /,_,$@).json \
		/mnt/$(REFERENCE_PROVIDER).selectors_$(notdir $@).json

.PHONY: benchmark-selectors benchmark-arguments benchmark-mutability benchmark-storage benchmark-flow \
	build run-selectors run-arguments run-mutability run-storage run-blocks run-flow \
	$(BUILD_TARGETS) $(ALL_RUN_TARGETS)
