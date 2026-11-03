# Independent snapshot assembly delta review

The assembler integration `e984c42` passes source review. The collector remains byte-identical to the [accepted `771423a` review](linux-script-state-snapshot-review.md). [Evidence](../evidence/linux/script-snapshot-assembly-review.json) records the exact changed sources and focused checks. No native profile or capture is admitted here.

`--script-snapshot` requires the current collector hash in the actual built source inventory. It adds the bounded unresolved-mode spec and all eight exact argument-layout body gates for the actual host, using the same values in the plan and stack packet. It preserves the original fragment, avoids duplicate matching bodies and refuses conflicting sizes/hashes. Without the flag, the original plan fields remain unchanged. The adapter now also compares both snapshot specs.

Three producer assembly tests pass. Independent checks confirm input preservation, idempotence, all eight actual-host gates and conflicting size/hash refusal. The actual adapter validator accepts a coherent opt-in synthetic envelope, then refuses a coherently rehashed packet with a different snapshot budget. That envelope executes no original code and supplies no native evidence.

Fresh source/archive/preload/profile, actual launch-path identity and new snapshot bytes remain separate reviews. Existing ownership, hardware, caller and bounded-copy requirements remain active. Extra HKS/GC/startup attribution may remain unresolved; this change establishes no callback effects or gameplay parity.
