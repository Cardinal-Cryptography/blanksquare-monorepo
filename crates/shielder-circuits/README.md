# `shielder-circuits`

Contains implementation of the halo2 circuits for Shielder.

**Audit report** by zkSecurity is available [here](https://reports.zksecurity.xyz/reports/aleph-zero-shielder/).

# Benchmarks

In order to run the CPU benchmarks, execute the following command:

```bash
cargo bench
```

Benchmarks are configured in the `benches` directory. We use the `criterion` crate to run benchmarks.

You can find the results in the `target/criterion` directory.

# Code conventions

## Cells

Values that we work with during circuit synthesis (filling the PLONK table) appear in two forms:
1. `Value<F>` - a raw field element, which **is not** embedded into the circuit (yet).
   Usually, all the circuit inputs are given in this form (before they are explicitly embedded into the table).
   Similarly, most intermediate values that are computed off-circuit will firstly be created as `Value<F>`.
2. `AssignedCell<F>` - a value that is already embedded into the circuit and will be a part of the witness, that the prover will commit to.
   Such a cell has very precise coordinates: region identifier, column identifier, and a relative row offset (within the region).

We introduced `LazyAssignedCell<F>` which is a simple enum that can be either `Value<F>` or `AssignedCell<F>`.
In many cases, we want to use a value to compute something (like Poseidon hash), but we don't want to explicitly enforce it to be already embedded to a circuit.
Therefore, we defer embedding as long as possible and only do it when it is necessary.
Once a raw value is embedded into the circuit, the variant will be changed to `AssignedCell<F>`, so that consecutive uses (e.g., advice copies) are constrained (by copy-constraints) and no unnecessary work is done.

Please check: [documentation](src/lazy_assigned_cell.rs).

## Gates

It is critical for the security to carefully isolate certain responsibilities during circuit definition and building.
Since gates generate actual verification constraints, we really need to encapsulate separate logic of defining polynomial from e.g., input computation.
That is why we have a `Gate` concept.

A gate is responsible only for defining the polynomial by choosing appropriate columns and offsets and enforce consistent usage during synthesis.
The flow is as follows:
1. Configuration phase
    - define how many advice columns are needed (`Gate::Advice`)
    - create dedicated selector column (they are simple selectors and in the end, all of them should be compressed to a single column)
    - query required cells and build a polynomial expression based on them
2. Synthesis phase
    - get input values as `LazyAssignedCell<F>` (`Gate::Values`); usually this is a struct that binds values or cells to semantic naming
    - create a dedicated region
    - introduce input values to this region (by constrained copying), placing them exactly as they are expected (consistently with the configuration phase)
    - turn on the selector
    - return a new object of the same type (`Gate::Values`) that contains newly assigned cells

Please check: [documentation](src/gates/mod.rs).

## Circuit

The circuit is a high-level abstraction that is responsible for defining the whole circuit.
Usually it will use one or more chips.
Circuit **should not** contain any logic in its `synthesize` method.
It should serve only as an entrypoint to the underlying chip semantics.

There is only one place for some logic in the circuit: the `configure` method.
This is where the global `InstanceWrapper` object should be created.
Similarly, configurations for all underlying chips should be done here: see `ConfigsBuilder`.

Please check: [documentation](src/instance_wrapper.rs).
Please check: [documentation](src/config_builder.rs).

## Chips

Chips usually have their public inputs that will be narrowed versions of the global `InstanceWrapper` object (see `InstanceWrapper::narrow`).
Also, they will define their configuration struct that normally contains:
- some advice columns in case manual advice assignment must be done (see `AdvicePool`)
- underlying chips' configurations
- gates used in that chip
- public instance
