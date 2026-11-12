use serde_json::{Value, json};

/// One metric's result, in the shape every plugin reports (D27).
#[derive(Debug, Clone, PartialEq)]
pub enum Reading {
    Value {
        value: f64,
        /// What the value is out of, when that depends on the machine and so
        /// cannot be written in the plugin's manifest (D52).
        max: Option<f64>,
    },
    Unavailable(String),
}

impl Reading {
    pub fn value(value: f64) -> Self {
        Reading::with_max(value, None)
    }

    /// A value together with the total it is a part of.
    pub fn value_of(value: f64, max: f64) -> Self {
        Reading::with_max(value, Some(max).filter(|max| max.is_finite()))
    }

    fn with_max(value: f64, max: Option<f64>) -> Self {
        if value.is_finite() {
            Reading::Value { value, max }
        } else {
            Reading::unavailable("reading was not a finite number")
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Reading::Unavailable(reason.into())
    }

    pub fn to_json(&self) -> Value {
        match self {
            Reading::Value { value, max: None } => json!({ "value": value }),
            Reading::Value {
                value,
                max: Some(max),
            } => json!({ "value": value, "max": max }),
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
    fn value_of_a_total_serialises_with_max() {
        assert_eq!(
            Reading::value_of(5.5, 8.0).to_json(),
            json!({ "value": 5.5, "max": 8.0 })
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
