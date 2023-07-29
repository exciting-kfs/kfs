#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum SigCode {
	SI_USER,   // kill
	SI_KERNEL, // from kernel
	SI_TIMER,
	SI_SIGIO,
	SI_TKILL,
	ILL_ILLOPC,  // Opcode
	ILL_ILLOPN,  // Operand
	ILL_ILLTRP,  // Trap
	ILL_PRVOPC,  // Privileged opcode
	ILL_PRVREG,  // Privileged register
	ILL_BADSTK,  // Stack
	FPE_INTDIV,  // Integer div by 0
	FPE_INTOVF,  // Integer overflow
	FPE_FLTDIV,  // Floating-point div by 0
	FPE_FLTOVF,  // Floating-point overflow
	FPE_FLTUND,  // Floating-point underflow
	FPE_FLTRES,  // Floating-point inexact result
	FPE_FLTINV,  // Floating-point invalid operation.
	SEGV_MAPERR, // Address not mapped to object.
	SEGV_ACCERR, // Invalid permissions for mapped object.
	SEGV_BNDERR, // Failed address bound checks.
	BUS_ADRALN,  // Invalid address alignment.
	CLD_EXITED,
	CLD_KILLED,
	CLD_DUMPED, // Child terminated abnormally.
	CLD_TRAPPED,
	CLD_STOPPED,
	CLD_CONTINUED,
	// SI_QUEUE,
	// SI_ASYNCIO,
	// ILL_ILLADR,  // Addressing mode
	// ILL_COPROC,  // Coprocessor error
	// FPE_FLTSUB,  // Subscript out of range.
	// BUS_ADRERR,  // Nonexistent physical address. ?
	// BUS_OBJERR,  // Object-specific hardware error. ?
	// BUS_MCEERR_AR,
	// BUS_MCEERR_AO,
	// TRAP_BRKPT,
	// TRAP_TRACE,
	// TRAP_BRANCH,
	// TRAP_HWBKPT,
	// POLL_IN,
	// POLL_OUT,
	// POLL_MSG,
	// POLL_ERR,
	// POLL_PRI,
	// POLL_HUP,
	// SYS_SECCOMP,
}
