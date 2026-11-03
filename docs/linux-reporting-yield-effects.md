# Linux plot-yield effect boundary

This is a static, target-specific refinement of the [active reporting provider](linux-reporting-provider.md), not a pure-getter certificate or an implementation of live gameplay. The [audit](../tools/linux/reporting_yield_effects_audit.py) and [result](../evidence/linux/reporting-yield-effects.json) pin 56 relevant method bodies, the actual plot virtual bindings, the standard native property-map lookup bindings, and the critical conditional instructions against the supplied Linux XP2 and matching host hashes. Proprietary instructions and DWARF remain private local evidence.

The accepted reporting replay has one active human, 6,996 plot-cache entries and 182 trade-cache entries. Thus the original YieldMonitor really reaches plot yield checking. It does not establish which per-plot improvement, property or appeal branches were taken. No new native execution was performed for this audit.

## Actual call and excluded temporary-object path

`Rules::Yields::GetYield` at `0x2bf5ad0` (3,498 bytes) returns an `int32` in EAX. Its arguments are receiver RDI, const Plot pointer RSI, YieldTypes EDX, fallback PlayerTypes ECX and bool R8b. There is no hidden return buffer. Original `YieldMonitor::CheckPlot` iterates yield indices from zero to the actual `NUM_YIELD_TYPES` and calls `Plot::GetYield` with false; the original wrapper forwards to Rules. Native integer ordering, wrapping and FixedPoint conversions are part of the contract.

The direct monitor chain **does not reach the temporary PlotYields add/remove branch**. Rules zeros R8 at `0x2bf6118` before calling Improvement.Yields.GetYieldFrom. That wrapper preserves this null contribution-list argument and forwards it in R9. GetPlayerYieldFrom saves it at stack `+0x18`, tests it at `0x2a3f08d`, and its null branch skips the call to GetPlayerBonusYieldFromModifiers. This exclusion follows the actual argument and branch bytes; it does not rely on a namespace, a null global GameEffects pointer, an empty map or an unchanged Save. The audit pins the full argument chain and skip branch.

## Read and temporary-output groups

The inspected scalar/definition accessors read existing fields and arrays. Feature.Manager.Edit follows Globals.EditMap and an existing component-container lookup; that inspected chain does not lazily create a component. The distinct-terrain statistic uses a local bool vector, and building enumeration clears/appends a caller-owned temporary vector. Their local allocations and writes must not be misclassified as writes to the queried Stats or City receiver.

The actual Plot vtable resolves appeal, city attributes and both property-map getters to the pinned native implementations. Plot.GetDynamicProperties returns the existing pointer at plot `+0x20`, or the original static empty map. The keyed accessor looks up an existing variant; it does not allocate a missing plot map. Improvement.Instance is an eight-byte owner-Plot pointer, and GetProperties uses that owner's keyed accessor with the original improvement key. The standard host TypedVariantMap virtual `+0x60` FindVariant and `+0x70` GetVariant are const tree lookups/default-return paths, not insertion paths. The actual nested property-map type used by a relevant property remains unobserved, so this standard binding cannot be substituted for every possible live nested map.

Owned-plot appeal reads the existing signed 16-bit value at plot `+0x4a`. Unowned appeal invokes the original Rules.Appeal calculation and a concrete local callback whose additions target its captured local accumulator. Remaining governor/rules dependencies are not a complete transitive effects proof. The active fixture's unowned, appeal-sensitive branch applicability is unobserved.

## General contribution-list case

Other callers supplying a nonnull contribution list can reach a materially different path. GetPlayerBonusYieldFromModifiers creates a real temporary GameEffects.PlotYields object, reads the calculated yield and removes it. Add synchronously dispatches ObjectAdded; Remove invokes ModifierSystem.ObjectDestroyed, clears the object's modifier IDs through ClearModifier, and dispatches its OnDestroyed signal. Actual subscriber effects and complete cleanup semantics have not been closed.

The system is 88 bytes: owner reference `+0`, uint32 counter `+8`, ObjectAdded signal `+0x10`, ID vector `+0x28` (four-byte elements), and shared-object vector `+0x40` (16-byte elements). Add increments the counter with native 32-bit wrap, stores the resulting ID in the object's `+0x18` field and appends the ID/object. The 344-byte concrete object embeds its PlotYieldData at `+296`; its destruction signal is at `+0xe0`. Remove erases the corresponding entries but does not roll the counter back. The inspected RemoveAll implementation also does not reset it. Constructor/Add are known counter writers; complete writer coverage is unproven.

A persistent live counter is **not proof of authoritative gameplay state**. Its serialization or synchronization status, surviving ID consumers after Remove, ordering effects on future rules, and complete modifier cleanup remain unknown. Conversely, allocation/query terminology does not establish a planning-only or UI-only exclusion. This general path is explicitly outside the monitor's proven null-contribution chain.

## Remaining conditional implementation boundary

The relevant unresolved domains are bounded by actual inputs:

- Improvement property contributions require a valid improvement and the relevant property. Outer native lookup behavior is identified; nested live map bindings/provenance and exact property values are still required.
- Improvement adjacency and bonus contributions require matching original definitions and rule references. Remaining getter dependencies and exact input values must be modeled; optional null planning/contribution arguments do not justify excluding gameplay data under AI names.
- Unowned appeal recomputation requires an appeal-sensitive yield and owner `-1`. Actual branch applicability and the remaining rule getters are not observed in the existing fixture.

A minimal passive refinement can inspect existing plot/property pointers and vtables at the existing monitor stop, without another breakpoint or inferior call. The actual Game map pointer is at `+0x250`; its plot vector is at Map `+0x1b0`, with 80-byte Plot elements. Plot `+0x20` identifies the existing property map. This only attests captured outer bindings; arbitrary nested variants require type provenance. Separately, actual Game `+0x268` then GameEffects `+0xa8` identifies the PlotYields system for a raw before/after counter observation. A raw delta is not an Add count without complete writer coverage. The provider observer owns any such replay; this audit performs none.

GetYield has no identified direct RNG or reporting call, but complete transitive RNG/callback absence is not proven. Outside GetYield, the original monitor compares and updates its receiver-owned IndexLists and emits changed-plot/aggregate reports in native order. A future typed implementation must preserve those effects and the exact rule/provider bindings, or reject the unsupported environment before mutation. This checkpoint admits no Rust live action, full-host action, full-state/RNG equivalence or turn support. Unknown save bytes, including the separate 89,198-byte Save-to-Load failure, remain unclassified and unmasked.

Reproduce the read-only target-byte check:

```sh
python3 tools/linux/reporting_yield_effects_audit.py --output target/linux/reporting-yield-effects.json
```
