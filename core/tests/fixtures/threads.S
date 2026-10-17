// Rebuild: llvm-mc -triple=aarch64 -filetype=obj threads.S -o threads.o
//          llvm-objcopy -O binary --only-section=.text threads.o threads.bin
.text
.org 0
// Two copies alternate a protected counter ten thousand times each.
    mov x19, #0x40000000
    mov x20, x0
    mov x21, x1
    mov x22, #10000
lock:
    ldxr w9, [x19]
    cbnz w9, contended
    stxr w10, w21, [x19]
    cbnz w10, lock
    b acquired
contended:
    orr w10, w9, #0x40000000
    stxr w11, w10, [x19]
    cbnz w11, lock
    and w0, w9, #0xbfffffff
    mov x1, x19
    mov x2, x21
    svc #0x1a
    cbnz w0, fail
    ldr w9, [x19]
    and w9, w9, #0xbfffffff
    cmp w9, w21
    b.ne lock
acquired:
    ldr w9, [x19, #8]
    cmp w9, w20
    b.eq increment
    mov x0, x19
    add x1, x19, #4
    mov x2, x21
    mov x3, #-1
    svc #0x1c
    cbnz w0, fail
    b acquired
increment:
    ldr w9, [x19, #12]
    add w9, w9, #1
    str w9, [x19, #12]
    eor w9, w20, #1
    str w9, [x19, #8]
    add x0, x19, #4
    mov x1, #-1
    svc #0x1d
    cbnz w0, fail
    mov x0, x19
    svc #0x1b
    cbnz w0, fail
    subs x22, x22, #1
    b.ne lock
    svc #0x0a
fail:
    brk #1

.org 0x400
    mov x19, #0x40000000
    mov w9, #5
    str w9, [x19]
    ldxr w20, [x19]
    mov x0, #0
    svc #0x0b
    ldr w9, [x19, #8]
    str w9, [x19, #12]
    stxr w21, w20, [x19]
    str w21, [x19, #4]
    svc #0x0a
.org 0x480
    mov x19, #0x40000000
    mov w9, #6
    str w9, [x19]
    mov w9, #5
    str w9, [x19]
    mov w9, #1
    str w9, [x19, #8]
    svc #0x0a

.org 0x600
    mov x19, #0x40000000
    mov x1, x19
    mov x2, #1
    mov x3, #10000
    svc #0x18
    str w0, [x19, #8]
    str w1, [x19, #12]
    ldr w0, [x19]
    svc #0x17
    str w0, [x19, #16]
    mov x1, x19
    mov x2, #1
    mov x3, #0
    svc #0x18
    str w0, [x19, #20]
    svc #0x0a

.org 0x800
    mov x19, #0x40000000
    str w1, [x19]
    ldr w0, [x19, #4]
    svc #0x09
    ldr w0, [x19, #8]
    svc #0x09
    mov x1, #-32768
    svc #0x0c
    str w1, [x19, #12]
    mov w9, #1
    str w9, [x19, #16]
    mov x0, x19
    svc #0x1b
    mov x1, #-32768
    svc #0x0c
    str w1, [x19, #20]
    svc #0x0a
.org 0x900
    mov x19, #0x40000000
    mov x2, x1
    ldr w0, [x19]
    orr w9, w0, #0x40000000
    str w9, [x19]
    mov x1, x19
    svc #0x1a
    cbnz w0, fail
    mov x0, x19
    svc #0x1b
    svc #0x0a
.org 0xa00
    mov x19, #0x40000000
    ldr w9, [x19, #16]
    str w9, [x19, #24]
    svc #0x0a

.org 0xc00
    mov x0, #0x40000000
    add x0, x0, #0x2000
    mov sp, x0
    mov x0, #0x1000000
    msr fpcr, x0
    mov x0, #0x8000000
    msr fpsr, x0
    mov x0, #0xa0000000
    msr nzcv, x0
    mov x0, #0x3c0
    msr daif, x0
    mov x0, #0x1234
    msr tpidr_el0, x0
.irp n,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31
    movi v\n\().16b, #\n
.endr
.irp n,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30
    mov x\n, #(0x100 + \n)
.endr
    stp x0, x1, [sp, #-16]!
    mov x0, #0x40000000
sentinel_wait:
    ldr w1, [x0, #0x1000]
    cbz w1, sentinel_wait
    ldp x0, x1, [sp], #16
.irp n,0,2,4,6,8,10,12,14,16,18,20,22,24,26,28
    str x\n, [sp, #(\n * 8)]
.endr
.irp n,1,3,5,7,9,11,13,15,17,19,21,23,25,27,29,30
    str x\n, [sp, #(\n * 8)]
.endr
    mov x0, sp
    str x0, [sp, #248]
    add x0, x0, #256
.irp n,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31
    str q\n, [x0, #(\n * 16)]
.endr
    mrs x1, nzcv
    str x1, [x0, #512]
    mrs x1, fpcr
    str x1, [x0, #520]
    mrs x1, fpsr
    str x1, [x0, #528]
    mrs x1, tpidr_el0
    str x1, [x0, #536]
    mrs x1, tpidrro_el0
    str x1, [x0, #544]
    mrs x1, cntfrq_el0
    str x1, [x0, #552]
    mrs x1, cntpct_el0
    str x1, [x0, #560]
    mrs x1, mpidr_el1
    str x1, [x0, #568]
    mrs x1, daif
    str x1, [x0, #592]
    mrs x1, currentel
    str x1, [x0, #600]
    mrs x1, spsel
    str x1, [x0, #608]
    svc #0x10
    mov x19, #0x40000000
    str x0, [x19, #0x2340]
    svc #0x1e
    str x0, [x19, #0x2348]
    svc #0x0a
.org 0x1000
    mov x0, #0
    msr fpcr, x0
    msr fpsr, x0
    msr nzcv, x0
    msr tpidr_el0, x0
    msr daif, x0
.irp n,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31
    movi v\n\().16b, #255
.endr
    mov x0, #0x40000000
    mov w1, #1
    str w1, [x0, #0x1000]
    mrs x1, tpidrro_el0
    str x1, [x0, #0x1010]
    svc #0x0a

.org 0x1200
benchmark_loop:
    subs x9, x9, #1
    b.ne benchmark_loop
    svc #0x07
