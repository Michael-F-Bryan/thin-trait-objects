REQUIRED_LIBRARIES=pthread dl
LIBS=$(patsubst %,-l%,$(REQUIRED_LIBRARIES))
PACKAGE=thin_trait_objects
RUST_FILES=$(shell find .. -name '*.rs')
CFLAGS=-std=c11 -g -I. -O3

example: main.c libthin_trait_objects.a thin_trait_objects.h
	$(CC) $(CFLAGS) -o $@ $< libthin_trait_objects.a $(LIBS)

thin_trait_objects.h: $(RUST_FILES)
	cd .. && cbindgen --lang c > example/$@

libthin_trait_objects.a: $(RUST_FILES)
	cd .. && cargo build && cp target/debug/libthin_trait_objects.a example/$@

clean:
	$(RM) example libthin_trait_objects.a *.txt thin_trait_objects.h

.PHONY: clean
