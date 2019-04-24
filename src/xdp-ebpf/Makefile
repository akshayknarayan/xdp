OBJS = xdp-bpf.o

LLC ?= llc
CLANG ?= clang
INC_FLAGS = -nostdinc -isystem `$(CLANG) -print-file-name=include`
EXTRA_CFLAGS ?= -O2 -emit-llvm

# In case up-to-date headers are not installed locally in /usr/include,
# use source build.

linuxhdrs ?= /home/akshayn/linux

LINUXINCLUDE =  -I $(linuxhdrs)/arch/x86/include/uapi \
				-I $(linuxhdrs)/include/uapi \
				-I $(linuxhdrs)/include \

prefix ?= /usr/local

INSTALLPATH = $(prefix)/lib/bpf

install_PROGRAM = install
install_DIR = install -dv

all: $(OBJS)

.PHONY: clean

clean:
	rm -f $(OBJS)

INC_FLAGS = -nostdinc -isystem `$(CLANG) -print-file-name=include`

$(OBJS):  %.o:%.c
	$(CLANG) $(INC_FLAGS) \
                -D__KERNEL__ -D__ASM_SYSREG_H \
                -Wno-unused-value -Wno-pointer-sign \
                -Wno-compare-distinct-pointer-types \
                -Wno-gnu-variable-sized-type-not-at-end \
                -Wno-address-of-packed-member -Wno-tautological-compare \
                -Wno-unknown-warning-option \
                -Iinclude -I include/uapi $(LINUXINCLUDE) \
                $(EXTRA_CFLAGS) -c $< -o -| $(LLC) -march=bpf -filetype=obj -o $@

install: $(OBJS)
	$(install_DIR) -d $(INSTALLPATH) ; \
	$(install_PROGRAM) $^ -t $(INSTALLPATH)

uninstall: $(OBJS)
	rm -rf $(INSTALLPATH)
