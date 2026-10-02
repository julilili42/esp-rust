SoC (System-on-a-Chip)
Espressif portfolio two different architectures: Xtensa and RISC-V.
Rust does not fully support Xtensa since it uses LLVM and LLVM does not support Xtensa. 

ESP-C6 Board:
- 2 USB-C Ports:
  - USB-C to UART: Used for power supply to the board, flashing and communication with chip via onboard USB-to-UART bridge. 
  - USB-C port

`esp-generate` to create new project. `esp-hal` ties all crates together. 

ESP-Chips have two bootloaders. 
ROM Bootloader is a small program, executed when the chip is powered on. Initializes Hardware, checks mode of chip waits until software is received and starts second Bootloader.
Second Bootloader loads application and sets up memory.

Heap allocation returns a memory address which is saved on the stack, and points to the beginning of the allocated memory. If more memory is required the memory manager searches for a new memory address.

Fragmentation can occure if f.ex. 20 KB memory is requested while non-connected 10 KB + 10 KB are available. 

Since we only have limited RAM -> Do not use heap
Think about how large certain values are during development.


Test as much as possible on the host machine. 
Hardware-in-Loop Testing (HIL) use of real devices in the testing setup. 
