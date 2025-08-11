use core::ops::{Add, Mul, Sub};

use halo2_proofs::{arithmetic::CurveExt, halo2curves::grumpkin::G1, plonk::Expression};

use super::V;
use crate::{Field, Fr, Value};

/// An abstraction over the scalar field of the curve.
pub trait CurveScalarField:
    Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Clone
{
    /// Returns the parameter `b` from the curve equation.
    fn b() -> Self;
    /// Returns the parameter `b` from the curve equation added to itself 3 times.
    fn b3() -> Self;
    /// Returns the zero element of the scalar field.
    fn zero() -> Self;
    /// Returns the one element of the scalar field.
    fn one() -> Self;
}

impl CurveScalarField for Fr {
    fn b() -> Self {
        G1::b()
    }
    fn b3() -> Self {
        Self::b() + Self::b() + Self::b()
    }

    fn zero() -> Self {
        Fr::ZERO
    }

    fn one() -> Self {
        Fr::ONE
    }
}

impl CurveScalarField for Value {
    fn b() -> Self {
        Value::known(G1::b())
    }
    fn b3() -> Self {
        Value::known(Fr::b3())
    }

    fn zero() -> Self {
        Value::known(Fr::zero())
    }

    fn one() -> Self {
        Value::known(Fr::one())
    }
}

impl CurveScalarField for V {
    fn b() -> Self {
        V(Value::b())
    }

    fn b3() -> Self {
        V(Value::b3())
    }

    fn zero() -> Self {
        V(Value::zero())
    }

    fn one() -> Self {
        V(Value::one())
    }
}

impl CurveScalarField for Expression<Fr> {
    fn b() -> Self {
        Expression::Constant(Fr::b())
    }
    fn b3() -> Self {
        Expression::Constant(Fr::b3())
    }

    fn zero() -> Self {
        Expression::Constant(Fr::zero())
    }

    fn one() -> Self {
        Expression::Constant(Fr::one())
    }
}
