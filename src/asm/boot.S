.option norvc
.section .data

.section .text.init

.global _start
_start:

  # Use only hart with ID 0 to bootstrap, and have all others wait for an
  #inter-processor interrupt (IPI) via the wait-for-interrupt (wfi) instruction
  csrr t0, mhartid
  bnez t0, 3f

  # Set the supervisor address translation and protection register to zero
  csrw satp, zero

 # Disable linker instruction relaxation and force the global pointer (gp)
 # register to always be calculated from the pc
.option push
.option norelax
  la  gp, _global_pointer
.option pop

  # Zero out the block started by symbol (BSS) segment
  la a0, _bss_start
  la a1, _bss_end
  bgeu a0, a1, 2f
1:
  sd zero, (a0)
  addi a0, a0, 8
  bltu a0, a1, 1b
2:
  # Point the stack pointer to the stack segment
  la sp, _stack_end

  # Set bits in the status register to enter machine protection to 3 mode and
  # enable interrupts
  li  t0, (0b11 << 11) | (1 << 7) | (1 << 3)
  csrw mstatus, t0

  # Point the machine exception program counter (mepc) to point to the entry
  # point of the Rust lib module's kmain function, which is where we'll land
  # on the mret instruction
  la  t1, kmain
  csrw mepc, t1

  # Set the machine trap vector register (mtvec) to point to asm_trap_vector
  # defined in trap.S
  la  t2, asm_trap_vector
  csrw mtvec, t2

  # Enable software, timer, and external interrupts in the machine interrupt
  # enable register
  li  t3, (1 << 3) | (1 << 7) | (1 << 11)
  csrw mie, t3

  # Set the return address to wait indefinitely for interrupts
  la  ra, 3f

  # Use mret to land on the entry point of the Rust lib
  mret
3:
  wfi
  j 3b
