OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

KERNEL_VIRT_BASE = 0xFFFFFFFF80000000;

PHDRS
{
    text    PT_LOAD FLAGS(5); /* R-X */
    rodata  PT_LOAD FLAGS(4); /* R-- */
    data    PT_LOAD FLAGS(6); /* RW- */
}

SECTIONS
{
    . = KERNEL_VIRT_BASE;

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VIRT_BASE)
    {
        __text_start = .;
        *(.text .text.*)
        __text_end = .;
    } :text

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VIRT_BASE)
    {
        __rodata_start = .;
        *(.rodata .rodata.*)
        __rodata_end = .;
    } :rodata

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VIRT_BASE)
    {
        __data_start = .;
        *(.data .data.*)

        /* Limine requests section: start marker, requests, end marker.
           Ordering enforced by subsection names. */
        *(.requests_start_marker)
        *(.requests)
        *(.requests_end_marker)

        __data_end = .;
    } :data

    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_VIRT_BASE)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    } :data

    __kernel_end = .;

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.*)
    }
}
