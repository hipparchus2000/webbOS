; Simple Multiboot2 Boot Stub for WebbOS
; This allows the kernel to be loaded directly by QEMU/GRUB without UEFI

section .multiboot2_header
align 8

; Multiboot2 header magic
MB2_MAGIC equ 0xe85250d6
MB2_ARCH equ 0  ; i386 (works for x86_64 too)

header_start:
    dd MB2_MAGIC
    dd MB2_ARCH
    dd header_end - header_start
    dd -(MB2_MAGIC + MB2_ARCH + (header_end - header_start))

    ; Information request tag
    align 8
    dw 1
    dw 0
    dd 24
    dd 4    ; Basic mem info
    dd 5    ; BIOS boot device
    dd 6    ; Memory map
    dd 9    ; ELF sections

    ; Address tag (tell bootloader where to load us)
    align 8
    dw 2
    dw 0
    dd 24
    dd header_start
    dd 0x100000    ; Load base address (1MB)
    dd 0           ; Load offset
    dd 0           ; Load length

    ; Entry point tag
    align 8
    dw 3
    dw 0
    dd 12
    dd start

    ; Flags tag
    align 8
    dw 4
    dw 0
    dd 8

    ; End tag
    align 8
    dw 0
    dw 0
    dd 8
header_end:

section .bss
align 4096
pml4:
    resb 4096
pdpt:
    resb 4096
pd:
    resb 4096
stack_bottom:
    resb 16384
stack_top:

section .text
bits 32

global start
start:
    ; Multiboot2 info pointer is in EBX
    ; Magic value is in EAX (should be 0x36d76289)
    
    ; Set up stack
    mov esp, stack_top
    
    ; Save multiboot info
    push ebx
    push eax
    
    ; Check multiboot2 magic
    cmp eax, 0x36d76289
    jne .no_multiboot
    
    ; Set up basic identity paging for first 1GB
    ; This gets us to 64-bit mode where the kernel can set up proper paging
    
    ; Clear PML4
    mov edi, pml4
    xor eax, eax
    mov ecx, 1024
    rep stosd
    
    ; Set up PML4[0] -> PDPT
    mov edi, pml4
    mov eax, pdpt
    or eax, 0x003  ; Present + writable
    mov [edi], eax
    
    ; Set up PDPT[0] -> PD
    mov edi, pdpt
    mov eax, pd
    or eax, 0x003
    mov [edi], eax
    
    ; Set up PD entries for 1GB identity mapping
    ; Each entry maps 2MB, we need 512 entries for 1GB
    mov edi, pd
    mov eax, 0x00000083  ; Present + writable + huge page (2MB)
    mov ecx, 512
.set_pd:
    mov [edi], eax
    add edi, 8
    add eax, 0x200000    ; Next 2MB page
    dec ecx
    jnz .set_pd
    
    ; Load PML4
    mov eax, pml4
    mov cr3, eax
    
    ; Enable PAE
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax
    
    ; Enable long mode (EFER.LME)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr
    
    ; Enable paging
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax
    
    ; Now in compatibility mode, need to jump to 64-bit code
    lgdt [gdt64.pointer]
    jmp gdt64.code:start64

.no_multiboot:
    ; Print 'M' to indicate no multiboot
    mov al, 'M'
    mov [0xb8000], al
    mov byte [0xb8001], 0x0C
    hlt

section .data
align 8
gdt64:
    dq 0  ; Null descriptor
.code equ $ - gdt64
    dq (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)  ; Code: exec, code, present, 64-bit
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

section .text
bits 64

start64:
    ; We're in 64-bit mode!
    ; Set up data segments
    mov ax, 0
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    
    ; Restore multiboot info
    pop rax  ; magic
    pop rbx  ; multiboot info pointer
    
    ; The kernel expects to be loaded at 0xFFFF_8000_0000_0000
    ; We loaded it at 0x100000 (1MB)
    ; We need to either:
    ; 1. Copy it to the higher half (need more page tables)
    ; 2. Or jump to it at 1MB and let it set up its own higher-half mapping
    
    ; For simplicity, we'll jump to the kernel at 1MB
    ; The kernel will need to handle its own relocation/mapping
    
    ; Clear screen (VGA text mode)
    mov rdi, 0xb8000
    mov rax, 0x0f200f200f200f20  ; Space with white on black
    mov ecx, 1000
    rep stosq
    
    ; Print 'WebbOS' at top left
    mov rdi, 0xb8000
    mov rsi, msg
    call print_string
    
    ; Jump to kernel at 1MB + 1MB (2MB, after this stub)
    ; The kernel ELF should be loaded there by the bootloader
    mov rax, 0x200000  ; 2MB
    jmp rax

; Print string at RSI to VGA buffer at RDI
print_string:
    push rax
    push rcx
.loop:
    lodsb
    test al, al
    jz .done
    mov [rdi], al
    mov byte [rdi + 1], 0x0F
    add rdi, 2
    jmp .loop
.done:
    pop rcx
    pop rax
    ret

section .rodata
msg: db "WebbOS Boot Stub", 0
