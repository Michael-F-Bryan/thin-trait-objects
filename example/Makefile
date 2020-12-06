REQUIRED_LIBRARIES=pthread dl
LIBS=$(patsubst %,-l%,$(REQUIRED_LIBRARIES))
RUST_FILES=$(wildcard ../src/**/*.rs)

example: main.c libthin_trait_objects.a thin_trait_objects.h
	$(CC) -I. -o $@ $< libthin_trait_objects.a $(LIBS)

thin_trait_objects.h: $(RUST_FILES)
	cd .. && cbindgen --lang c > example/$@

libthin_trait_objects.a: $(RUST_FILES)
	cd .. && cargo build && cp target/debug/libthin_trait_objects.a example

clean:
	$(RM) example libthin_trait_objects.a *.txt

.PHONY: clean
