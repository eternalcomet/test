TARGET_DIR = build/$(ARCH)

all: build

prepare:
	mkdir -p $(TARGET_DIR)

build: prepare
	cp -r $(ARCH)/* $(TARGET_DIR)

clean:
	rm -f $(TARGET_DIR)

.PHONY: all prepare build clean