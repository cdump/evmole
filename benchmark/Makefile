PROVIDERS ?= etherscan simple whatsabi evmole-py evmole-js
DATASETS ?= largest1k random50k vyper
DOCKER ?= docker
DOCKER_CPUS ?= 1
DOCKER_PREFIX ?= evmole-bench

DATASET=$(shell pwd)/datasets
RES=$(shell pwd)/results

BUILD_TARGETS=$(addsuffix .build, $(PROVIDERS))
RUN_TARGETS=$(foreach p,$(PROVIDERS),$(addprefix $(p)/, $(DATASETS)))
UNPACK_TARGETS=$(foreach d,$(DATASETS),$(addprefix datasets/, $(d)))

benchmark: build run

build: $(BUILD_TARGETS)

run: $(RUN_TARGETS)

$(BUILD_TARGETS):
	$(info [*] Building $(basename $@)...)
	# special hack for evmole:
	[ "$@" = "evmole-py.build" ] && cp -r ../evmole providers/evmole-py/ || true
	[ "$@" = "evmole-js.build" ] && cp -r ../js providers/evmole-js/ || true
	$(DOCKER) build -t $(DOCKER_PREFIX)-$(basename $@) providers/$(basename $@)
	[ "$@" = "evmole-py.build" ] && rm -rf providers/evmole-py/evmole || true
	[ "$@" = "evmole-js.build" ] && rm -rf providers/evmole-js/js || true

$(UNPACK_TARGETS):
	$(info [*] Unpacking $@...)
	tar -C datasets/ -zxf $@.tar.gz

.SECONDEXPANSION:
$(RUN_TARGETS): datasets/$$(notdir $$@)
	$(info [*] Running $@...)
	/bin/time -f '%e' $(DOCKER) run --cpus=$(DOCKER_CPUS) --rm -v $(DATASET)/$(notdir $@):/dataset -v $(RES):/mnt -it $(DOCKER_PREFIX)-$(subst /,,$(dir $@)) /dataset /mnt/$(subst /,_,$@).json 2> $(RES)/$(subst /,_,$@).time

.PHONY: benchmark build run $(BUILD_TARGETS) $(RUN_TARGETS)
