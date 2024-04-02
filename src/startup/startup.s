.code16

/* Entrypoint for stage 2 */

.section .entrypoint

.extern ruststart
.global start
.type   start, @function
start:
    /* Print boot message */
    movw $boot_message, %si
    call msg_print


    /* Switching to protected mode, following recommendations laid out in
     * Intel's Software Developer Manual, Vol. 3A, section 9.9.1 - Switching to
     * Protected Mode. */

    /* 1. Disable interrupts, and NMI */
    cli
    /* Disable NMI */
    inb  $0x70, %al
    orb  $0x80, %al
    outb %al,  $0x80
    inb  $0x71, %al

    /* 2. Setup GDT */
    lgdt (gdtr)

    /* 3. Enable PE in CR0 */
    movl %cr0, %eax
    orl  $1,   %eax
    movl %eax, %cr0


    /* 4. Far jump */
    ljmpl $0x08, $1f

.code32
1:
    /* 5-7. N/A */
    /* 8. Set up task state segment */

    /* 9. Reload segment registers DS, SS, ES, FS, GS */
    movw $0x10, %ax
    movw %ax,   %ds
    movw %ax,   %ss
    movw %ax,   %es
    movw %ax,   %fs
    movw %ax,   %gs

    /* 10. Setup IDT */
    lidt (idtr)

    /* 11. Re-enable interrupts and NMI */
    /* This happens after the IDT is set up */
    /* Enable NMI */
    /* @todo? */
    /*inb  $0x70, %al
    andb $0x7F, %al
    outb %al,  $0x80
    inb  $0x71, %al*/

    /* Enable A20 line @todo Attempt multiple methods, perhaps in Rust */
    inb   $0x92, %al
    testb $0x02, %al
    jnz   1f
    orb   $0x02, %al
    outb  %al, $0x92
1:

    /* Call into C code */
    call ruststart

    jmp .
.size start, (. - start)


/* Print message.
 *
 * Parameters:
 *   %si: Pointer to string to print.
 */
.type msg_print, @function
msg_print:
    pusha
    movb $0x0E, %ah /* Write character */
    xorw %bx,   %bx /* Page, no color for text mode */
.loop:
    lodsb           /* Load character and increment %si */
    cmpb $0, %al    /* Exit if character is null */
    je   .end
    int  $0x10      /* Print character */
    jmp  .loop
.end:
    popa
    ret
.size msg_print, (. - msg_print)


boot_message:
    .asciz "\r\nStage2.\r\n"

gdtr:
    .word ((gdt_end - gdt) - 1)  /* Limit */
    .long gdt                    /* Base */

idtr:
    .word ((idt_end - idt) - 1) /* Limit */
    .long idt                   /* Base */

.align 8
/* @note Flat memory model with no protection */
gdt:
    /* 0x00: Null descriptor */
    .quad 0x00000000
    /* 0x08: 32-bit Code segment */
    .long 0x0000FFFF
    .long 0x00CF9A00
    /* 0x10: 32-bit Data segment */
    .long 0x0000FFFF
    .long 0x00CF9200
gdt_end:

idt:
    /* @todo */
    .quad 0
idt_end:

