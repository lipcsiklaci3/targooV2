// Data validation logic for ESG metrics
use std::fmt;

#[derive(Debug, Clone)]
pub enum ValidationError {
    UnknownUnit(String),
    UnitFamilyMismatch {
        input_unit: String,
        factor_unit: String,
        message: String,
    },
    RangeViolation {
        esrs_target: String,
        value: f64,
        min: f64,
        max: f64,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::UnknownUnit(u) => write!(f, "Unknown unit: {}", u),
            ValidationError::UnitFamilyMismatch { message, .. } => write!(f, "Unit family mismatch: {}", message),
            ValidationError::RangeViolation { esrs_target, value, min, max } => {
                write!(f, "Range violation for {}: {} is outside ({}-{})", esrs_target, value, min, max)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CanonicalUnit {
    KiloWattHour,
    MegaWattHour,
    Liter,
    CubicMeter,
    Kilogram,
    Tonne,
    Kilometer,
    TonneKilometer,
    Piece,
}

impl fmt::Display for CanonicalUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CanonicalUnit::KiloWattHour => "kWh",
            CanonicalUnit::MegaWattHour => "MWh",
            CanonicalUnit::Liter => "liter",
            CanonicalUnit::CubicMeter => "m3",
            CanonicalUnit::Kilogram => "kg",
            CanonicalUnit::Tonne => "t",
            CanonicalUnit::Kilometer => "km",
            CanonicalUnit::TonneKilometer => "tkm",
            CanonicalUnit::Piece => "stk",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UnitFamily {
    Energy,
    Volume,
    Mass,
    Distance,
    Transport,
    Count,
}

impl CanonicalUnit {
    pub fn family(&self) -> UnitFamily {
        match self {
            CanonicalUnit::KiloWattHour | CanonicalUnit::MegaWattHour => UnitFamily::Energy,
            CanonicalUnit::Liter | CanonicalUnit::CubicMeter => UnitFamily::Volume,
            CanonicalUnit::Kilogram | CanonicalUnit::Tonne => UnitFamily::Mass,
            CanonicalUnit::Kilometer => UnitFamily::Distance,
            CanonicalUnit::TonneKilometer => UnitFamily::Transport,
            CanonicalUnit::Piece => UnitFamily::Count,
        }
    }
}

pub struct UnitValidator;

impl UnitValidator {
    pub fn parse(raw: &str) -> Result<CanonicalUnit, ValidationError> {
        let normalized = raw.trim().to_lowercase();
        match normalized.as_str() {
            "kwh" | "kilowattstunde" => Ok(CanonicalUnit::KiloWattHour),
            "mwh" | "megawattstunde" => Ok(CanonicalUnit::MegaWattHour),
            "l" | "liter" | "litre" => Ok(CanonicalUnit::Liter),
            "m3" | "m³" | "kubikmeter" => Ok(CanonicalUnit::CubicMeter),
            "kg" | "kilogramm" | "kilogramme" => Ok(CanonicalUnit::Kilogram),
            "t" | "tonne" | "tonnen" | "ton" => Ok(CanonicalUnit::Tonne),
            "km" | "kilometer" => Ok(CanonicalUnit::Kilometer),
            "tkm" | "tonnenkilometer" => Ok(CanonicalUnit::TonneKilometer),
            "stk" | "stück" | "piece" | "unit" => Ok(CanonicalUnit::Piece),
            _ => Err(ValidationError::UnknownUnit(raw.to_string())),
        }
    }

    pub fn check_family_match(input: &CanonicalUnit, factor_unit: &CanonicalUnit) -> Result<(), ValidationError> {
        if input.family() != factor_unit.family() {
            return Err(ValidationError::UnitFamilyMismatch {
                input_unit: input.to_string(),
                factor_unit: factor_unit.to_string(),
                message: format!(
                    "Cannot calculate: Input unit '{}' ({:?}) is incompatible with Emission Factor unit '{}' ({:?})",
                    input,
                    input.family(),
                    factor_unit,
                    factor_unit.family()
                ),
            });
        }
        Ok(())
    }

    pub fn convert_to_base(value: f64, from: &CanonicalUnit) -> f64 {
        match from {
            CanonicalUnit::MegaWattHour => value * 1000.0,
            CanonicalUnit::Tonne => value * 1000.0,
            _ => value,
        }
    }
}

#[derive(Debug)]
pub enum RangeCheckResult {
    Ok,
    BestEffort(String),
    HardStop(String),
}

pub struct RangeGuard;

impl RangeGuard {
    pub fn get_range(esrs_target: &str) -> Option<(f64, f64)> {
        match esrs_target {
            "E1-6_Scope2_Electricity" => Some((0.00010, 0.00095)),
            "E1-1_Scope1_NaturalGas" => Some((0.00015, 0.00025)),
            "E1-1_Scope1_Diesel" => Some((0.00240, 0.00280)),
            "E1-1_Scope1_Petrol" => Some((0.00210, 0.00250)),
            "E1-1_Scope1_Heating_Oil" => Some((0.00240, 0.00270)),
            "E1-3_Scope3_Road_Freight" => Some((0.00008, 0.00015)),
            "E1-3_Scope3_Air_Freight" => Some((0.00040, 0.00090)),
            "E1-3_Scope3_Rail_Freight" => Some((0.00001, 0.00006)),
            "E1-2_Scope1_Refrigerants" => Some((0.00100, 4.00000)),
            "E1-3_Scope3_Waste_Landfill" => Some((0.00040, 0.00120)),
            _ => None,
        }
    }

    pub fn check(esrs_target: &str, calculated_tco2e_per_unit: f64) -> RangeCheckResult {
        let (min, max) = match Self::get_range(esrs_target) {
            Some(range) => range,
            None => return RangeCheckResult::Ok,
        };

        if calculated_tco2e_per_unit >= min && calculated_tco2e_per_unit <= max {
            return RangeCheckResult::Ok;
        }

        let best_effort_min = min * 0.1;
        let best_effort_max = max * 3.0;

        if calculated_tco2e_per_unit >= best_effort_min && calculated_tco2e_per_unit <= best_effort_max {
            RangeCheckResult::BestEffort(format!(
                "Value {} is outside normal range ({}-{}) for {}. Included with warning.",
                calculated_tco2e_per_unit, min, max, esrs_target
            ))
        } else {
            let n = if calculated_tco2e_per_unit > max {
                calculated_tco2e_per_unit / max
            } else {
                min / calculated_tco2e_per_unit
            };
            RangeCheckResult::HardStop(format!(
                "PHYSICS ERROR: Value {} is {:.1}x outside the expected range for {}. Quarantined.",
                calculated_tco2e_per_unit, n, esrs_target
            ))
        }
    }
}
