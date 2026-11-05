# Ordinary Builder Repair

`loaded_actions::session::Session::repair` executes an externally selected Repair
order on the same retained state and stock services used by other actions. The
supported case is a Builder repairing an existing ordinary resource-free Farm.
It checks current charges, movement, pillage/flood status, repair-unit rules,
political ownership or suzerainty, and the supported drought domain.

Repair clears the pillage flag, delivers the ordered current repair subscribers,
updates the reached improvement/yield reporting and appeal, finishes movement,
and completes the queue and `HeadlessMessagesProcessed` aftermath. Repair does
**not** consume a Builder charge or destroy a one-charge Builder. This operation
does not call construction's citizen reallocation or culture bomb path.

The caller supplies admitted rules, actual ordered callback registrations, the
same current stock backend and visibility owner, current drought inputs and
qualified report recipients. Modifier attachment, resource extraction, positive
research boosts and active drought membership outside the supported branch fail
explicitly. A failed action retains its already applied prefix; callers must
inspect the Session failure instead of treating that prefix as a completed order.

Linux validation uses the target in [targets.md](targets.md). A representative
original-engine fixture initializes an owned Farm and one-charge Builder through
original setup methods before the input boundary, then executes Repair through
the original operation queue. The independent Rust Session replay matches the
affected typed components and player fields, completed unit lifecycle, canonical
territory/PlotYields identities, modifier allocator, appeal, and observed RNG
state. Native AFTER is comparison-only. This is a bounded ordinary-Farm result;
complete gameplay state, all actions and turn execution remain unfinished. The
complete native RNG inventory is still an explicit validation limit.
