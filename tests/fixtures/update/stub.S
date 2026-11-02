# Fixture Linux x86_64: somente escreve uma versão simulada e encerra.
.global _start
.section .text
_start:
    mov $1, %rax
    mov $1, %rdi
    lea version(%rip), %rsi
    mov $9, %rdx
    syscall
    mov $60, %rax
    xor %rdi, %rdi
    syscall
.section .rodata
version:
    .ascii "fp 0.2.0\n"
.section .note.GNU-stack,"",@progbits
