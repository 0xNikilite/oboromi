/* A flat image loaded at 0x0800_0000 with _start at its first byte; the stack grows down from the load address. */
ENTRY(_start)

SECTIONS
{
    . = 0x08000000;
    __stack_top = .;
    .text : { KEEP(*(.text._start)) *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data : { *(.data .data.*) }
    .bss : { *(.bss .bss.* COMMON) }
    /DISCARD/ : { *(.comment) *(.note .note.*) *(.eh_frame*) }
}
