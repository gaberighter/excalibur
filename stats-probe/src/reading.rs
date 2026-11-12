use serde_json::{Value, json};

/// One metric's result, in the shape every plugin reports (D27).
#[derive(Debug, Clone, PartialEq)]
pub enum Reading {
    Value(f64),
    Unavailable(String),
}

impl Reading {
    pub fn value(v: f64) -> Self {
        if v.is_finite() {
            Reading::Value(v)
        } else {
            Reading::unavailable("reading was not a finite number")
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Reading::Unavailable(reason.into())
    }

    pub fn to_json(&self) -> Value {
        match self {
            Reading::Value(v) => json!({ "value": v }),
            Reading::Unavailable(reason) => json!({ "value": null, "reason": reason }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_serialises_as_number() {
        assert_eq!(
            Reading::value(42.5).to_json().to_string(),
            r#"{"value":42.5}"#
        );
    }

    #[test]
    fn unavailable_serialises_as_null_with_reason() {
        assert_eq!(
            Reading::unavailable("sensor not found").to_json(),
            json!({ "value": null, "reason": "sensor not found" })
        );
    }

    #[test]
    fn non_finite_value_is_unavailable() {
        assert!(matches!(Reading::value(f64::NAN), Reading::Unavailable(_)));
        assert!(matches!(
            Reading::value(f64::INFINITY),
            Reading::Unavailable(_)
        ));
    }
}
