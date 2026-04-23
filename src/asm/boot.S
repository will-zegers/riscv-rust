.option norvc
.section .data

.section .text.init

.global _start
_start:

  # Use only hart with ID 0 to bootstrap, and have all others wait for an
  #inter-processor interrupt (IPI) via the wait-for-interrupt (wfi) instruction
  csrr t0, mhartid
  bnez t0, 4f

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
2: # Machine mode
  # Point the stack pointer to the stack segment
  la sp, _stack_end

  # Set the mstatus register to be in machine mode
  li t0, 0b11 << 11
  csrw mstatus, t0

  # Make sure the hart is non-preemtable while we're running kinit
  csrw mie, zero

  # Point the machine exception program counter (mepc) to point to the entry
  # point of the Rust lib module's kinit function, which is where we'll land
  # on the machine return (mret) instruction
  la t1, kinit
  csrw mepc, t1

  la ra, 3f
  mret
3: # Supervisor mode
  la sp, _stack_end

  # Set bits in the status register to enter use mode and enable interrupts
  li  t0, (0b00 << 11) | (1 << 7) | (1 << 5)
  csrw mstatus, t0

  # Set the machine trap vector register (mtvec) to point to m_trap_vector
  # defined in trap.S
  la  t2, m_trap_vector
  csrw mtvec, t2

  # Enable machine and supervisor software, time, and external interrupts
  li t2, 0xaaa
  csrw mie, t2

  # Use mret to land on the entry point of the Rust lib
  la ra, 5f
  mret

4:# Harts with non-zero ID will be parked here
  # Give each hart a it's own slice of the stack
  la sp, _stack_end
  li t0, 0x10000
  csrr a0, mhartid
  mul t0, t0, a0
  sub sp, sp, t0

  # Put hart in machine mode with interrupts enabled
  li t0, (0b11 << 11) | (1 << 7)
  csrw mstatus, t0

  # Enable machine software inpterrupts
  li t3, (1 << 3)
  csrw mie, t3

  la t1, kinit_hart
  csrw mepc, t1

  la t2, m_trap_vector
  csrw mtvec, t2

  la ra, 5f
  mret
5:
  wfi
  j 5b
