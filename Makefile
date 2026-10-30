# Build and run the Raspberry Pi 3 BSP. See README.md.
#
# The images need the QNX SDP 8.0 environment: source qnxsdp-env.sh from
# the SDP first, which sets QNX_HOST and QNX_TARGET and puts mkifs on the
# PATH. The startup, the unit tests and the books do not.

BUILD := build

CLANG ?= clang
LLD   ?= ld.lld

QEMU_DIR := $(BUILD)/qemu
BSP_DIR  := $(BUILD)/bsp

# The Rust startup: cargo profile "release" (default) or "dev".
TARGET      := aarch64-unknown-none-softfloat
PROFILE     ?= release
PROFILE_DIR := $(if $(filter dev,$(PROFILE)),debug,$(PROFILE))
STARTUP_LIB := target/$(TARGET)/$(PROFILE_DIR)/libstartup_bcm2837_rpi3.a
STARTUP_DIR := $(BUILD)/startup
STARTUP     := $(STARTUP_DIR)/startup-bcm2837-rpi3
IMAGE       := $(BUILD)/rpi3.ifs

# The startup library, oscraft-startup, is a git dependency of the board
# crate. OSCRAFT_STARTUP=<dir> uses a local checkout of it instead, for
# working on both at once. (Cargo then records it in Cargo.lock: build
# once without it before committing the lock file.)
ifneq ($(OSCRAFT_STARTUP),)
CARGO_FLAGS += --config 'patch."https://github.com/capitan-davide/oscraft-startup".oscraft-startup.path="$(abspath $(OSCRAFT_STARTUP))"'
endif
# The library's directory, wherever cargo has it: for its linker script and
# its book.
STARTUP_SRC = $(shell cargo metadata $(CARGO_FLAGS) --format-version 1 | python3 -I -c 'import json, sys; print(next(p["manifest_path"] for p in json.load(sys.stdin)["packages"] if p["name"] == "oscraft-startup").rsplit("/", 1)[0])')

# The first command of a recipe that needs the SDP.
need_sdp = @test -n "$(QNX_TARGET)" || { echo "QNX_TARGET is not set: source qnxsdp-env.sh from the QNX SDP 8.0 first" >&2; exit 1; }

# QNX's RPi4 BSP, as installed by the QNX Software Center into the SDP's
# bsp/ directory (the newest one, if there are several). QNX has no BSP for
# the Raspberry Pi 3, but the BCM2837 has the BCM2711's mini-UART: the RPi4
# BSP's prebuilt driver goes into the image.
QNX_BSP_ZIP ?= $(lastword $(sort $(wildcard $(QNX_TARGET)/../../bsp/BSP_raspberrypi-bcm2711-rpi4_*.zip)))

# Options for startup-bcm2837-rpi3 in images/rpi3.build: -u reg takes the
# DTB address from x0, which is what the firmware (and fwstub) passes.
STARTUP_OPTS ?= -v -D miniuart -u reg

# mkifs's search path for the image's files: $(1), then the BSP's
# prebuilt binaries, then the SDP's. The first directory that has a file
# wins.
SDP_PATH := $(addprefix $(QNX_TARGET)/aarch64le/,boot/sys sbin usr/sbin bin usr/bin lib usr/lib)
BSP_PATH := $(addprefix $(abspath $(BSP_DIR))/prebuilt/aarch64le/,boot/sys sbin)
empty :=
space := $(empty) $(empty)
define mkifs
	$(need_sdp)
	@mkdir -p $(@D)
	sed 's|$${STARTUP_OPTS}|$(STARTUP_OPTS)|' $< > $@.build
	MKIFS_PATH=$(subst $(space),:,$(strip $(1) $(BSP_PATH) $(SDP_PATH))) mkifs $@.build $@
endef

.PHONY: all startup image run test unit-test qemu-test book book-check book-serve qemu bsp clean

all: image qemu

# ANCHOR: make_startup
# The startup program: the board crate as a static library, partially
# linked (ld -r) with the library's linker script into the one
# relocatable object mkifs links into the image. -u _start pulls in the
# entry point and, from there, everything it uses.
startup:
	cargo build $(CARGO_FLAGS) -p startup-bcm2837-rpi3 --target $(TARGET) --profile $(PROFILE)
	@mkdir -p $(STARTUP_DIR)
	$(LLD) -r -u _start -T $(STARTUP_SRC)/startup.ld -o $(STARTUP) $(STARTUP_LIB)
# ANCHOR_END: make_startup

# The image, with our startup: build/startup comes first in mkifs's path.
image: $(IMAGE)

$(IMAGE): images/rpi3.build startup $(BSP_DIR)/.unpacked
	$(call mkifs,$(abspath $(STARTUP_DIR)))

run: $(IMAGE) qemu
	tools/qemu/run.sh $< -no-reboot

# ANCHOR: make_test
# All tests: the unit tests on the host, then the image booted in QEMU.
test: unit-test qemu-test

# The board crate's unit tests, on the host. (The library has its own, in
# its repository.)
unit-test:
	cargo test $(CARGO_FLAGS) -p startup-bcm2837-rpi3

# Boot the image in QEMU and check QNX comes up on all 4 CPUs.
qemu-test: $(IMAGE) qemu
	python3 -I tools/qemu/test.py $(IMAGE) 4
# ANCHOR_END: make_test

# The two books, built side by side into build/book/: the BSP's
# (docs/book), and the startup library's, from the library's source,
# which the BSP's book links to.
book: book-check
	mdbook build docs/book
	mdbook build $(STARTUP_SRC)/docs/book -d $(abspath $(BUILD))/book/startup
	cp docs/book/index.html $(BUILD)/book/index.html

book-check:
	OTHER_BOOK_SRC=$(STARTUP_SRC)/docs/book/src python3 -I docs/book/check.py

book-serve: book
	python3 -I -m http.server --directory $(BUILD)/book 3001

# QEMU support files: firmware stub and device tree.
qemu: $(QEMU_DIR)/fwstub.elf $(QEMU_DIR)/bcm2710-rpi-3-b.dtb

$(QEMU_DIR)/fwstub.o: tools/qemu/fwstub/fwstub.S
	@mkdir -p $(@D)
	$(CLANG) --target=aarch64-none-elf -c $< -o $@

$(QEMU_DIR)/fwstub.elf: $(QEMU_DIR)/fwstub.o tools/qemu/fwstub/fwstub.ld
	$(LLD) -T tools/qemu/fwstub/fwstub.ld $< -o $@

$(QEMU_DIR)/bcm2710-rpi-3-b.dtb: tools/rpi3/fetch-firmware.sh
	tools/rpi3/fetch-firmware.sh $(@F) $@

# QNX's prebuilt RPi4 BSP binaries, unpacked from the BSP zip.
bsp: $(BSP_DIR)/.unpacked

$(BSP_DIR)/.unpacked: $(QNX_BSP_ZIP)
	$(need_sdp)
	@test -n "$<" || { echo "QNX RPi4 BSP zip not found: install the BSP with the QNX Software Center, or set QNX_BSP_ZIP" >&2; exit 1; }
	@mkdir -p $(@D)
	unzip -oq $< 'prebuilt/*' 'binary_files_with_symbols/*' -d $(BSP_DIR)
	touch $@

clean:
	rm -rf $(BUILD)
