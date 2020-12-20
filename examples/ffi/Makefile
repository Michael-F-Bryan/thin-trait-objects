REQUIRED_LIBRARIES=pthread dl
LIBS=$(patsubst %,-l%,$(REQUIRED_LIBRARIES))
PACKAGE=thin_trait_objects
CFLAGS=-std=c11 -g -I. -O3
CRATE_ROOT=../..
RUST_FILES=$(shell find $(CRATE_ROOT) -name '*.rs')

example: main.c libthin_trait_objects.a thin_trait_objects.h
	$(CC) $(CFLAGS) -o $@ $< libthin_trait_objects.a $(LIBS)

thin_trait_objects.h: $(RUST_FILES)
	cbindgen --lang c $(CRATE_ROOT) -o $@

libthin_trait_objects.a: $(RUST_FILES)
	cargo build --manifest-path "$(CRATE_ROOT)/Cargo.toml" && cp "$(CRATE_ROOT)/target/debug/libthin_trait_objects.a" ./$@

clean:
	$(RM) example libthin_trait_objects.a *.txt thin_trait_objects.h

.PHONY: clean
