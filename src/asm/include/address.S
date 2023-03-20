%define VM_OFFSET 0xc0000000

%define physical_addr(x) (x - VM_OFFSET)
%define virtual_addr(x)  (x + VM_OFFSET)
