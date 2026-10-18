/* A flat image loaded at 0x0800_0000 with _start at its first byte; the stack grows down from the load address.
   The segment symbols fill the homebrew bin's NRO header. */
ENTRY(_start)

SECTIONS
{
    . = 0x08000000;
    __stack_top = .;
    .text : { KEEP(*(.text._start)) *(.text .text.*) }
    .rodata : { __ro = .; *(.rodata .rodata.*) }
    .data : { __data = .; *(.data .data.*) }
    .bss : { __bss = .; *(.bss .bss.* COMMON) }
    __ro_size = __data - __ro;
    __data_size = __bss - __data;
    __bss_size = . - __bss;
    /DISCARD/ : { *(.comment) *(.note .note.*) *(.eh_frame*) }
}
